# Flusso e funzionamento della chat privata

Questo documento descrive il flusso end-to-end, la logica client/server e le soluzioni adottate per la chat privata nel progetto Ruggine.

## Scopo
Breve guida su come funziona la chat privata: apertura, polling in tempo reale, invio/ricezione messaggi, gestione dello stato di caricamento e crittografia persistente.

## File chiave
- `src/client/models/app_state.rs` — stato dell'app, gestione dei messaggi UI, loader per chat private, logica per invio e caricamento messaggi.
- `src/client/gui/views/private_chat.rs` — rendering della view della chat privata (loader, placeholder "Nessun messaggio ancora...", lista messaggi).
- `src/client/gui/app.rs` — inizializzazione, delega update all'app state, orchestrazione del polling periodico.
- `src/client/services/chat_service.rs` — chiamate di rete verso il server (get/send private messages).
- `src/common/crypto.rs` — helper di crittografia condivisi (parsing del master key, derivazione chat-key, encrypt/decrypt).
- `src/server/config.rs` — caricamento della configurazione server (incluso `ENCRYPTION_MASTER_KEY`).
- `src/server/messages.rs` — logica di memorizzazione e lettura messaggi sul server (encrypt/decrypt, fallback su errore di decrittazione).

## Flusso lato client
1. Apertura chat privata
   - L'azione `OpenPrivateChat(username)` imposta lo stato `AppState::PrivateChat(username)` e inserisce `username` in `loading_private_chats`.
   - Viene lanciato un comando asincrono `StartMessagePolling { with: username }` per iniziare il polling delle nuove permesse.

2. Visualizzazione
   - Se `loading_private_chats` contiene l'username, la view mostra un loader (es. "Caricamento messaggi...").
   - Se la chat non è in loading e non ci sono messaggi presenti nella cache `private_chats[username]`, la view mostra il placeholder "Nessun messaggio ancora...".
   - Se ci sono messaggi cache, la view mostra l'elenco dei messaggi.

3. Polling e aggiornamenti in tempo reale
   - L'app esegue polling periodico con piccole pause (es. 100 ms) quando `polling_active` è true.
   - Quando arrivano `NewMessagesReceived { with, messages }`, i messaggi vengono inseriti in `private_chats[with]` e l'entry `with` viene rimossa da `loading_private_chats`.
   - Dopo aver inserito i messaggi, il polling continua automaticamente (ri-lancio del fetch asincrono con sleep breve).

4. Invio messaggio
   - `SendPrivateMessage { to }` verifica che l'input non sia vuoto e se esiste `session_token` esegue l'invio asincrono tramite `ChatService`.
   - Se la cronologia della chat non è ancora in cache, il client imposta `loading_private_chats.insert(to.clone())` per mostrare il loader fino al refresh.
   - L'input viene resettato immediatamente (UX ottimistica) e viene lanciato il comando di invio; al termine si chiede un `TriggerImmediateRefresh { with: to }` per forzare il refresh immediato della cronologia.

## Logica lato server e persistenza
- Il server memorizza i messaggi in forma cifrata usando una chat-key derivata da un master key e dall'elenco partecipanti.
- Derivazione: chat-key = SHA256(master_key || sorted_participants) (deterministico e simmetrico tra client/server se entrambi usano gli stessi input).

## Crittografia e gestione della master key
Problema affrontato: se il server genera ad ogni avvio un nuovo `ENCRYPTION_MASTER_KEY`, i messaggi precedentemente memorizzati diventano illeggibili.

Soluzioni adottate:
- Persistenza della master key tramite `.env` (variabile `ENCRYPTION_MASTER_KEY`).
  - `src/server/config.rs` tenta di caricare `ENCRYPTION_MASTER_KEY` dall'ambiente/.env; se non presente, genera un nuovo valore e suggerisce di salvarlo.
- Helper centralizzati in `src/common/crypto.rs`:
  - parsing/validazione del valore hex a 32 byte;
  - funzione `load_master_key_from_env()` che cerca la variabile/il file `.env` e restituisce l'array di 32 byte;
  - funzioni di encrypt/decrypt che implementano AES-256-GCM con nonce random e serializzazione base64 per storage.

Nota: è fondamentale che client e server condividano la stessa modalità di derivazione della chat-key (stesso ordine dei partecipanti e stessa input canonicalization).

## Robustezza su errori di decrittazione
- Se la decrittazione fallisce durante la lettura dei messaggi, il server ora:
  - registra log diagnostici più dettagliati (partecipanti, id messaggio, errore di base64, ecc.);
  - non restituisce il ciphertext grezzo all'interfaccia utente: restituisce un placeholder leggibile tipo "[DECRYPTION FAILED]" in corrispondenza del messaggio non decifrabile.
- Questo evita la visualizzazione di dati binari o errori incomprensibili nella UI.

## Correzioni UX e bug tecnici applicati
- Evitare flicker del loader:
  - nuovo campo `loading_private_chats: HashSet<String>` in `ChatAppState` per tracciare quali chat sono in fetch/loading.
  - la view controlla questo set per decidere se mostrare loader vs placeholder.
- Chiarezza sul lifecycle delle flag `loading`:
  - le flag di loading vengono rimosse quando arrivano `NewMessagesReceived` o `PrivateMessagesLoaded`.
- Problemi di borrow/lifetime in codice asincrono GUI:
  - i comandi asincroni che spawnano `Command::perform` non devono catturare `&mut self`; è stata fatta la copia/clonazione esplicita dei valori necessari (es. username, token, host, svc) prima di creare le future asincrone.
  - alcune funzioni di update sono state centralizzate in `ChatAppState::update` per semplificare i borrow e mantenere il codice pulito.

## Decisioni aperte e alternative
- Dove cifrare i messaggi?
  - Opzione A (semplificata): il client invia plaintext al server; il server cifra prima di salvarli. Più semplice da implementare ma richiede fiducia nel server.
  - Opzione B (più sicura): il client cifra i messaggi prima dell'invio usando lo stesso master key/chat-key; il server salva il ciphertext così com'è. Richiede che il client abbia accesso al `ENCRYPTION_MASTER_KEY` (o a una derivazione) — comporta rischi di distribuzione della chiave.
- Attuale implementazione: il repository contiene helper per usare una master key persistente e la logica server-side per crittografare/decrittografare; se si desidera il client-side encryption, occorre riallineare `chat_service` per cifrare lato client con le stesse helper e derivazioni.

## Raccomandazioni e next steps
- Assicurarsi che `.env` nel deployment contenga `ENCRYPTION_MASTER_KEY` valido per mantenere la continuità dei messaggi.
- Unificare formalmente la funzione di derivazione della chat-key (documentare input, sorting e canonicalization dei partecipanti) e aggiungere test automatizzati che verificano che client e server producano la stessa key.
- Se si vuole privacy end-to-end, implementare client-side encryption (Opzione B) con attenzione a come distribuire/proteggere la master key.
- Aggiungere test unitari/integration tests per: derivazione chat-key, encrypt/decrypt roundtrip, fallback su decryption error.

## Riassunto rapido
- Loader UI: `loading_private_chats` evita flicker.
- Polling: fetch periodico continuo con sleep breve e trigger immediato dopo invio.
- Invio: input resettato subito, loading impostato se history mancante, refresh immediato richiesto.
- Crittografia: master key persistente via `.env`, helper condivisi in `src/common/crypto.rs`.
- Robustezza: decryption failure -> placeholder e log dettagliati.

Se vuoi, posso:
- Generare diagrammi sequenziali (sequenza) per il flusso client/server.
- Aggiungere una checklist operativa per deploy (come generare/salvare il master key in `.env`).
- Implementare client-side encryption riutilizzando gli helper esistenti.

---
Documento generato automaticamente — breve, focalizzato e aggiornabile su richiesta.
