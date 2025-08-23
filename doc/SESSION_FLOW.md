# Session & Presence Flow (descrizione tecnica)

Questo documento spiega il flusso di sessione, presenza e gestione delle disconnessioni implementato nell'applicazione.
È pensato come riferimento rapido per sviluppatori e per il debug operativo.

## Componenti principali

- Client (GUI)
  - ChatService: service persistente che mantiene una connessione TCP/TLS al server e invia comandi sequenzialmente (mpsc + oneshot response).
  - UI (iced): genera messaggi `Message::Logout`, `SubmitLoginOrRegister`, ecc. L'azione di logout invia `/logout <token>` al server se disponibile.

- Server
  - `connection.rs`: handler per ogni connessione (TCP o TLS). Riceve linee, le logga, le inoltra al dispatcher `Server::handle_command` e gestisce la registrazione della presenza.
  - `presence::PresenceRegistry`: registro in-memoria (Arc + Mutex) che associa `user_id -> Vec<oneshot::Sender<()>>`. Usato per segnalare (kick) connessioni attive.
  - `auth.rs`: login, register, validate_session, logout. Login è effettuato in una transazione che garantisce l'invalidazione atomica delle sessioni precedenti e l'inserimento della nuova.
  - Database (sqlite via sqlx): tabelle `users` (colonna `is_online`), `sessions`, `session_events`.

## Eventi nel DB (`session_events`)
Tipi usati:
- `login_success` — inserito al termine del login atomico.
- `logout` — inserito quando l'utente esegue esplicitamente /logout.
- `quit` — inserito quando una connessione termina senza eseguire prima /logout (es. chiusura client, crash).
- `kicked_out` — inserito quando una sessione viene forzatamente disconnessa perché l'utente ha effettuato login su un altro device.

Nota: lo schema registra eventi distinti per distinguere `logout` (intenzionale) da `quit` (disconnessione non intenzionale).

## Sequenze chiave

1) Login (device B) — guarantee single-session
- Client invia `/login user pass`.
- Server (in transazione): elimina righe `sessions` esistenti per `user_id`, inserisce nuova session row (token), setta `users.is_online = 1`, inserisce `login_success` event, commit.
- Server risponde includendo `SESSION: <token>`.
- Connessione che rileva `SESSION:` valida il token con `validate_session`, chiama `presence.kick_all(user_id)` per forzare la disconnessione di vecchie connessioni, registra la nuova presenza (`presence.register`), setta `is_online=1` e tiene il receiver per essere notificata in caso di kick. Viene registrato `kicked_out` per le sessioni rimosse.

2) Auto-login (startup client)
- Client con token salvato invia `/validate_session <token>` usando ChatService persistente.
- Se OK: server registra la presenza (non effettua kick) e setta `is_online = 1`. Questo consente di riconoscere la connessione come attiva (preserve session row for auto-login semantics).

3) Logout (esplicito)
- Client invia `/logout <token>`.
- Server verifica il token, chiama `auth::logout` che elimina le sessioni del user e setta `users.is_online = 0`, inserisce evento `logout`.
- Server chiama `presence.kick_all(user_id)` per segnalare alle connessioni attive di chiudersi.
- Le connessioni che ricevono il oneshot di kick escono; il server pulisce la presenza.

4) Quit (chiusura client senza /logout)
- Quando la connessione TCP si chiude (read_line -> 0) il server esegue cleanup della presenza e se non ci sono più connessioni attive per l'utente setta `is_online = 0` e inserisce `quit` event. La session row può rimanere per supportare l'auto-login.

## Regole e invarianti
- Single-session invariant: alla fine del login (commit) esiste una sola session row valida per l'utente nello scenrio tipico (login rimuove le vecchie righe).
- is_online == 1 <=> almeno 1 connessione registrata in `PresenceRegistry` per l'utente.
- logout deve invalidare tutte le sessioni e portare `is_online = 0`.
- quit non invalida sessioni; permette auto-login.

## Debugging rapido
- Client non invia `/logout`: controllare log `[CLIENT:SVC] Sending logout command` nello stdout del client GUI.
- Server non riceve la linea: cercare `[CONN:RAW] [peer] Raw line received: '...'` nei log server.
- Dopo `/logout` dovresti vedere (server):
  - `validate_session` (se presente), `Handling /logout for user ...`, `Deleted N session rows`, `Set is_online=0`, `Inserted logout event`, `Kicking N connections`.
- Se vedi sia `logout` che `quit` per la stessa azione, è perché la connessione ha eseguito `/logout` e poi la routine di cleanup ha inserito anche `quit` — questo è intenzionale nella versione corrente per assicurare eventi coerenti in tutti i casi (si può cambiare se si preferisce evitare duplicati).

## Edge cases e suggerimenti
- Race: se due login paralleli avvengono contemporaneamente, la transazione nel login minimizza la chance di doppie sessioni; comunque è consigliato serializzare l'accesso a login per lo stesso user in livelli più alti (throttling) se necessario.
- Persistenza e auditing: `session_events` permette ricostruire la timeline; considera aggiungere un campo `source` (es. device id, ip) se vuoi attribuire meglio gli eventi.
- Test consigliati: scenari automatizzati per (login A -> login B -> A kicked), (login -> quit -> validate_session auto-login), (login -> logout -> no auto-login).

## Conclusione
Questa implementazione cerca di bilanciare l'usabilità (preserve session per auto-login) con la sicurezza/consistenza (logout invalida sessioni, login è atomico e forza kick). Se vuoi, posso aggiungere uno schema ER o una sequenza PlantUML nel documento, o convertire questo file in `doc/SESSION_AND_TLS.md` sovrascrivendo l'esistente.
