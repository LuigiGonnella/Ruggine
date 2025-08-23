Session and TLS operations for Ruggine

Questo documento spiega in dettaglio i cambiamenti introdotti per la gestione delle sessioni, il comportamento `is_online`, la pulizia periodica delle sessioni scadute e l'abilitazione di TLS sul server.

1) Panoramica delle entità

- sessions (DB table): contiene colonne `user_id`, `session_token` (PK), `created_at`, `expires_at`.
- users: contiene `is_online` (INTEGER 0/1) che indica se l'utente è considerato online.

2) Flusso di login / register

- Al login o alla registrazione viene generato un `session_token` (UUID + random) e inserito in `sessions` con `created_at` e `expires_at`.
- Il server risponde al client con `OK: <username> SESSION: <token>`.
- Il client salva il token in keyring (con fallback su `data/session_token.txt`) e all'avvio lo invia a `/validate_session <token>`; se valido, il client entra nello stato `MainActions` senza mostrare la schermata di login.

3) Logout e `is_online`

- Comportamento precedente: `logout` cancellava la riga della sessione e subito eseguiva `UPDATE users SET is_online = 0 WHERE id = ?`. Questo causava che se l'utente aveva più sessioni (più dispositivi), la prima disconnessione marcava l'account come offline anche se altre sessioni restavano attive.

- Correzione introdotta: ora `logout` cancella la sessione specifica e poi esegue una query `SELECT COUNT(1) FROM sessions WHERE user_id = ?`. Solo se il conteggio è 0 esegue `UPDATE users SET is_online = 0 WHERE id = ?`.
  - Vantaggi: consente accessi multipli per lo stesso utente su dispositivi diversi senza falsi offline.
  - Nota: se desideri un modello "single device only", dovresti invece cancellare tutte le sessioni su logout o rifiutare nuove sessioni.

4) Cleanup periodico delle sessioni scadute

- È stato aggiunto `cleanup_expired_sessions(db: Arc<Database>)` in `src/server/auth.rs` che esegue `DELETE FROM sessions WHERE expires_at <= now`.
- In `src/server/main.rs` viene spawnato un task Tokio che chiama questa funzione ogni ora (configurabile modificando l'intervallo o aggiungendo una variabile d'ambiente).
- Consigli: in produzione impostare l'intervallo a un valore adeguato (es. 10-60 minuti) e monitorare i log per eventuali errori.

5) TLS: abilitazione e suggerimenti operativi

- Dipendenze: `rustls`, `tokio-rustls` e `rustls-pemfile` sono state aggiunte.
- Abilitazione: `ServerConfig.enable_encryption` viene letto dal `.env` (variabile `ENABLE_ENCRYPTION`). Se vero, il server cerca le variabili d'ambiente `TLS_CERT_PATH` e `TLS_KEY_PATH` che devono puntare a file PEM (certificato e chiave privata).
  - Il server supporta chiavi in formato PKCS8 e RSA (tentativo PKCS8 prima, poi RSA).
  - Se le variabili non sono impostate o i file non sono validi, il server logga un warning e continua in plain TCP.
- Protocollo: l'ALPN è impostato con `ruggine` come esempio; puoi rimuoverlo se non necessario.

6) Note di sicurezza e raccomandazioni

- Trasporto: anche se TLS è opzionale, raccomando vivamente di usarlo in produzione quando si scambiano token e credenziali.
- Storage: il client utilizza il keyring OS; il fallback a file è solo per ambienti in cui keyring non è disponibile. Evitare il fallback in produzione o criptare il file.
- Session rotation: per migliorare sicurezza, considera la rigenerazione del token periodicamente (es. dopo X giorni o su eventi sensibili) e la revoca dei token più vecchi oltre una soglia.
- Limiti di sessioni: se vuoi limitare il numero di dispositivi per utente, applica la logica al login: se COUNT(sessions) >= MAX, rimuovi la sessione più vecchia o rifiuta.

7) Come testare (end-to-end)

- Creare certificati (self-signed per test):
  - openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes -subj "/CN=localhost"
  - Esporta i file in una cartella e imposta TLS_CERT_PATH e TLS_KEY_PATH a quei percorsi.
- Avvia server in modalità TLS:
  - export ENABLE_ENCRYPTION=true
  - export TLS_CERT_PATH=./cert.pem
  - export TLS_KEY_PATH=./key.pem
  - cargo run --bin ruggine-server
- Avvia GUI e prova login/register; assicurati che il traffico sia cifrato dal punto di vista del sistema (strumenti come `openssl s_client` o tcpdump possono aiutare).

8) File to-check / punti di estensione

- `src/server/auth.rs` : cleanup_expired_sessions, logout aggiornato.
- `src/server/connection.rs` : listener TLS optional, handle_tls_client wrapper.
- `src/server/main.rs` : spawn task per cleanup e logging TLS hints.
- `src/client/utils/session_store.rs` : keyring + fallback file storage.

Se vuoi, posso:
- aggiungere un flag/variabile per controllare la frequenza del cleanup;
- implementare un job di rotation token (refresh token / short-lived access token + refresh token);
- evitare il fallback file nel client e fallire esplicitamente quando keyring non è disponibile (migliore per sicurezza).

Fine del documento.
