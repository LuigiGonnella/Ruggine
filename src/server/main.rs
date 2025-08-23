// src/server/main.rs
// Entry point per il server ruggine_modulare
use ruggine_modulare::server::{connection::Server, config::ServerConfig, database::Database};
use clap::Parser;
use log::{info, error};
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "ruggine-server")]
#[command(about = "A chat server application")]
struct Args {
    #[arg(long, default_value = "0.0.0.0")]
    host: String,
    #[arg(short, long, default_value = "5000")]
    port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // load .env for development configuration
    let _ = dotenvy::dotenv();
    env_logger::init();
    let args = Args::parse();
    let config = ServerConfig::from_env();

    // Log DB URL for diagnostics
    info!("Using database URL: {}", config.database_url);

    // Ensure parent directory for sqlite file exists when using file-backed sqlite URL
    if let Some(mut db_path) = config.database_url.strip_prefix("sqlite:") {
        // normalize leading slashes (handles sqlite://path and sqlite:////absolute)
        while db_path.starts_with('/') {
            db_path = &db_path[1..];
        }

        // skip memory DBs like :memory:
        if !db_path.contains("memory") {
            use std::path::Path;
            let path = Path::new(db_path);
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        anyhow::anyhow!("Failed to create DB parent dir '{}': {}", parent.display(), e)
                    })?;
                }
            }

            // Try to create or open the file to surface permission errors early
            use std::fs::OpenOptions;
            match OpenOptions::new().create(true).append(true).open(path) {
                Ok(_) => info!("Ensured DB file exists or is creatable: {}", path.display()),
                Err(e) => {
                    error!("Failed to create/open DB file {}: {}", path.display(), e);
                    return Err(anyhow::anyhow!(e))
                }
            }
        }
    }
    info!("Starting ruggine_modulare server on {}:{}", args.host, args.port);
    let db = Arc::new(Database::connect(&config.database_url).await?);
    db.migrate().await?;
    let server = Server { db: db.clone(), config: config.clone() };

    // Spawn periodic cleanup of expired sessions (runs every hour)
    let cleaner_db = db.clone();
    tokio::spawn(async move {
        loop {
            ruggine_modulare::server::auth::cleanup_expired_sessions(cleaner_db.clone()).await;
            tokio::time::sleep(std::time::Duration::from_secs(60 * 60)).await;
        }
    });

    // TLS hint for the operator
    if config.enable_encryption {
        log::info!("TLS is enabled; set TLS_CERT_PATH and TLS_KEY_PATH env vars to point to cert and key PEM files.");
    } else {
        log::info!("TLS is disabled; connections will be plain TCP.");
    }

    server.run(&format!("{}:{}", args.host, args.port)).await?;
    Ok(())
}
