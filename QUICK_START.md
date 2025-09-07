# 🚀 Guida Avvio Rapido Ruggine

## Setup Iniziale (Una Volta Sola)
```powershell
# 1. Prima volta: installa Redis
.\setup_redis.ps1

# Fatto! Non serve più ripeterlo
```

## Avvio Quotidiano (Un Solo Comando)
```powershell
# Avvia TUTTO automaticamente:
.\startup.ps1 -StartClient
```

**Questo comando singolo:**
- 🟢 Avvia Redis (se non già attivo)
- 🟢 Compila e avvia il server Ruggine
- 🟢 Compila e avvia il client GUI
- 🟢 Gestisce l'attesa tra i componenti
- 🟢 Apre tutto in finestre separate

## Comandi Alternativi
```powershell
# Solo Redis + Server (senza GUI)
.\startup.ps1

# Solo Redis
.\startup.ps1 -StartRedis -StartServer:$false

# Server già avviato, solo GUI
.\startup.ps1 -StartRedis:$false -StartServer:$false -StartClient
```

## 🎯 Risultato
**UN COMANDO → TUTTO FUNZIONA!**

Non devi:
- ❌ Avviare Redis manualmente
- ❌ Scrivere `cargo run --bin ruggine-server`
- ❌ Scrivere `cargo run --bin ruggine-gui`
- ❌ Gestire l'ordine di avvio
- ❌ Aprire terminali multipli

## 🔄 Workflow Tipico
```powershell
# Mattina:
.\startup.ps1 -StartClient

# Lavori tutto il giorno...

# Sera: Ctrl+C nelle finestre per fermare tutto
```

**È tutto automatizzato! 🎉**
