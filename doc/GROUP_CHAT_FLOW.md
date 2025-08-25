# Flusso e funzionamento della chat di gruppo

Questo documento descrive il flusso end-to-end, la logica client/server e le soluzioni adottate per la chat di gruppo nel progetto Ruggine, basato sull'architettura già implementata per le chat private.

## Scopo
Breve guida su come funziona la chat di gruppo: apertura, polling in tempo reale, invio/ricezione messaggi, gestione dello stato di caricamento e crittografia persistente per più membri.

## File chiave
- `src/client/models/app_state.rs` — stato dell'app, gestione dei messaggi UI per gruppi, loader per chat di gruppo, logica per invio e caricamento messaggi.
- `src/client/gui/views/group_chat.rs` — rendering della view della chat di gruppo (loader, placeholder "Nessun messaggio ancora...", lista messaggi).
- `src/client/gui/app.rs` — inizializzazione, delega update all'app state, orchestrazione del polling periodico per gruppi.
- `src/client/services/chat_service.rs` — chiamate di rete verso il server (get/send group messages).
- `src/common/crypto.rs` — helper di crittografia condivisi (parsing del master key, derivazione chat-key per gruppi, encrypt/decrypt).
- `src/server/config.rs` — caricamento della configurazione server (incluso `ENCRYPTION_MASTER_KEY`).
- `src/server/messages.rs` — logica di memorizzazione e lettura messaggi sul server per gruppi (encrypt/decrypt, fallback su errore di decrittazione).

## Flusso lato client
1. Apertura chat di gruppo
   - L'azione `OpenGroupChat(group_id, group_name)` imposta lo stato `AppState::GroupChat(group_id, group_name)` e inserisce `group_id` in `loading_group_chats`.
   - Viene lanciato un comando asincrono `StartGroupMessagePolling { group_id }` per iniziare il polling dei nuovi messaggi.

2. Visualizzazione
   - Se `loading_group_chats` contiene il group_id, la view mostra un loader (es. "Caricamento messaggi...").
   - Se la chat non è in loading e non ci sono messaggi presenti nella cache `group_chats[group_id]`, la view mostra il placeholder "Nessun messaggio ancora...".
   - Se ci sono messaggi cache, la view mostra l'elenco dei messaggi.

3. Polling e aggiornamenti in tempo reale
   - L'app esegue polling periodico con piccole pause (es. 100 ms) quando `group_polling_active` è true.
   - Quando arrivano `NewGroupMessagesReceived { group_id, messages }`, i messaggi vengono inseriti in `group_chats[group_id]` e l'entry `group_id` viene rimossa da `loading_group_chats`.
   - Dopo aver inserito i messaggi, il polling continua automaticamente (ri-lancio del fetch asincrono con sleep breve).

4. Invio messaggio
   - `SendGroupMessage { group_id }` verifica che l'input non sia vuoto e se esiste `session_token` esegue l'invio asincrono tramite `ChatService`.
   - Se la cronologia della chat non è ancora in cache, il client imposta `loading_group_chats.insert(group_id.clone())` per mostrare il loader fino al refresh.
   - L'input viene resettato immediatamente (UX ottimistica) e viene lanciato il comando di invio; al termine si chiede un `TriggerImmediateGroupRefresh { group_id }` per forzare il refresh immediato della cronologia.

## Logica lato server e persistenza
- Il server memorizza i messaggi in forma cifrata usando una chat-key derivata da un master key e dall'elenco dei membri del gruppo.
- Derivazione: chat-key = SHA256(master_key || sorted_group_members) (deterministico e simmetrico tra client/server se entrambi usano gli stessi input).

## Crittografia e gestione della master key per gruppi
La logica è identica a quella delle chat private, ma con alcune differenze:
- Per i gruppi, la chat-key viene derivata dall'elenco di tutti i membri del gruppo (ordinati alfabeticamente per consistenza).
- Il server recupera automaticamente la lista dei membri del gruppo dalla tabella `group_members` per generare la chiave corretta.
- Quando un membro viene aggiunto o rimosso dal gruppo, i messaggi precedenti rimangono cifrati con la vecchia chiave, mentre i nuovi messaggi usano la nuova chiave derivata dalla membership aggiornata.

## Differenze rispetto alle chat private

### Gestione dei membri
- **Chat private**: Solo 2 partecipanti fissi (sender e recipient).
- **Chat gruppi**: N partecipanti variabili, gestiti tramite `group_members` table.

### Derivazione chiave crittografica
- **Chat private**: `sorted([user_id1, user_id2])`
- **Chat gruppi**: `sorted([all_group_member_ids])`

### Identificazione chat
- **Chat private**: `chat_id = "private:{user1_id}-{user2_id}"`
- **Chat gruppi**: `chat_id = "group:{group_id}"`

### Visualizzazione messaggi
- **Chat private**: Mostra username del sender (convertito da user_id).
- **Chat gruppi**: Mostra user_id del sender (può essere esteso per mostrare username).

## Robustezza su errori di decrittazione
- Identica alla logica delle chat private: se la decrittazione fallisce, viene mostrato "[DECRYPTION FAILED]" invece del ciphertext grezzo.
- Log diagnostici dettagliati per debugging senza esporre contenuti sensibili.

## Correzioni UX e bug tecnici applicati
- Evitare flicker del loader: nuovo campo `loading_group_chats: HashSet<String>` per tracciare quali chat di gruppo sono in fetch/loading.
- Chiarezza sul lifecycle delle flag `loading`: rimosse quando arrivano `NewGroupMessagesReceived` o `GroupMessagesLoaded`.
- Gestione asincrona: i comandi asincroni clonano esplicitamente i valori necessari prima di creare le future.

## Funzionalità aggiuntive per gruppi
- **Lista gruppi**: `MyGroups` carica la lista dei gruppi dell'utente e apre automaticamente il primo gruppo disponibile.
- **Creazione gruppi**: `CreateGroup { name }` permette di creare nuovi gruppi.
- **Gestione membri**: Supporto per inviti, accettazione/rifiuto, join/leave (già implementato lato server).

## Raccomandazioni e next steps
- **Gestione membership dinamica**: Implementare notifiche quando membri vengono aggiunti/rimossi.
- **Visualizzazione membri**: Mostrare lista membri attivi nel gruppo.
- **Permessi**: Implementare ruoli (admin, moderatore, membro) per gestire chi può invitare/rimuovere membri.
- **Notifiche**: Implementare notifiche push per nuovi messaggi nei gruppi.

## Riassunto rapido
- **Loader UI**: `loading_group_chats` evita flicker.
- **Polling**: fetch periodico continuo con sleep breve e trigger immediato dopo invio.
- **Invio**: input resettato subito, loading impostato se history mancante, refresh immediato richiesto.
- **Crittografia**: master key persistente via `.env`, derivazione chiave basata su membri gruppo.
- **Robustezza**: decryption failure -> placeholder e log dettagliati.
- **Multi-membro**: gestione dinamica dei membri del gruppo per derivazione chiave.

---
Documento generato automaticamente — breve, focalizzato e aggiornabile su richiesta.