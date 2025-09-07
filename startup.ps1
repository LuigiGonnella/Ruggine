# Ruggine WebSocket + Redis Startup Script
# Automatizza l'avvio di Redis e del server Ruggine

param(
    [switch]$StartRedis = $true,
    [switch]$StartServer = $true,
    [switch]$StartClient = $false,
    [string]$RedisConfig = "redis.conf",
    [string]$LogLevel = "info"
)

Write-Host "🦀 Ruggine WebSocket + Redis Startup Script" -ForegroundColor Cyan
Write-Host "=============================================" -ForegroundColor Cyan

# Imposta variabili d'ambiente
$env:RUST_LOG = $LogLevel
$env:REDIS_URL = "redis://127.0.0.1:6379"

# Funzione per verificare se un processo è in esecuzione
function Test-ProcessRunning {
    param([string]$ProcessName)
    return (Get-Process -Name $ProcessName -ErrorAction SilentlyContinue) -ne $null
}

# Funzione per verificare se una porta è disponibile
function Test-PortAvailable {
    param([int]$Port)
    try {
        $listener = [System.Net.NetworkInformation.IPGlobalProperties]::GetIPGlobalProperties().GetActiveTcpListeners()
        return -not ($listener | Where-Object { $_.Port -eq $Port })
    }
    catch {
        return $true
    }
}

# 1. Avvia Redis se richiesto
if ($StartRedis) {
    Write-Host "`n📊 Avvio Redis Server..." -ForegroundColor Yellow
    
    if (Test-ProcessRunning "redis-server") {
        Write-Host "✅ Redis è già in esecuzione" -ForegroundColor Green
    }
    elseif (-not (Test-PortAvailable 6379)) {
        Write-Host "⚠️  Porta 6379 occupata, ma redis-server non rilevato" -ForegroundColor Orange
    }
    else {
        if (Test-Path $RedisConfig) {
            Write-Host "🚀 Avvio Redis con configurazione: $RedisConfig" -ForegroundColor Blue
            Start-Process -FilePath "redis-server" -ArgumentList $RedisConfig -WindowStyle Minimized
            Start-Sleep -Seconds 2
        }
        else {
            Write-Host "🚀 Avvio Redis con configurazione default" -ForegroundColor Blue
            Start-Process -FilePath "redis-server" -WindowStyle Minimized
            Start-Sleep -Seconds 2
        }
        
        # Verifica che Redis sia avviato
        try {
            $result = redis-cli ping 2>$null
            if ($result -eq "PONG") {
                Write-Host "✅ Redis avviato con successo" -ForegroundColor Green
            }
            else {
                Write-Host "❌ Redis non risponde al ping" -ForegroundColor Red
                exit 1
            }
        }
        catch {
            Write-Host "❌ Errore nel testare Redis" -ForegroundColor Red
            exit 1
        }
    }
}

# 2. Avvia il Server Ruggine se richiesto
if ($StartServer) {
    Write-Host "`n🦀 Avvio Ruggine Server..." -ForegroundColor Yellow
    
    if (-not (Test-PortAvailable 8080)) {
        Write-Host "❌ Porta 8080 (TCP) già occupata" -ForegroundColor Red
        exit 1
    }
    
    if (-not (Test-PortAvailable 8081)) {
        Write-Host "❌ Porta 8081 (WebSocket) già occupata" -ForegroundColor Red
        exit 1
    }
    
    Write-Host "🚀 Compilazione e avvio del server..." -ForegroundColor Blue
    Write-Host "📡 TCP Server: 127.0.0.1:8080" -ForegroundColor Gray
    Write-Host "🔌 WebSocket Server: 127.0.0.1:8081" -ForegroundColor Gray
    
    # Avvia il server in una nuova finestra
    Start-Process -FilePath "powershell" -ArgumentList "-Command", "cargo run --bin ruggine-server" -WorkingDirectory $PWD
    
    Write-Host "✅ Server avviato in nuova finestra" -ForegroundColor Green
    Write-Host "📋 Controlla la finestra del server per i log" -ForegroundColor Gray
}

# 3. Avvia il Client GUI se richiesto
if ($StartClient) {
    Write-Host "`n🖥️  Avvio Client GUI..." -ForegroundColor Yellow
    
    # Aspetta un po' per assicurarsi che il server sia pronto
    if ($StartServer) {
        Write-Host "⏳ Attendo che il server sia pronto..." -ForegroundColor Gray
        Start-Sleep -Seconds 5
    }
    
    Write-Host "🚀 Compilazione e avvio del client..." -ForegroundColor Blue
    Start-Process -FilePath "powershell" -ArgumentList "-Command", "cargo run --bin ruggine-gui" -WorkingDirectory $PWD
    
    Write-Host "✅ Client GUI avviato in nuova finestra" -ForegroundColor Green
}

Write-Host "`n🎉 Startup completato!" -ForegroundColor Green
Write-Host "=====================================" -ForegroundColor Cyan

# Mostra informazioni utili
Write-Host "`n📋 Informazioni Sistema:" -ForegroundColor Cyan
Write-Host "• Redis URL: $env:REDIS_URL" -ForegroundColor Gray
Write-Host "• Log Level: $env:RUST_LOG" -ForegroundColor Gray
Write-Host "• TCP Port: 8080" -ForegroundColor Gray
Write-Host "• WebSocket Port: 8081" -ForegroundColor Gray

Write-Host "`n🔧 Comandi Utili:" -ForegroundColor Cyan
Write-Host "• Testa Redis: redis-cli ping" -ForegroundColor Gray
Write-Host "• Monitor Redis: redis-cli monitor" -ForegroundColor Gray
Write-Host "• Setup Redis (se manca): .\setup_redis.ps1" -ForegroundColor Gray

# Se solo Redis è stato avviato, mostra i prossimi passi
if ($StartRedis -and -not $StartServer) {
    Write-Host "`n➡️  Prossimi passi:" -ForegroundColor Yellow
    Write-Host "• Avvia il server: .\startup.ps1 -StartServer" -ForegroundColor Gray
    Write-Host "• O avvia tutto: .\startup.ps1 -StartClient" -ForegroundColor Gray
}

Write-Host "`nPremere Ctrl+C per terminare i processi avviati" -ForegroundColor Yellow
