# Session & TLS (flusso di sessione, presenza e TLS)

Questo documento unificato descrive il comportamento delle sessioni, la gestione della presenza/concorrenza tra dispositivi, gli eventi d'audit persistiti e le note operative su TLS.

## Panoramica delle entità

- `sessions` (DB): colonne principali `user_id`, `session_token` (PK), `created_at`, `expires_at`.
- `users`: contiene `is_online` (INTEGER 0/1) che indica se l'utente è considerato online.
- `session_events`: tabella di audit con eventi come `login_success`, `logout`, `quit`, `kicked_out`.

## Componenti principali

- Client (GUI)
  - `ChatService`: servizio persistente che mantiene connessione TCP/TLS al server e invia comandi sequenzialmente (mpsc + oneshot per la risposta).
  - UI (iced): genera messaggi (`Message::Logout`, `SubmitLoginOrRegister`, ecc.). L'azione di logout invia `/logout <token>` se presente.

- Server
  - `connection.rs`: handler per ogni connessione (TCP o TLS). Logga le linee raw, le inoltra a `Server::handle_command` e gestisce la registrazione della presenza.
  - `presence::PresenceRegistry`: registro in-memoria (Arc + Mutex) che associa `user_id -> Vec<oneshot::Sender<()>>` per segnalare (kick) connessioni attive.
  - `auth.rs`: contiene `login`, `register`, `validate_session`, `logout`, e la pulizia di sessioni scadute. `login` è eseguito in transazione per garantire atomicità.
  - Database (sqlite via sqlx): tabelle `users`, `sessions`, `session_events`.

## Eventi nel DB (`session_events`)
Tipi usati:
- `login_success` — inserito al termine del login atomico.
- `logout` — inserito quando l'utente esegue esplicitamente `/logout`.
- `quit` — inserito quando una connessione termina senza eseguire prima `/logout` (es. chiusura client, crash).
- `kicked_out` — inserito quando una sessione viene forzatamente disconnessa perché l'utente ha effettuato login su un altro device.

Lo schema differenzia `logout` (azione intenzionale dell'utente) da `quit` (disconnessione non intenzionale) per scopi di auditing e comportamenti di auto-login.

## Sequenze chiave

1) Login (device B) — single-session guarantee

- Client invia `/login user pass`.
- Server (in transazione): elimina righe `sessions` esistenti per `user_id`, inserisce nuova riga `sessions` (token), setta `users.is_online = 1`, inserisce `login_success` event, commit.
- Server risponde includendo `SESSION: <token>`.
- La connessione che riceve `SESSION:` (tipicamente la stessa che ha fatto login) valida il token con `validate_session`, chiama `presence.kick_all(user_id)` per forzare la disconnessione di vecchie connessioni, registra la nuova presenza (`presence.register`), setta `is_online=1` e mantiene il receiver per ricevere kick futuri. Per ogni sessione rimossa viene registrato un `kicked_out`.

2) Auto-login (startup client)

- Il client, se ha un token salvato, invia `/validate_session <token>` usando il `ChatService` persistente.
- Se la risposta è OK, il server registra la presenza (NON effettua kick) e setta `is_online = 1`. Questo consente il comportamento di auto-login senza invalidare la session row.

3) Logout (esplicito)

- Client invia `/logout <token>`.
- Server verifica il token e chiama `auth::logout` che elimina le sessioni del user e setta `users.is_online = 0`. Viene inserito un evento `logout`.
- Server chiama `presence.kick_all(user_id)` per notificare e forzare la chiusura di connessioni attive.
- Le connessioni che ricevono il oneshot di kick escono; il server pulisce la presenza.

4) Quit (chiusura client senza `/logout`)

- Quando la connessione TCP si chiude (`read_line -> 0`) il server esegue cleanup della presenza. Se non ci sono più connessioni attive per l'utente setta `is_online = 0` e inserisce `quit` event. La session row può rimanere per supportare l'auto-login.

## Regole e invarianti

- Single-session invariant: il `login` atomico tende a garantire che alla fine rimanga una singola session row valida per l'utente nello scenario tipico (il login rimuove le vecchie righe prima di inserire la nuova).
- `is_online == 1` <=> almeno 1 connessione registrata in `PresenceRegistry` per l'utente.
- `logout` invalida tutte le sessioni e dovrebbe portare `is_online = 0`.
- `quit` non invalida sessioni (permette auto-login sul device successivo).

## Debugging rapido

- Client non invia `/logout`: controllare stdout della GUI per la riga `[CLIENT:SVC] Sending logout command (redacted)` o `[CLIENT:SVC] Sending command: ...`.
- Server non riceve la linea: cercare `[CONN:RAW] [peer] Raw line received: '...'` nei log server.
- Dopo `/logout` dovresti vedere in ordine approssimativo (server):
  - `validate_session` (se pertinente), `Handling /logout for user ...`, `Deleted N session rows`, `Set is_online=0`, `Inserted logout event`, `Kicking N connections`.
- Se vedi sia `logout` che `quit` per la stessa azione, questo può avvenire se la connessione esegue `/logout` e la routine di cleanup della connessione registra anche `quit` — nella versione corrente è stato deciso di mantenere entrambi gli eventi per avere una traccia completa della transizione; è possibile modificare questo comportamento se preferisci evitare duplicati.

## TLS: abilitazione e suggerimenti operativi

- Dipendenze: `rustls`, `tokio-rustls` e `rustls-pemfile` sono usate per TLS.
- Abilitazione: impostare `ENABLE_ENCRYPTION=true` e fornire `TLS_CERT_PATH` e `TLS_KEY_PATH` che puntano ai file PEM (certificato e chiave privata). Il server prova PKCS8 prima di RSA per le chiavi.
- Se la configurazione TLS non è valida, il server cade automaticamente su plain TCP e logga un avviso.
- L'ALPN è impostato su `ruggine` come esempio; puoi rimuoverlo o cambiarlo.

## Note di sicurezza e raccomandazioni

- Trasporto: usare TLS in produzione è fortemente raccomandato per proteggere token e credenziali.
- Storage: il client preferisce il Keyring di sistema; il fallback a file è solo per ambienti di sviluppo. Evitare il fallback in produzione o criptare il file.
- Session rotation: considera la rigenerazione dei token su eventi sensibili o periodicamente e la revoca dei token più vecchi.
- Limiti di sessioni: per imporre un numero massimo di dispositivi, applica la logica al login: se `COUNT(sessions)` >= MAX, rimuovi la sessione più vecchia o rifiuta il login.

## Cleanup periodico delle sessioni scadute

- È disponibile una funzione `cleanup_expired_sessions(db: Arc<Database>)` (in `src/server/auth.rs`) che esegue `DELETE FROM sessions WHERE expires_at <= now`.
- Un task Tokio nel `main` può chiamare questa funzione periodicamente (es. ogni 10-60 minuti). Configura l'intervallo via variabile d'ambiente se desideri.

## Edge cases e suggerimenti operativi

- Condizioni di race: la transazione nel login aiuta, ma scenari molto concorrenti potrebbero richiedere meccanismi di locking a livello di applicazione per lo stesso `user_id`.
- Audit: `session_events` è utile per ricostruire le timeline; aggiungi un campo `source` (device id, ip) se vuoi attribuire eventi a device specifici.
- Test consigliati: (login A -> login B -> A kicked), (login -> quit -> validate_session auto-login), (login -> logout -> no auto-login).

## File rilevanti

- `src/server/auth.rs` : login/logout logic, cleanup_expired_sessions.
- `src/server/connection.rs` : handler per connessioni TCP/TLS, raw line logging, presenza.
- `src/server/presence.rs` : `PresenceRegistry` (register/kick/unregister/count).
- `src/server/main.rs` : spawn task per cleanup e inizializzazione server.
- `src/client/services/chat_service.rs` : `ChatService` persistente e log client-side.
- `src/client/utils/session_store.rs` : keyring + fallback file storage.

## Conclusione e possibili estensioni

Questa implementazione bilancia usabilità (preserve session per auto-login) con consistenza e sicurezza (logout invalida sessioni; login è atomico e forza kick). Possibili estensioni:

- Aggiungere campo `source` a `session_events` (device id, IP).
- Implementare refresh tokens / short-lived access tokens per migliorare sicurezza.
- Rendere opzionale il comportamento di duplicazione `logout` + `quit` (se preferisci evitare duplicate events, possiamo cambiare il cleanup della connessione per saltare `quit` quando la stessa connessione ha eseguito `logout`).

Se vuoi, posso generare anche uno schema ER o diagrammi PlantUML per queste sequenze.
