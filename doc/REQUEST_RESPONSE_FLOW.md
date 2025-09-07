## Flusso richiesta/risposta, sessioni e riconnessione

Questa pagina descrive il comportamento corrente nel repository: come il server gestisce sessioni e presenza, come il client mantiene una connessione persistente e come vengono gestiti i casi limite (kick, logout, socket chiuso). Include anche raccomandazioni pratiche per evolvere il protocollo (per esempio Request-ID).

### Obiettivo
- Documentare il comportamento attuale (server + client) così com'è nel codice.
- Spiegare i failure-mode comuni e perché la soluzione client-side attuale evita il problema di "login -> logout -> login" che fallisce.
- Fornire raccomandazioni tecniche e UX per miglioramenti futuri.

### Contratto minimo (input/output)
- Input: linee di testo inviate dal client (es. `/login user pass`, `/logout <token>`, `/validate_session <token>`). Ogni comando è una singola linea terminata da `\n`.
- Output: singola linea di risposta del server per ogni comando (es. `OK: ...`, `ERR: ...`). Alcune risposte contengono `SESSION: <token>` al login.
- Comportamento d'errore: se la connessione è chiusa dal server, il client tenta di riconnettere e reinviare la stessa richiesta.

### File chiave (riferimento)
- Server: `src/server/connection.rs`, `src/server/auth.rs`, `src/server/presence.rs`, `src/server/websocket.rs`
- Client: `src/client/services/chat_service.rs`, `src/client/services/websocket_service.rs`
- Test: `src/bin/chat_test.rs` (client TCP di test), `src/bin/db_inspect.rs` (tool di debug database)

## Server: sessioni e presenza

1. Login
- Endpoint: comando `/login user pass` in `Server::handle_command` (delegato a `auth::login`).
- Se il login crea una nuova sessione, la risposta include una linea che contiene `SESSION: <token>`.
- Quando la connessione che invia il comando riceve una risposta contenente `SESSION:`, il codice di connessione (in `handle_client` / `handle_tls_client`) estrae il token, risolve `user_id` con `auth::validate_session(...)` e chiama `presence.kick_all(&user_id)` per disconnettere le altre connessioni attive di quel user (ratifica della politica "single session per device" che abbiamo scelto: kick previous devices).
- Se `kick_all` ritorna > 0, viene inserito un evento `session_events` con `event_type = 'kicked_out'`.
- Infine la connessione corrente registra la propria presenza con `presence.register(&user_id)`, imposta `users.is_online = 1` e mantiene un receiver (`kick_rx`) per eventuali kick futuri.

2. validate_session (auto-login)
- Comando `/validate_session <token>` risponde `OK: <username>` se il token è valido.
- Dopo una `OK` il codice connessione registra la presenza (senza kickare altre sessioni) e imposta `is_online = 1`.

3. Logout
- Comando `/logout <token>` delegato a `auth::logout`.
- Il server cancella le sessioni corrispondenti al token e imposta `users.is_online = 0` dentro la transazione di logout.


4. Fine connessione
- Quando una connessione termina (read == 0 o `kick_rx` innescato), la connessione fa `presence.unregister_one(&user_id)`.
- Se `presence.count(&user_id)` ritorna 0, il server scrive `users.is_online = 0` e inserisce un evento `session_events` con `event_type = 'quit'`.

### PresenceRegistry (concetto)
- Tenuta in memoria tramite `PresenceRegistry` che mappa user_id -> lista di `oneshot::Sender<()>` per notificare il kick.
- `register` restituisce un `oneshot::Receiver<()>` che la connessione ascolta; `kick_all` invia `()` a tutti i sender e pulisce la lista; `unregister_one` rimuove una singola registrazione.

## Client: ChatService e connessione persistente

Implementazione attuale (file `src/client/services/chat_service.rs`):

1. Struttura
- `ChatService` mantiene un canale `mpsc::UnboundedSender<(String, oneshot::Sender<String>)>` verso un task in background.
- Il task di background possiede la socket `TcpStream` (split in reader/writer) e processa le richieste in entrata in modo sequenziale.

2. Per ogni comando inviato dall'app
- L'app crea un `oneshot::channel()` e invia `(cmd, resp_tx)` sul canale mpsc.
- Il background task prende la coppia e prova a scrivere `cmd + "\n"`, flush, e poi legge una singola linea di risposta.

3. Riconnessione e replay (comportamento chiave)
- Se in qualsiasi punto la scrittura fallisce, la lettura fallisce, o la `read_line` ritorna 0 (peer chiuso), il background task effettua una riconnessione TCP e riprova a spedire la stessa richiesta.
- Il retry è interno al task e continua finché non si ottiene una risposta valida o finché non si scopre un errore irreversibile (es. impossibile riconnettersi). Quando non è possibile riconnettere, il task invia una risposta del tipo `ERR: reconnect failed: ...` sul `resp_tx` e passa alla richiesta successiva.
- Questo approccio fa sì che, se il server chiude la connessione in risposta ad un `/logout` o durante la fase di kick, la richiesta successiva (es. il successivo `/login` fatto immediatamente dopo) venga reinviata automaticamente e non fallisca per la prima disconnessione.

4. Vantaggi e limiti
- Vantaggio: semplice, non richiede modifiche al protocollo; trasparente per la UI quando la riconnessione ha successo entro un tempo breve.
- Limite: non distingue le risposte alle richieste da messaggi asincroni del server; se il server invia notifiche non correlate, il client assume che la prima linea letta sia la risposta alla richiesta corrente. Questo è attualmente accettabile perché il protocollo server->client non manda messaggi spontanei nello stesso canale di richiesta/risposta per le operazioni sincrone usate dalla GUI.

## Failure modes e casi limite

1. Server chiude socket come conseguenza di kick
- Server invia semplicemente `OK: ... SESSION: ...` (o nulla) e chiude le connessioni precedenti tramite `kick_all`, che causa `read_line`==0 nelle connessioni vecchie.
- Client rileva `read_line` == 0, riconnette e reinvia la stessa richiesta. Questo risolve la race di `login->logout->login` quando il client non si riavvia tra le azioni.

2. oneshot cancellato dal chiamante
- Se il chiamante drop-a il lato ricevente (`resp_rx`) prima che la risposta arrivi, il `resp_tx.send(...)` fallirà. Il background task gestisce comunque l'errore ignorando l'invio. Questo è un comportamento accettabile (caller non vuole più la risposta).

3. Messaggi asincroni/server-push
- Se il server in futuro inizi a spedire messaggi push non correlati alle richieste in corso, l'attuale design potrebbe sbagliare e consegnare una notifica al `resp_tx` della richiesta corrente.

## Raccomandazioni

1. Protocollo Request-ID (raccomandato per robustezza)
- Aggiungere un request-id per ogni richiesta (es. `RID:<uuid> /login ...`) e includere lo stesso RID nella risposta. Il client mantiene una mappa RID -> oneshot Sender e può distinguere risposte autonome. Questo permette di supportare server-push e parallellismo delle richieste.
- Nota: l'implementazione end-to-end richiede: server che rispedisce RID nelle risposte; client che lega RID alle richieste ed alla mappa; gestione dei RID scaduti e policy di replay dopo riconnessione.

2. Backoff e jitter sulle riconnessioni
- Attualmente il task tenta riconnessioni immediate; consigliabile aggiungere backoff esponenziale con jitter limitato per evitare hot loops quando il server non è raggiungibile.

3. UX su kick e logout
- Quando il client riceve una notifica di kick (oppure quando il `presence.kick_all` provoca la chiusura della connessione), la GUI dovrebbe mostrare una notifica chiara come "Sei stato disconnesso perché hai effettuato il login da un altro dispositivo" e reindirizzare alla pagina di autenticazione (opzione scelta nel progetto: comportamento restrittivo). Questo mapping va implementato nella parte GUI: `src/client/gui/...` ascoltando gli errori di connessione o notifications dal server.

4. Tracciamento e log
- Mantenere logging lato server per ogni evento di sessione (`login_success`, `logout`, `kicked_out`, `quit`) e lato client per ogni riconnessione e retry. I file `data/ruggine_performance.log` e `data/ruggine_modulare.db` sono utili per debug.

## Come riprodurre il problema e testare la soluzione

1. Avvia il server:
- `cargo run --bin ruggine-server` (esegui su Windows PowerShell come nel progetto).

2. Usa il test automatico:
- `cargo run --bin chat_test` che esegue sequenze di `/login`, `/logout`, `/login` per verificare il comportamento di riconnessione e replay.

3. Ispeziona il DB per eventi di sessione:
- `cargo run --bin db_inspect` (stampa `users`, `sessions`, `session_events`).

## Mappatura rapida codice -> comportamento
- `src/server/connection.rs` — ascolta linee, chiama `Server::handle_command`, registra presenza su `/validate_session` e su risposta `SESSION:`; chiama `presence.kick_all` su login; setta `is_online`.
- `src/server/presence.rs` — implementa `register`, `kick_all`, `unregister_one`, `count`.
- `src/client/services/chat_service.rs` — background task mpsc+oneshot: invia, legge, su socket error riconnette e reinvia la richiesta.

## Conclusione e prossimi passi
- Lo stato attuale è operativo e risolve la race di `login->logout->login` tramite il replay dal background task client.
- Se si desidera scalare il protocollo (supportare push, richieste parallele, miglior logging client-server) il passo successivo consigliato è introdurre Request-ID + pending-map lato client e supporto RID nelle risposte lato server.

Se vuoi, applico subito una PR che:
- implementa RID end-to-end (server+client) con map pending requests e timeout; oppure
- aggiunge backoff/jitter al task di riconnessione del client; oppure
- aggiunge la notifica GUI per "kicked" e la pagina di redirect.
