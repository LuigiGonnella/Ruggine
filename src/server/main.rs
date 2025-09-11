// src/server/main.rs
// Entry point per il server ruggine_modulare
use ruggine_modulare::server::{config::ServerConfig, database::Database, connection::Server};
use ruggine_modulare::server::websocket::ChatWebSocketManager;
use ruggine_modulare::utils::performance;
use std::sync::Arc;
use tokio::net::TcpListener;
use log::{info, error};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Configura logging
    let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    std::env::set_var("RUST_LOG", &log_level); //setto env var per usare log::info
    env_logger::init();

    let config = ServerConfig::from_env();

    // TLS hint for the operator
    if config.enable_encryption {
        log::info!("TLS is enabled; set TLS_CERT_PATH and TLS_KEY_PATH env vars to point to cert and key PEM files.");
    } else {
        log::info!("TLS is disabled; connections will be plain TCP.");
    }

    // Initialize database and server
    let database = Arc::new(Database::connect(&config.database_url).await?);
    
    // Run database migrations to create tables if they don't exist
    info!("🗄️ Running database migrations...");
    database.migrate().await.map_err(|e| {
        error!("Database migration failed: {}", e);
        e
    })?;
    info!("✅ Database migrations completed successfully");
    
    let presence = ruggine_modulare::server::presence::PresenceRegistry::new();
    let server = Server { 
        db: database.clone(), 
        config: config.clone(), 
        presence 
    };

    // Start performance logger in background
    let perf_log_path = std::env::var("PERFORMANCE_LOG_PATH")
        .unwrap_or_else(|_| "data/ruggine_performance.log".to_string());
    let perf_db = database.clone();
    tokio::spawn(async move {
        info!("📊 Starting performance logger - logging every 120 seconds to: {}", perf_log_path);
        performance::start_performance_logger(perf_db, &perf_log_path).await;
    });

    // Initialize WebSocket manager with Redis
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let ws_manager = Arc::new(ChatWebSocketManager::new(&redis_url).await?);
    
    // Start Redis subscriber for cross-instance messaging
    ws_manager.start_redis_subscriber().await?;
    
    // Start WebSocket server on a different port
    let ws_port = config.port + 1; // WebSocket su porta +1 rispetto al server principale
    let ws_host = config.host.clone();
    let ws_manager_clone = ws_manager.clone();
    let database_clone = database.clone();
    let config_clone = config.clone();
    tokio::spawn(async move {
        if let Err(e) = start_websocket_server(&format!("{}:{}", ws_host, ws_port), ws_manager_clone, database_clone, config_clone).await {
            error!("WebSocket server error: {}", e);
        }
    });

    info!("WebSocket server started on {}:{}", config.host, ws_port);

    server.run(&format!("{}:{}", config.host, config.port)).await?;
    Ok(())
}

async fn start_websocket_server(
    addr: &str, 
    ws_manager: Arc<ChatWebSocketManager>,
    database: Arc<Database>,
    config: ServerConfig
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!("WebSocket server listening on {}", addr);

    while let Ok((stream, addr)) = listener.accept().await {
        info!("New WebSocket connection from {}", addr);
        let ws_manager = ws_manager.clone();
        let database = database.clone();
        let config = config.clone();
        
        tokio::spawn(async move {
            match tokio_tungstenite::accept_async(stream).await {
                Ok(ws_stream) => {
                    // Usa l'autenticazione corretta invece di user_id fittizio
                    if let Err(e) = ws_manager.handle_authenticated_connection(ws_stream, database, config).await {
                        error!("Error handling WebSocket connection: {}", e);
                    }
                }
                Err(e) => {
                    error!("Error during WebSocket handshake: {}", e);
                }
            }
        });
    }
    
    Ok(())
}


