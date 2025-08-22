use std::env;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub max_clients: usize,
    pub enable_encryption: bool,
    pub log_level: String,
    pub session_expiry_days: u32,
    pub argon2_salt_length: u32,
    pub max_message_length: usize,
}

impl ServerConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();
        Self {
            host: env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("SERVER_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(5000),
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data/ruggine_modulare.db".to_string()),
            max_clients: env::var("MAX_CLIENTS").ok().and_then(|v| v.parse().ok()).unwrap_or(100),
            enable_encryption: env::var("ENABLE_ENCRYPTION").map(|v| v == "true" || v == "1").unwrap_or(true),
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
            session_expiry_days: env::var("SESSION_EXPIRY_DAYS").ok().and_then(|v| v.parse().ok()).unwrap_or(7),
            argon2_salt_length: env::var("ARGON2_SALT_LENGTH").ok().and_then(|v| v.parse().ok()).unwrap_or(16),
            max_message_length: env::var("MAX_MESSAGE_LENGTH").ok().and_then(|v| v.parse().ok()).unwrap_or(2048),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub default_host: String,
    pub default_port: u16,
    pub public_host: String,
}

impl ClientConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();
        Self {
            default_host: env::var("CLIENT_DEFAULT_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            default_port: env::var("CLIENT_DEFAULT_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(5000),
            public_host: env::var("CLIENT_PUBLIC_HOST").unwrap_or_else(|_| "remote.example.com".to_string()),
        }
    }
}
