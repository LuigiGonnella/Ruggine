# Ruggine WebSocket Real-Time Chat System - Technical Documentation

## Quick Start

### 1. Avvia Redis
```powershell
# Avvia Redis con la configurazione fornita
redis-server redis.conf
```

### 2. Verifica Redis
```powershell
# Testa la connessione Redis
redis-cli ping
# Dovrebbe rispondere: PONG
```

### 3. Avvia il Server Ruggine
```powershell
# Il server avvierà automaticamente sia TCP che WebSocket
cargo run --bin ruggine_server

# Output atteso:
# [INFO] Database connected successfully
# [INFO] Redis connected successfully  
# [INFO] TCP Server listening on 127.0.0.1:8080
# [INFO] WebSocket Server listening on 127.0.0.1:5001
```

### 4. Avvia il Client GUI
```powershell
# Il client GUI si connetterà automaticamente via WebSocket
cargo run --bin ruggine_gui
```

## Architettura del Sistema WebSocket

### Panoramica Generale
Il sistema Ruggine implementa una chat real-time utilizzando **WebSocket puri** senza polling HTTP. L'architettura è divisa in due componenti principali:

```
┌─────────────────┐    WebSocket     ┌─────────────────┐
│   Client GUI    │◄─────────────────►│  Ruggine Server │
│                 │                  │                 │
│ - iced GUI      │    TCP (auth)    │ - TCP Server    │
│ - WebSocket     │◄─────────────────►│ - WebSocket Mgr │
│   Service       │                  │ - Redis Client  │
└─────────────────┘                  └─────────────────┘
                                               │
                                               │ Pub/Sub
                                               ▼
                                      ┌─────────────────┐
                                      │   Redis Server  │
                                      │                 │
                                      │ - chat_messages │
                                      │ - user_events   │
                                      │ - group_events  │
                                      └─────────────────┘
```

### 1. Server WebSocket (`src/server/websocket.rs`)

Il server WebSocket gestisce tutte le connessioni real-time e la distribuzione dei messaggi:

#### Gestione Connessioni
```rust
// Struttura principale per gestire le connessioni WebSocket
pub struct WebSocketManager {
    user_connections: Arc<Mutex<HashMap<String, HashMap<String, WebSocketSender>>>>,
    redis_client: Arc<Mutex<redis::Client>>,
    db: Arc<Database>,
}
```

#### Autenticazione WebSocket
```rust
// In handle_websocket_connection()
let auth_msg: WebSocketMessage = serde_json::from_str(&message)?;
if auth_msg.message_type == "auth" {
    // Valida il token di sessione usando il database
    let user_id = validate_session_token(&auth_msg.session_token, &db).await?;
    
    // Registra la connessione per l'utente
    user_connections.lock().await.entry(user_id.clone())
        .or_insert_with(HashMap::new)
        .insert(connection_id, tx.clone());
}
```

#### Routing dei Messaggi
Il server converte i nomi utente in UUID per il routing interno:
```rust
// Conversione username -> user_id per il routing
let target_user_id = match sqlx::query("SELECT id FROM users WHERE username = ?")
    .bind(to_user)
    .fetch_optional(&db_clone.pool)
    .await {
        Ok(Some(row)) => row.get::<String, _>("id"),
        _ => {
            eprintln!("[WS:BROADCAST] ❌ User '{}' not found", to_user);
            return;
        }
    };

// Invio del messaggio a tutte le connessioni dell'utente target
if let Some(user_sessions) = user_connections.get(&target_user_id) {
    for (session_id, sender) in user_sessions {
        let _ = sender.send(Message::Text(message_json.clone())).await;
    }
}
```

### 2. Client WebSocket (`src/client/services/websocket_client.rs`)

Il client gestisce la connessione persistente con il server:

#### Connessione e Autenticazione
```rust
pub async fn connect_and_authenticate(
    url: &str,
    session_token: &str,
) -> Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, String), Box<dyn std::error::Error + Send + Sync>> {
    // Connessione WebSocket
    let (ws_stream, _) = connect_async(url).await?;
    
    // Invio messaggio di autenticazione
    let auth_message = json!({
        "message_type": "auth",
        "session_token": session_token
    });
    
    tx.send(Message::Text(auth_message.to_string())).await?;
}
```

#### Gestione Messaggi Incoming
```rust
// Loop per messaggi in arrivo dal server
while let Some(msg) = rx.next().await {
    if let Ok(Message::Text(text)) = msg {
        let parsed: WebSocketMessage = serde_json::from_str(&text)?;
        
        match parsed.message_type.as_str() {
            "auth_response" => { /* Gestione autenticazione */ },
            "incoming_message" => {
                // Nuovo messaggio ricevuto via WebSocket
                let _ = message_tx.send(parsed).await;
            },
            _ => {}
        }
    }
}
```

### 3. Integrazione GUI (`src/client/gui/app.rs`)

L'applicazione GUI gestisce il ciclo di vita dei WebSocket e l'aggiornamento dell'interfaccia:

#### Inizializzazione WebSocket
```rust
// In handle_auth_result()
if success {
    // Dopo autenticazione TCP successful, connetti WebSocket
    return Command::perform(
        ChatService::connect_websocket("127.0.0.1:5001".to_string()),
        Message::WebSocketConnected,
    );
}
```

#### Loop di Controllo Messaggi
```rust
// Controllo continuo per nuovi messaggi WebSocket
Message::CheckWebSocketMessages => {
    return Command::perform(
        ChatService::check_websocket_messages(),
        Message::WebSocketMessageReceived,
    );
}

Message::WebSocketMessageReceived(result) => {
    match result {
        Ok(Some(ws_message)) => {
            // Nuovo messaggio ricevuto via WebSocket
            if ws_message.message_type == "incoming_message" {
                // Aggiorna la cache locale dei messaggi
                self.add_message_to_cache(&ws_message);
                
                // Riavvia il loop di controllo
                return Command::batch(vec![
                    Command::perform(async {}, |_| Message::CheckWebSocketMessages),
                ]);
            }
        }
    }
}
```

### 4. Servizio Chat (`src/client/services/chat_service.rs`)

Il servizio chat coordina TCP e WebSocket:

#### Invio Messaggi
```rust
pub async fn send_private_message_websocket(
    to_user: &str,
    content: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Invia tramite WebSocket per delivery real-time
    let message = OutgoingChatMessage {
        message_type: "send_message".to_string(),
        chat_type: "private".to_string(),
        to_user: Some(to_user.to_string()),
        group_id: None,
        content: content.to_string(),
    };
    
    WEBSOCKET_SERVICE.send_message(message).await
}
```

#### Ricezione Messaggi
```rust
pub async fn check_websocket_messages() -> Result<Option<WebSocketMessage>, String> {
    // Controlla per nuovi messaggi dal WebSocket
    match WEBSOCKET_SERVICE.receive_message().await {
        Ok(msg) => Ok(Some(msg)),
        Err(_) => Ok(None),
    }
}
```

## Flusso Completo dei Messaggi Real-Time

### 1. Fase di Connessione
```
Client                    Server                     Redis/Database
  |                         |                            |
  |-- TCP Auth (/login) --->|                            |
  |<-- Session Token -------|                            |
  |                         |                            |
  |-- WebSocket Connect --->|                            |
  |-- Auth Message -------->|-- Validate Token --------->|
  |<-- Auth Success --------|<-- User Info --------------|
  |                         |-- Register Connection ---->|
```

### 2. Invio Messaggio (Esempio: luigi → dory)
```
Client Luigi              Server                     Client Dory
     |                      |                           |
     |-- WebSocket Msg ----->|                           |
     |   {"message_type":    |-- Save to DB ----------->|
     |    "send_message",    |                           |
     |    "to_user":"dory",  |-- Lookup dory's ID ------>|
     |    "content":"ciao"}  |                           |
     |                      |-- Find dory's WS conn --->|
     |                      |                           |
     |<-- Echo Confirmation--|-- Forward Message ------->|-- Receive Real-Time
     |                      |   {"message_type":        |   Update UI
     |                      |    "incoming_message",    |
     |                      |    "from":"luigi",        |
     |                      |    "content":"ciao"}      |
```

### 3. Dettaglio Tecnico del Routing

#### Server-Side Message Processing (`websocket.rs:185-220`)
```rust
// 1. Parse del messaggio dal client
let parsed_message: OutgoingChatMessage = serde_json::from_str(&message)?;

// 2. Salvataggio nel database
sqlx::query("INSERT INTO private_messages (sender_id, receiver_id, content, timestamp) VALUES (?, ?, ?, ?)")
    .bind(&user_id)
    .bind(&receiver_id)  
    .bind(&parsed_message.content)
    .bind(timestamp)
    .execute(&db_clone.pool).await?;

// 3. Conversione username -> user_id per routing
let target_user_id = sqlx::query("SELECT id FROM users WHERE username = ?")
    .bind(&parsed_message.to_user.unwrap())
    .fetch_one(&db_clone.pool).await?
    .get::<String, _>("id");

// 4. Lookup connessioni WebSocket attive
if let Some(user_sessions) = user_connections.get(&target_user_id) {
    // 5. Broadcast a tutte le sessioni dell'utente target
    for (session_id, sender) in user_sessions {
        sender.send(Message::Text(response_json)).await?;
    }
}
```

#### Client-Side Message Reception (`websocket_client.rs:95-110`)
```rust
// Loop continuo per messaggi incoming
while let Some(msg) = rx.next().await {
    match msg {
        Ok(Message::Text(text)) => {
            let parsed: WebSocketMessage = serde_json::from_str(&text)?;
            
            match parsed.message_type.as_str() {
                "incoming_message" => {
                    // Messaggio ricevuto da altro utente
                    println!("[WS:CLIENT] New message from {}: {}", 
                        parsed.from.unwrap_or("unknown".to_string()), 
                        parsed.content.unwrap_or("".to_string()));
                    
                    // Invia al thread principale GUI
                    let _ = message_tx.send(parsed).await;
                }
            }
        }
    }
}
```

### 4. Gestione Stato GUI (`app.rs:320-350`)

#### Aggiornamento Real-Time dell'Interfaccia
```rust
Message::WebSocketMessageReceived(result) => {
    if let Ok(Some(ws_message)) = result {
        match ws_message.message_type.as_str() {
            "incoming_message" => {
                // Aggiorna cache locale messaggi
                let chat_key = if ws_message.chat_type == "private" {
                    ws_message.from.clone().unwrap_or_default()
                } else {
                    format!("group_{}", ws_message.group_id.unwrap_or_default())
                };
                
                // Aggiungi messaggio alla cache
                self.private_chats.entry(chat_key.clone())
                    .or_insert_with(Vec::new)
                    .push(/* nuovo messaggio */);
                
                // Riavvia il loop di controllo WebSocket
                return Command::batch(vec![
                    Command::perform(async {}, |_| Message::CheckWebSocketMessages),
                ]);
            }
        }
    }
}
```

### 5. Vantaggi dell'Implementazione WebSocket

#### Performance Real-Time
- **Latenza**: ~1-5ms per delivery locale (vs 500-2000ms polling HTTP)
- **Bandwidth**: Solo dati necessari (vs HTTP headers ripetuti)
- **Scalabilità**: Connessioni persistenti vs richieste continue

#### Persistenza Connessione
```rust
// Gestione automatica riconnessione in websocket_service.rs
impl WebSocketService {
    async fn ensure_connected(&self) -> Result<(), String> {
        if !self.is_connected().await {
            self.reconnect().await?;
        }
        Ok(())
    }
}
```

#### Multi-Session Support
Il sistema supporta multiple sessioni per utente:
```rust
// Ogni utente può avere più connessioni WebSocket attive
user_connections: HashMap<String, HashMap<String, WebSocketSender>>
//                 ^^^^^^          ^^^^^^
//                 user_id         session_id -> websocket_sender
```

## Verifica Funzionamento WebSocket

### Test Manuale WebSocket
```powershell
# Installa websocat per test WebSocket
cargo install websocat

# Connetti al WebSocket server
websocat ws://127.0.0.1:5001

# Invia messaggio di autenticazione (sostituisci con token valido)
{"message_type":"auth","session_token":"your-session-token-here"}

# Invia messaggio di test
{"message_type":"send_message","chat_type":"private","to_user":"luigi","content":"test message"}
```

### Verifica Redis Pub/Sub
```powershell
# Terminal 1 - Monitor tutti i comandi Redis
redis-cli monitor

# Terminal 2 - Avvia client e invia messaggi
cargo run --bin ruggine_gui
# I messaggi appariranno nel monitor Redis
```

### Log di Debug Dettagliati

Il sistema include logging estensivo per debugging:

#### Server Logs (`websocket.rs`)
```
[WS:AUTH] User authenticated: fdfd42be-fab1-4f64-b993-222dc07bc782
[WS:RECV] Received message: {"message_type":"send_message","to_user":"dory"}  
[WS:BROADCAST] ✅ Delivered message to user dory (user_id: fdfd42be-fab1-4f64-b993-222dc07bc782)
```

#### Client Logs (`websocket_client.rs`, `app.rs`)
```
[WS:CLIENT] Connected to ws://127.0.0.1:5001
[WS:CLIENT] Authentication successful for user: Some("fdfd42be-fab1-4f64-b993-222dc07bc782")
[WS:CLIENT] New message from luigi: ciao
[APP] WebSocket connesso, avviando controllo messaggi
```

## Ports e Configurazione

- **8080**: TCP Server (autenticazione, comandi legacy)
- **5001**: WebSocket Server (messaggi real-time)  
- **6379**: Redis Server (pub/sub, cache sessioni)

## Debugging Avanzato e Performance

### Configurazione Log Levels
```powershell
# Debug completo - mostra tutti i log WebSocket
$env:RUST_LOG = "debug"
cargo run --bin ruggine_server

# Solo errori per produzione
$env:RUST_LOG = "error"
cargo run --bin ruggine_server
```

### Monitoring Redis Real-Time
```powershell
# Monitor tutti i comandi Redis in tempo reale
redis-cli monitor

# Vedi client WebSocket connessi (tramite Redis)
redis-cli client list

# Controlla memoria usata da Redis
redis-cli info memory

# Debug specifico per chat messages
redis-cli
PSUBSCRIBE chat_*
```

### Analisi Performance WebSocket

#### Latenza Misurazione
```rust
// In websocket.rs - timestamp dei messaggi per latency tracking
let timestamp = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_secs();

println!("[PERF] Message sent at timestamp: {}", timestamp);
```

#### Memory Usage Tracking
```rust
// In websocket_client.rs - monitoring connessioni attive
println!("[PERF] Active WebSocket connections: {}", 
    user_connections.lock().await.len());
```

### Profiling WebSocket Performance

1. **Throughput Test**: Misura messaggi/secondo
2. **Latency Test**: Tempo round-trip client-server-client  
3. **Memory Test**: Uso memoria con N connessioni simultanee
4. **Stress Test**: Comportamento sotto carico pesante

## Troubleshooting Specifici WebSocket

### "WebSocket connection failed"
```powershell
# 1. Verifica che il server sia in ascolto
netstat -an | findstr 5001

# 2. Test connessione diretta
telnet 127.0.0.1 5001

# 3. Verifica firewall Windows
New-NetFirewallRule -DisplayName "Ruggine WebSocket" -Direction Inbound -Protocol TCP -LocalPort 5001 -Action Allow
```

### "Messaggi non ricevuti in tempo reale"
```powershell
# 1. Controlla log server per errori WebSocket routing
cargo run --bin ruggine_server 2>&1 | findstr "WS:BROADCAST\|ERROR"

# 2. Verifica connessioni Redis attive
redis-cli client list | findstr -i websocket

# 3. Test manually WebSocket flow
websocat ws://127.0.0.1:5001
```

### "Authentication failed su WebSocket"
- Verifica che il session token sia valido tramite TCP `/validate_session`
- Controlla che l'utente esista nel database: `SELECT * FROM users WHERE id = 'user-id'`
- Verifica formato messaggio auth: `{"message_type":"auth","session_token":"token"}`

### "Connessioni WebSocket multiple non funzionano"
```rust
// Debug in websocket.rs - verifica user_connections HashMap
println!("[DEBUG] User {} has {} active connections", 
    user_id, user_connections.get(&user_id).map(|m| m.len()).unwrap_or(0));
```

## Ottimizzazioni Performance

### 1. Redis Connection Pooling
```rust
// In redis_cache.rs - pool di connessioni Redis
pub struct RedisCache {
    pool: r2d2::Pool<redis::Client>,  // Connection pool instead of single connection
}
```

### 2. WebSocket Message Batching
```rust
// Batch multiple messages to reduce syscalls
let mut batch_messages = Vec::new();
// Collect messages...
for message in batch_messages {
    sender.send(Message::Text(message)).await?;
}
```

### 3. Memory Management
```rust
// Periodic cleanup of disconnected WebSocket connections
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        cleanup_dead_connections().await;
    }
});
```

### 4. Database Query Optimization
```rust
// Prepared statements for frequent queries
let stmt = sqlx::query("SELECT id FROM users WHERE username = ?");
let user_id = stmt.bind(username).fetch_one(&pool).await?;
```

## Metriche e Monitoring Produzione

### Key Performance Indicators (KPI)
- **Message Delivery Time**: < 50ms average
- **WebSocket Connection Success Rate**: > 99.5%
- **Memory Usage**: < 100MB per 1000 concurrent users
- **Database Query Time**: < 10ms per message save/retrieve

### Alerting Setup
```powershell
# Monitor script per connessioni WebSocket
$connections = redis-cli client list | Select-String "websocket" | Measure-Object
if ($connections.Count -gt 1000) {
    Write-Host "WARNING: High WebSocket connection count: $($connections.Count)"
}
```
