# 🚀 Guida Completa Setup Ruggine WebSocket + Redis

## ✅ **Situazione Attuale**
- ✅ Redis è già installato: `C:\Program Files\Redis\redis-server.exe`
- ✅ Progetto Ruggine compila correttamente
- ✅ WebSocket infrastructure implementata
- ✅ Configurazione Redis preparata (`redis.conf`)

## 📋 **Procedura Step-by-Step**

### **Step 1: Avvia Redis Server**
```powershell
# Opzione A - Con configurazione custom (raccomandato)
cd "c:\Users\dorot\OneDrive - Politecnico di Torino\Desktop\Ruggine"
& "C:\Program Files\Redis\redis-server.exe" redis.conf

# Opzione B - Con configurazione default
& "C:\Program Files\Redis\redis-server.exe"
```

**💡 Cosa succede:** Redis si avvia e resta in ascolto sulla porta 6379. Lascia questa finestra aperta!

---

### **Step 2: Verifica Redis (nuovo terminale)**
```powershell
# Testa se Redis risponde
& "C:\Program Files\Redis\redis-cli.exe" ping
# Dovrebbe rispondere: PONG
```

---

### **Step 3: Avvia Ruggine Server (nuovo terminale)**
```powershell
cd "c:\Users\dorot\OneDrive - Politecnico di Torino\Desktop\Ruggine"
cargo run --bin ruggine-server
```

**💡 Cosa succede:** 
- Server TCP si avvia sulla porta 8080
- Server WebSocket si avvia sulla porta 8081
- Si connette automaticamente a Redis

**🔍 Output atteso:**
```
[INFO] Database connected successfully
[INFO] Redis connected successfully  
[INFO] TCP Server listening on 0.0.0.0:8080
[INFO] WebSocket Server listening on 0.0.0.0:8081
```

---

### **Step 4: Avvia Client GUI (nuovo terminale)**
```powershell
cd "c:\Users\dorot\OneDrive - Politecnico di Torino\Desktop\Ruggine"
cargo run --bin ruggine-gui
```

**💡 Cosa succede:** Si apre l'interfaccia grafica che si connette automaticamente via WebSocket per i messaggi real-time.

---

## 🤖 **Procedura Automatica (Alternativa)**

Usa lo script che ho creato per te:

```powershell
# Avvia tutto automaticamente
.\startup.ps1 -StartClient

# Solo Redis e Server
.\startup.ps1 -StartServer

# Solo Redis
.\startup.ps1 -StartRedis
```

---

## 🔧 **Configurazione WebSocket**

**NON devi configurare nulla manualmente per WebSocket!** È tutto automatico:

1. **Server Side**: WebSocket manager si avvia automaticamente quando lanci `ruggine_server`
2. **Client Side**: Il client GUI si connette automaticamente al WebSocket
3. **Fallback**: Se WebSocket non funziona, usa automaticamente TCP

### **Porte utilizzate:**
- **8080**: Server TCP principale (autenticazione, comandi)
- **8081**: Server WebSocket (messaggi real-time)
- **6379**: Redis (pub/sub, cache)

---

## 🧪 **Test del Sistema**

### **1. Test Redis:**
```powershell
& "C:\Program Files\Redis\redis-cli.exe" ping
# Output: PONG
```

### **2. Test WebSocket (con tool esterno):**
```powershell
# Se hai websocat installato
websocat ws://127.0.0.1:8081

# Oppure con browser: apri console e scrivi:
# const ws = new WebSocket('ws://localhost:8081');
# ws.onopen = () => console.log('Connected!');
```

### **3. Test Completo:**
1. Avvia Redis, Server, e 2 Client GUI
2. Fai login su entrambi con utenti diversi
3. Invia un messaggio → dovrebbe apparire istantaneamente sull'altro client

---

## 🚨 **Troubleshooting**

### **"Redis connection failed"**
```powershell
# Verifica se Redis è in esecuzione
netstat -an | findstr :6379

# Se non c'è output, Redis non è avviato
& "C:\Program Files\Redis\redis-server.exe" redis.conf
```

### **"WebSocket connection failed"**
```powershell
# Verifica se il server è in esecuzione
netstat -an | findstr :8081

# Controlla i log del server per errori
```

### **"Server won't start"**
```powershell
# Verifica che le porte siano libere
netstat -an | findstr :8080
netstat -an | findstr :8081

# Se occupate, chiudi i processi o cambia porta in config
```

---

## 📁 **File di Configurazione**

### **redis.conf** (già creato)
- Configurazione ottimizzata per Ruggine
- Persistence abilitata
- Memory management configurato

### **startup.ps1** (già creato)
- Script automatico per avviare tutto
- Gestione errori inclusa
- Output colorato per debug

---

## 🎯 **Vantaggi del Nuovo Sistema**

1. **Messaggi Istantanei**: No più polling, messaggi appaiono subito
2. **Meno Carico Database**: Solo per autenticazione, non per messaggi
3. **Scalabile**: Redis permette multiple istanze server
4. **Affidabile**: Fallback automatico da WebSocket a TCP

---

## ⭐ **Quick Start**

```powershell
# 1. Un comando per avviare tutto:
.\startup.ps1 -StartClient

# 2. Apri browser su: http://localhost:8081 (per test WebSocket)

# 3. Enjoy real-time chat! 🎉
```
