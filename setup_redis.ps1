# Script per installare e configurare Redis per Ruggine
param(
    [switch]$SkipInstall = $false
)

Write-Host "🦀 Setup Redis per Ruggine WebSocket" -ForegroundColor Cyan
Write-Host "=================================" -ForegroundColor Cyan

# Funzione per scaricare Redis
function Download-Redis {
    $redisUrl = "https://github.com/tporadowski/redis/releases/download/v5.0.14.1/Redis-x64-5.0.14.1.zip"
    $downloadPath = "$env:TEMP\redis.zip"
    $extractPath = ".\redis"
    
    Write-Host "📥 Download Redis da GitHub..." -ForegroundColor Yellow
    
    try {
        # Scarica Redis
        Invoke-WebRequest -Uri $redisUrl -OutFile $downloadPath -UseBasicParsing
        Write-Host "✅ Download completato" -ForegroundColor Green
        
        # Estrai Redis nella cartella del progetto
        if (Test-Path $extractPath) {
            Remove-Item $extractPath -Recurse -Force
        }
        
        Add-Type -AssemblyName System.IO.Compression.FileSystem
        [System.IO.Compression.ZipFile]::ExtractToDirectory($downloadPath, $extractPath)
        Write-Host "✅ Redis estratto in .\redis\" -ForegroundColor Green
        
        # Cleanup
        Remove-Item $downloadPath -Force
        
        return $true
    }
    catch {
        Write-Host "❌ Errore durante il download: $_" -ForegroundColor Red
        return $false
    }
}

# Verifica se Redis è già installato
$redisInstalled = $false
$redisPath = ""

# Controlla percorsi comuni
$commonPaths = @(
    "C:\Program Files\Redis\redis-server.exe",
    ".\redis\redis-server.exe"
)

foreach ($path in $commonPaths) {
    if (Test-Path $path) {
        $redisInstalled = $true
        $redisPath = $path
        Write-Host "✅ Redis trovato in: $path" -ForegroundColor Green
        break
    }
}

# Scarica Redis se non installato
if (-not $redisInstalled -and -not $SkipInstall) {
    Write-Host "📦 Redis non trovato, procedo con il download..." -ForegroundColor Yellow
    
    if (Download-Redis) {
        $redisPath = ".\redis\redis-server.exe"
        $redisInstalled = $true
    }
}

if (-not $redisInstalled) {
    Write-Host "❌ Redis non disponibile. Opzioni:" -ForegroundColor Red
    Write-Host "  1. Esegui: .\setup_redis.ps1" -ForegroundColor Yellow
    Write-Host "  2. Installa manualmente da: https://github.com/tporadowski/redis/releases" -ForegroundColor Yellow
    exit 1
}

# Testa Redis
Write-Host "`n🧪 Test di Redis..." -ForegroundColor Yellow

try {
    # Avvia Redis in background per il test
    $redisProcess = Start-Process -FilePath $redisPath -ArgumentList "redis.conf" -PassThru -WindowStyle Hidden
    Start-Sleep -Seconds 2
    
    # Testa connessione
    $testResult = & ".\redis\redis-cli.exe" ping 2>$null
    
    if ($testResult -eq "PONG") {
        Write-Host "✅ Redis funziona correttamente!" -ForegroundColor Green
        
        # Mostra informazioni
        Write-Host "`n📊 Informazioni Redis:" -ForegroundColor Cyan
        Write-Host "• Percorso: $redisPath" -ForegroundColor Gray
        Write-Host "• Configurazione: redis.conf" -ForegroundColor Gray
        Write-Host "• Porta: 6379" -ForegroundColor Gray
        Write-Host "• Comando start: redis-server redis.conf" -ForegroundColor Gray
        Write-Host "• Test connessione: redis-cli ping" -ForegroundColor Gray
    }
    else {
        Write-Host "⚠️  Redis avviato ma non risponde correttamente" -ForegroundColor Orange
    }
    
    # Ferma il processo di test
    if ($redisProcess -and -not $redisProcess.HasExited) {
        Stop-Process -Id $redisProcess.Id -Force
    }
}
catch {
    Write-Host "❌ Errore durante il test di Redis: $_" -ForegroundColor Red
}

Write-Host "`n🚀 Prossimi passi:" -ForegroundColor Green
Write-Host "1. Setup completato! Usa lo script di avvio:" -ForegroundColor White
Write-Host "   .\startup.ps1                    # Avvia solo Redis" -ForegroundColor Yellow
Write-Host "   .\startup.ps1 -StartClient       # Avvia tutto (Redis + Server + Client)" -ForegroundColor Yellow
Write-Host "`n2. Oppure avvio manuale:" -ForegroundColor White
Write-Host "   redis-server redis.conf" -ForegroundColor Gray
Write-Host "   cargo run --bin ruggine-server" -ForegroundColor Gray
Write-Host "   cargo run --bin ruggine-gui" -ForegroundColor Gray
