use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

#[derive(Debug, Clone)]
pub struct Database {
    pub pool: SqlitePool,
}

impl Database {
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        println!("🔗 Attempting to connect to database: {}", database_url);
        
        // Extract file path from database URL to create directory if needed
        let file_path = if database_url.starts_with("sqlite://") {
            // Remove "sqlite://" prefix and any query parameters
            let path_part = &database_url[9..];
            if let Some(query_pos) = path_part.find('?') {
                &path_part[..query_pos]
            } else {
                path_part
            }
        } else if database_url.starts_with("sqlite:") {
            // Remove "sqlite:" prefix
            &database_url[7..]
        } else {
            database_url
        };
        
        println!("📁 Database file path: {}", file_path);
        
        if let Some(parent) = std::path::Path::new(file_path).parent() {
            println!("📂 Parent directory: {:?}", parent);
            if !parent.as_os_str().is_empty() && !parent.exists() {
                println!("📁 Directory does not exist, creating...");
                std::fs::create_dir_all(parent).map_err(|e| {
                    println!("❌ Failed to create directory: {}", e);
                    sqlx::Error::Configuration(Box::new(e))
                })?;
                println!("✅ Created directory: {:?}", parent);
            } else if parent.as_os_str().is_empty() {
                println!("📁 Using current directory");
            } else {
                println!("📁 Directory already exists: {:?}", parent);
            }
        }
        
        // Check if the file already exists
        if std::path::Path::new(file_path).exists() {
            println!("📄 Database file already exists");
        } else {
            println!("📄 Database file does not exist, SQLite will create it");
        }
        
        println!("🔗 Creating SQLite connection pool...");
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .map_err(|e| {
                println!("❌ SQLite connection failed: {}", e);
                e
            })?;
        
        println!("✅ Database connection successful!");
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<(), sqlx::Error> {
        // Users
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                username TEXT UNIQUE NOT NULL,
                created_at INTEGER NOT NULL,
                is_online INTEGER NOT NULL DEFAULT 0
            );
        "#).execute(&self.pool).await?;

        // User encryption keys
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS user_encryption_keys (
                user_id TEXT PRIMARY KEY,
                public_key TEXT NOT NULL,
                private_key TEXT NOT NULL
            );
        "#).execute(&self.pool).await?;

        // Group encryption keys
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS group_encryption_keys (
                group_id TEXT PRIMARY KEY,
                encryption_key TEXT NOT NULL
            );
        "#).execute(&self.pool).await?;

        // Deleted chats
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS deleted_chats (
                user_id TEXT NOT NULL,
                chat_id TEXT NOT NULL,
                deleted_at INTEGER NOT NULL,
                PRIMARY KEY (user_id, chat_id)
            );
        "#).execute(&self.pool).await?;

        // Encrypted messages
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS encrypted_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chat_id TEXT NOT NULL,
                sender_id TEXT NOT NULL,
                message TEXT NOT NULL,
                sent_at INTEGER NOT NULL
            );
        "#).execute(&self.pool).await?;

        // Friend requests
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS friend_requests (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                from_user_id TEXT NOT NULL,
                to_user_id TEXT NOT NULL,
                message TEXT,
                created_at INTEGER NOT NULL,
                status TEXT NOT NULL
            );
        "#).execute(&self.pool).await?;

        // Friendships
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS friendships (
                user1_id TEXT NOT NULL,
                user2_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (user1_id, user2_id)
            );
        "#).execute(&self.pool).await?;

        // Groups
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS groups (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_by TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );
        "#).execute(&self.pool).await?;

        // Group members
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS group_members (
                group_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                joined_at INTEGER NOT NULL,
                PRIMARY KEY (group_id, user_id)
            );
        "#).execute(&self.pool).await?;

        // Group invites
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS group_invites (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                group_id TEXT NOT NULL,
                invited_user_id TEXT NOT NULL,
                invited_by TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                status TEXT NOT NULL
            );
        "#).execute(&self.pool).await?;

        // Auth
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS auth (
                user_id TEXT PRIMARY KEY,
                password_hash TEXT NOT NULL
            );
        "#).execute(&self.pool).await?;

        // Sessions
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS sessions (
                user_id TEXT NOT NULL,
                session_token TEXT PRIMARY KEY,
                created_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL
            );
        "#).execute(&self.pool).await?;

        // Session events (login_success, logout, quit, kicked_out)
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS session_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );
        "#).execute(&self.pool).await?;

        Ok(())
    }
}
