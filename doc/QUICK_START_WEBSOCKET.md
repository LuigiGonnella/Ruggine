# Ruggine WebSocket + Redis Startup Guide

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
# [INFO] WebSocket Server listening on 127.0.0.1:8081
```

### 4. Avvia il Client GUI
```powershell
# Il client GUI si connetterà automaticamente via WebSocket
cargo run --bin ruggine_gui
```

## Verifica Funzionamento

### Test Redis Pub/Sub
```powershell
# Terminal 1 - Subscriber
redis-cli
SUBSCRIBE chat_messages

# Terminal 2 - Publisher  
redis-cli
PUBLISH chat_messages '{"from":"test","to":"all","content":"Hello WebSocket!"}'
```

### Test WebSocket Server
Puoi usare un tool come `websocat` per testare il WebSocket server:
```powershell
# Installa websocat
cargo install websocat

# Connetti al WebSocket server
websocat ws://127.0.0.1:8081
```

## Architettura del Sistema

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

## Flusso dei Messaggi

1. **Autenticazione**: Client si autentica via TCP
2. **Connessione WebSocket**: Client apre connessione WebSocket per messaggi real-time
3. **Invio Messaggio**: 
   - Client invia messaggio via WebSocket
   - Server salva nel database
   - Server pubblica su Redis
   - Tutti i client connessi ricevono il messaggio istantaneamente

## Ports utilizzate

- **8080**: TCP Server (autenticazione, comandi)
- **8081**: WebSocket Server (messaggi real-time)  
- **6379**: Redis Server (pub/sub, cache)

## Debugging

### Log Levels
```powershell
# Più verbose per debugging
$env:RUST_LOG = "debug"
cargo run --bin ruggine_server

# Solo errori per produzione
$env:RUST_LOG = "error"
```

### Redis Monitoring
```powershell
# Monitor tutti i comandi Redis
redis-cli monitor

# Vedi client connessi
redis-cli client list

# Controlla memoria usata
redis-cli info memory
```

## Performance Tips

1. **Redis Persistence**: Configurata in `redis.conf` per salvare messaggi su disco
2. **Connection Pooling**: Il server mantiene un pool di connessioni Redis
3. **Memory Management**: Redis configurato con maxmemory per evitare OOM
4. **WebSocket Keepalive**: Configurato per rilevare connessioni morte

## Troubleshooting Comuni

### "Redis connection failed"
- Verifica che Redis sia in esecuzione: `redis-cli ping`
- Controlla l'URL Redis nelle variabili d'ambiente
- Verifica che la porta 6379 sia disponibile

### "WebSocket connection failed"  
- Verifica che il server Ruggine sia in esecuzione
- Controlla che la porta WebSocket (8081) sia disponibile
- Verifica il firewall/antivirus

### "Messaggi non ricevuti in tempo reale"
- Controlla i log del server per errori WebSocket
- Verifica la connessione Redis con `redis-cli monitor`  
- Controlla che il client sia correttamente connesso al WebSocket
