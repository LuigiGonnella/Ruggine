# Documentazione Progetto: ruggine_modulare

> Aggiornamento: la documentazione tecnica relativa a sessioni, presenza, eventi di audit e TLS
> è stata unificata in `doc/SESSION_AND_TLS.md`. Per i dettagli sul flusso di login, logout,
> auto-login, kicked_out/quit events e la configurazione TLS consultare quel file.
>
> Per informazioni complete sul supporto multi-piattaforma vedere `doc/CROSS_PLATFORM_SUPPORT.md`.
>
> Questo file contiene ancora una panoramica generale del progetto; per dettagli operativi
> e debugging legati alle sessioni usare il documento specifico.

## 1. Idea del Progetto
"ruggine_modulare" è una piattaforma di chat client-server scritta in Rust, progettata per essere modulare, estendibile e facilmente manutenibile. L'obiettivo è fornire una base robusta per applicazioni di messaggistica, con supporto per autenticazione, gestione utenti, gruppi, messaggi, e interfacce sia CLI che GUI.

## 2. Struttura del Progetto

```
src/
  lib.rs                // Libreria principale
  main.rs               // Entry point GUI client
  bin/                  // File binari
    chat_test.rs        // Test client CLI
    db_inspect.rs       // Tool di ispezione database
  client/               // Logica lato client
    mod.rs              // Modulo client
    gui/                // Client GUI con Iced
      app.rs            // Applicazione principale
      mod.rs
      views/            // Viste GUI
        group_chat.rs
        logger.rs
        main_actions.rs
        private_chat.rs
        registration.rs
      widgets/          // Widget GUI
        alert.rs
        input_section.rs
    models/             // Modelli dati client
      app_state.rs      // Stato applicazione
      messages.rs       // Messaggi UI
      mod.rs
      ui_state.rs
    services/           // Servizi client
      chat_service.rs   // Gestione chat TCP
      connection.rs     // Connessioni di base
      message_parser.rs // Parse messaggi server
      mod.rs
      users_service.rs  // Servizi utenti
      websocket_client.rs // Client WebSocket
      websocket_service.rs // Servizio WebSocket
    utils/              // Utility client
      constants.rs      // Costanti
      mod.rs
      session_store.rs  // Gestione sessioni
  common/               // Moduli condivisi
    crypto.rs           // Crittografia condivisa
    mod.rs
    models.rs           // Modelli condivisi
  server/               // Logica lato server
    auth.rs             // Autenticazione
    chat_manager.rs     // Manager chat
    config.rs           // Configurazione server
    connection.rs       // Gestione connessioni TCP
    database.rs         // Persistenza SQLite
    groups.rs           // Gestione gruppi
    main.rs             // Server entry point
    messages.rs         // Gestione messaggi
    mod.rs
    presence.rs         // Presenza utenti
    redis_cache.rs      // Cache Redis
    users.rs            // Gestione utenti
    websocket.rs        // Server WebSocket
  utils/                // Utility generali
    mod.rs
    performance.rs      // Metriche performance
```

## 3. Flow e Funzionamento

### 3.1 Avvio Applicazione
- **GUI Client** (`main.rs`): Avvia l'applicazione Iced GUI
- **Server** (`src/server/main.rs`): Avvia server TCP + WebSocket con Redis
- **Test CLI** (`src/bin/chat_test.rs`): Client di test via TCP
- Il server inizializza database SQLite, Redis, autenticazione e WebSocket manager

### 3.2 Logica Client-Server
- **Client**: invia richieste (login, invio messaggi, creazione gruppi, ecc.) tramite socket TCP/UDP o altro protocollo.
- **Server**: riceve, valida, processa e risponde alle richieste. Gestisce la persistenza e la logica di business.
- **Comunicazione**: i messaggi sono serializzati (es. JSON, MessagePack) e trasmessi tra client e server.

### 3.3 Gestione Utenti
- Registrazione, login, logout.
- Gestione sessioni e permessi.
- Modifica profilo.

### 3.4 Gestione Messaggi
- Invio/ricezione messaggi singoli e di gruppo.
- Notifiche di lettura/consegna.
- Cronologia e ricerca messaggi.

### 3.5 Gestione Gruppi
- Creazione, modifica, eliminazione gruppi.
- Aggiunta/rimozione membri.
- Permessi e ruoli.

### 3.6 Persistenza Dati
- Database locale (es. SQLite) per utenti, messaggi, gruppi.
- Moduli di accesso dati separati per estendibilità.

### 3.7 Interfaccia CLI/GUI
- CLI: comandi testuali per tutte le operazioni.
- GUI: interfaccia grafica modulare, con viste e widget personalizzabili.

## 4. Comandi Principali (CLI)
- `login <username> <password>`: autenticazione utente
- `register <username> <password>`: registrazione
- `send <user|group> <message>`: invio messaggio
- `create_group <name> <members>`: crea gruppo
- `list_users`: mostra utenti
- `list_groups`: mostra gruppi
- `logout`: disconnessione

## 5. Logica di Funzionamento delle Features

### 5.1 Modularità
- Ogni feature (auth, chat, gruppi, ecc.) è un modulo separato.
- Interfacce e trait permettono l'estensione e la sostituzione di componenti.
- I moduli comuni (`common/`) contengono tipi e funzioni condivise.

### 5.2 Estendibilità
- Nuove funzionalità possono essere aggiunte creando nuovi moduli o estendendo quelli esistenti.
- La struttura favorisce l'iniezione di dipendenze e la separazione delle responsabilità.

### 5.3 Tracciabilità
- Ogni operazione è loggata (file di log, console, database).
- Gli errori sono gestiti centralmente.
- Le modifiche ai dati sono versionate dove necessario.

## 6. Estensione del Progetto
- Per aggiungere una nuova feature, creare un nuovo modulo in `client/` o `server/`.
- Implementare i trait/interfacce richieste.
- Aggiornare la documentazione e i comandi CLI/GUI.
- Scrivere test per la nuova funzionalità.

## 7. Best Practices
- Separazione netta tra client e server.
- Uso di trait per l'interfacciamento tra moduli.
- Documentazione dettagliata per ogni modulo.
- Test unitari e di integrazione.
- Logging e gestione errori centralizzata.

## 8. Esempio di Flow (Login e Invio Messaggio)
1. **Login**:
   - Client invia credenziali al server.
   - Server valida e risponde con token/sessione.
2. **Invio Messaggio**:
   - Client invia messaggio (testo, destinatario) al server.
   - Server valida, salva e inoltra al destinatario.
   - Client riceve conferma di consegna.

## 9. Documentazione Moduli
- Ogni modulo contiene un README con:
  - Scopo
  - API pubbliche
  - Esempi d'uso
  - Dipendenze

## 10. Estendibilità futura
- Possibilità di aggiungere:
  - Supporto multi-server/distribuito
  - Crittografia end-to-end
  - Plugin di terze parti
  - Integrazione con servizi esterni

---

Questa documentazione fornisce una panoramica completa e dettagliata del progetto "ruggine_modulare", garantendo tracciabilità, estendibilità e facilità di manutenzione.

---

## 11. Elenco Comandi Dettagliato

### Comandi pubblici (non richiedono login)
- `/register <username> <password>`  
  Registra un nuovo utente.  
  Esempio: `/register mario superpass`

- `/login <username> <password>`  
  Effettua il login e restituisce un token di sessione.  
  Esempio: `/login mario superpass`

- `/users`  
  Elenca gli utenti online.

- `/all_users`  
  Elenca tutti gli utenti registrati.

---

### Comandi autenticati (richiedono session_token)

#### Messaggistica
- `/send <group> <message>`  
  Invia un messaggio al gruppo specificato.  
  Sintassi interna: `/send <token> <group> <message>`

- `/send_private <user> <message>`  
  Invia un messaggio privato a un utente.  
  Sintassi interna: `/send_private <token> <user> <message>`

- `/private <user> <message>`  
  Alias di `/send_private`.

- `/get_group_messages <group>`  
  Recupera la cronologia dei messaggi di un gruppo.  
  Sintassi interna: `/get_group_messages <token> <group>`

- `/get_private_messages <user>`  
  Recupera la cronologia dei messaggi privati con un utente.  
  Sintassi interna: `/get_private_messages <token> <user>`

- `/delete_group_messages <group>`  
  Elimina tutti i messaggi di un gruppo.  
  Sintassi interna: `/delete_group_messages <token> <group>`

- `/delete_private_messages <user>`  
  Elimina tutti i messaggi privati con un utente.  
  Sintassi interna: `/delete_private_messages <token> <user>`

---

#### Gestione gruppi
- `/create_group <group_name>`  
  Crea un nuovo gruppo.

- `/my_groups`  
  Elenca i gruppi di cui l'utente è membro.

- `/invite <user> <group>`  
  Invita un utente in un gruppo.

- `/my_invites`  
  Elenca gli inviti ricevuti.

- `/accept_invite <invite_id>`  
  Accetta un invito a un gruppo.

- `/reject_invite <invite_id>`  
  Rifiuta un invito a un gruppo.

- `/join_group <group_name>`  
  Entra in un gruppo.

- `/leave_group <group_name>`  
  Esce da un gruppo.

---

#### Gestione sessione
- `/logout`  
  Disconnette l'utente e invalida la sessione.

---

### Note di parsing e sintassi
- I comandi che richiedono autenticazione aggiungono il token come primo argomento dopo il comando.
- La CLI effettua parsing e validazione dei parametri prima di inviare la richiesta al server.
- La risposta del server è sempre visualizzata con `[SERVER] <risposta>`.

---

### Esempio di flusso completo

1. Registrazione:  
   `/register mario superpass`

2. Login:  
   `/login mario superpass`  
   (ricezione token di sessione)

3. Creazione gruppo:  
   `/create_group amici`

4. Invio messaggio al gruppo:  
   `/send amici Ciao a tutti!`

5. Invio messaggio privato:  
   `/send_private luigi Ciao Luigi!`

6. Visualizzazione messaggi:  
   `/get_group_messages amici`  
   `/get_private_messages luigi`

7. Gestione inviti:  
   `/invite luigi amici`  
   `/my_invites`  
   `/accept_invite <id_invito>`

8. Uscita dal gruppo:  
   `/leave_group amici`

9. Logout:  
   `/logout`

---

