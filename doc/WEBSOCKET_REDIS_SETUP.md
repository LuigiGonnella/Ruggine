# WebSocket + Redis Setup per Ruggine Chat

## Panoramica
Il sistema di chat di Ruggine è stato aggiornato per utilizzare WebSockets + Redis invece del polling del database per migliorare le prestazioni e la scalabilità.

## Architettura

### Server
- **WebSocket Manager**: `src/server/websocket.rs`
  - Gestisce connessioni WebSocket multiple
  - Integrazione con Redis pub/sub per messaggi cross-instance
  - Broadcasting automatico dei messaggi a tutti i client connessi

### Client
- **WebSocket Service**: `src/client/services/websocket_service.rs`
  - Gestisce la connessione WebSocket al server
  - Invia e riceve messaggi in tempo reale
  - Integrato con il ChatService esistente

## Setup Redis

### 1. Installazione Redis
```bash
# Windows (usando Chocolatey)
choco install redis-64

# O scarica da: https://github.com/microsoftarchive/redis/releases
```

### 2. Configurazione
```bash
# Avvia Redis con la configurazione fornita
redis-server redis.conf
```

### 3. Variabili d'ambiente
```bash
# Imposta l'URL di Redis (opzionale, default: redis://127.0.0.1/)
$env:REDIS_URL = "redis://127.0.0.1:6379"
```

## Utilizzo

### Server
```bash
# Il server WebSocket si avvia automaticamente sulla porta TCP_PORT + 1
cargo run --bin ruggine_server

# Example: se TCP è sulla porta 8080, WebSocket sarà su 8081
```

### Client
```bash
# Il client GUI si connette automaticamente via WebSocket
cargo run --bin ruggine_gui
```

## Messaggi in Tempo Reale

### Tipi di Messaggio WebSocket
```rust
pub enum WebSocketMessage {
    ChatMessage {
        from: String,
        to: String,
        content: String,
        timestamp: String,
        is_group: bool,
    },
    UserJoined { username: String },
    UserLeft { username: String },
    GroupCreated { group_name: String, creator: String },
    // Altri tipi...
}
```

### Redis Channels
- `chat_messages`: Messaggi di chat privati e di gruppo
- `user_events`: Eventi di join/leave utenti
- `group_events`: Eventi di creazione/modifica gruppi

## Vantaggi rispetto al Polling

1. **Latenza ridotta**: Messaggi istantanei invece di attesa polling
2. **Meno carico sul database**: No query ripetute
3. **Scalabilità**: Redis pub/sub supporta multiple istanze server
4. **Efficienza di rete**: Solo messaggi necessari invece di query periodiche

## Debugging

### Monitoraggio Redis
```bash
# Connetti a Redis CLI
redis-cli

# Monitora tutti i comandi
MONITOR

# Verifica le connessioni attive
CLIENT LIST

# Controlla i canali pub/sub attivi
PUBSUB CHANNELS
```

### Log del Server
I log WebSocket sono disponibili nel file di log del server e includono:
- Connessioni/disconnessioni client
- Messaggi inviati/ricevuti
- Errori di connessione Redis

## Compatibilità

Il sistema mantiene la compatibilità con il protocollo TCP esistente:
- Le funzioni di autenticazione continuano via TCP
- I comandi di gestione utenti/gruppi via TCP
- Solo i messaggi di chat utilizzano WebSocket per tempo reale

## Troubleshooting

### Redis non raggiungibile
```
Error: Could not connect to Redis server
```
**Soluzione**: Verifica che Redis sia in esecuzione e l'URL sia corretto

### WebSocket connection failed
```
Error: Failed to connect to WebSocket server
```
**Soluzione**: Verifica che il server sia in esecuzione e la porta WebSocket sia disponibile

### Messaggi non ricevuti in tempo reale
1. Controlla la connessione WebSocket nel client
2. Verifica che Redis pub/sub funzioni correttamente
3. Controlla i log del server per errori

## Performance Considerations

- Redis mantiene i messaggi in memoria per performance ottimali
- La configurazione include persistence su disco per affidabilità
- Maxmemory impostato a 256MB (regolabile in produzione)
- Connection pooling per Redis per gestire multiple connessioni server
