use crate::server::database::Database;
use crate::server::config::ServerConfig;
use std::sync::Arc;
use sqlx::Row;
use argon2::{Argon2, password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString}};
use rand::RngCore;


/// Logout: elimina la sessione e imposta utente offline
pub async fn logout(db: Arc<Database>, session_token: &str) -> String {
    // Trova user_id dalla sessione
    println!("[AUTH] logout called (token masked)");
    let row = sqlx::query("SELECT user_id FROM sessions WHERE session_token = ?")
        .bind(session_token)
        .fetch_optional(&db.pool)
        .await;
    match row {
        Ok(Some(row)) => {
            let user_id: String = row.get("user_id");
            // Invalidate all sessions for this user (logout from all devices) to enforce single-session semantics
            match sqlx::query("DELETE FROM sessions WHERE user_id = ?")
                .bind(&user_id)
                .execute(&db.pool)
                .await
            {
                Ok(r) => println!("[AUTH] Deleted {} session rows for user {}", r.rows_affected(), user_id),
                Err(e) => println!("[AUTH] Failed deleting sessions for {}: {}", user_id, e),
            }

            // Force user offline
            match sqlx::query("UPDATE users SET is_online = 0 WHERE id = ?")
                .bind(&user_id)
                .execute(&db.pool)
                .await
            {
                Ok(_) => println!("[AUTH] Set is_online=0 for user {} due to logout", user_id),
                Err(e) => println!("[AUTH] Failed to set is_online=0 for {}: {}", user_id, e),
            }

            // Verify state after logout
            let sess_cnt = sqlx::query("SELECT COUNT(1) as c FROM sessions WHERE user_id = ?")
                .bind(&user_id)
                .fetch_one(&db.pool)
                .await
                .ok()
                .and_then(|r| r.try_get::<i64, _>("c").ok())
                .unwrap_or(-1);
            let is_online = sqlx::query("SELECT is_online FROM users WHERE id = ?")
                .bind(&user_id)
                .fetch_optional(&db.pool)
                .await
                .ok()
                .and_then(|opt| opt.map(|r| r.get::<i64, _>("is_online")))
                .unwrap_or(-1);
            println!("[AUTH][DB CHECK] logout completed: sessions_count={} users.is_online={} for user {}", sess_cnt, is_online, user_id);

            // record logout event
            let now = chrono::Utc::now().timestamp();
            match sqlx::query("INSERT INTO session_events (user_id, event_type, created_at) VALUES (?, ?, ?)")
                .bind(&user_id)
                .bind("logout")
                .bind(now)
                .execute(&db.pool)
                .await
            {
                Ok(_) => println!("[AUTH] Recorded logout event for {}", user_id),
                Err(e) => println!("[AUTH] Failed to record logout event for {}: {}", user_id, e),
            }

            println!("[AUTH] Logout success for user_id={}", user_id);
            "OK: Logout effettuato".to_string()
        }
        Ok(None) => {
            println!("[AUTH] Logout fallito: sessione non trovata");
            "ERR: Sessione non trovata".to_string()
        }
        Err(e) => {
            println!("[AUTH] Logout fallito: {}", e);
            format!("ERR: Logout fallito: {}", e)
        }
    }
}

fn hash_password(password: &str, salt_length: u32) -> String {
    // Genera un salt casuale della lunghezza specificata
    let mut salt_bytes = vec![0u8; salt_length as usize];
    rand::thread_rng().fill_bytes(&mut salt_bytes);
    let salt = SaltString::encode_b64(&salt_bytes).unwrap();
    let argon2 = Argon2::default();
    argon2.hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string()
}

fn verify_password(hash: &str, password: &str) -> bool {
    // Il salt è incluso nell'hash, quindi la verifica non cambia
    let parsed_hash = PasswordHash::new(hash).unwrap();
    Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok()
}

fn generate_session_token() -> String {
    let uuid = uuid::Uuid::new_v4().to_string();
    let mut random = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut random);
    format!("{}-{:x}", uuid, md5::compute(random))
}

pub async fn register(db: Arc<Database>, username: &str, password: &str, config: &ServerConfig) -> String {
    println!("[AUTH] Register attempt: {}", username);
    let user_id = uuid::Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().timestamp();
    let password_hash = hash_password(password, config.argon2_salt_length);
    let tx = db.pool.begin().await;
    match tx {
        Ok(mut tx) => {
            let res = sqlx::query("INSERT INTO users (id, username, created_at, is_online) VALUES (?, ?, ?, 0)")
                .bind(&user_id)
                .bind(username)
                .bind(created_at)
                .execute(&mut *tx)
                .await;
            if let Err(e) = res {
                // Detect common UNIQUE constraint failure (username already exists) and return a friendlier message
                let err_str = e.to_string();
                println!("[AUTH] Registration failed for {}: {}", username, err_str);
                if err_str.to_lowercase().contains("UNIQUE") || err_str.to_lowercase().contains("constraint failed") {
                    return "ERR: Username already used".to_string();
                }
                return "ERR: Registration failed".to_string();
            }
            sqlx::query("INSERT INTO user_encryption_keys (user_id, public_key, private_key) VALUES (?, '', '')")
                .bind(&user_id)
                .execute(&mut *tx)
                .await
                .ok();
            sqlx::query("INSERT INTO auth (user_id, password_hash) VALUES (?, ?)")
                .bind(&user_id)
                .bind(&password_hash)
                .execute(&mut *tx)
                .await
                .ok();
            // Imposta utente online e crea sessione subito dopo la registrazione
            let _ = sqlx::query("UPDATE users SET is_online = 1 WHERE id = ?")
                .bind(&user_id)
                .execute(&mut *tx)
                .await;
            println!("[AUTH] Set is_online=1 for new user {}", user_id);
            // Crea sessione come nel login
            let session_token = generate_session_token();
            let now = chrono::Utc::now().timestamp();
            let expires = now + 60*60*24*config.session_expiry_days as i64;
            sqlx::query("INSERT INTO sessions (user_id, session_token, created_at, expires_at) VALUES (?, ?, ?, ?)")
                .bind(&user_id)
                .bind(&session_token)
                .bind(now)
                .bind(expires)
                .execute(&mut *tx)
                .await
                .ok();
            println!("[AUTH] Created initial session for user {} token={}", user_id, session_token);
            tx.commit().await.ok();
            println!("[AUTH] Registered user {} (id={})", username, user_id);
            format!("OK: Registered as {} SESSION: {}", username, session_token)
        }
        Err(e) => {
            println!("[AUTH] Registration failed for {}: {}", username, e);
            format!("ERR: Registration failed: {}", e)
        }
    }
}

pub async fn login(db: Arc<Database>, username: &str, password: &str, config: &ServerConfig) -> String {
    println!("[AUTH] Login attempt: {}", username);
    let row = sqlx::query("SELECT users.id, password_hash FROM users JOIN auth ON users.id = auth.user_id WHERE username = ?")
        .bind(username)
        .fetch_optional(&db.pool)
        .await;
    match row {
        Ok(Some(row)) => {
            let user_id: String = row.get("id");
            let password_hash: String = row.get("password_hash");
            if verify_password(&password_hash, password) {
                // Begin transaction to ensure atomic single-session semantics
                match db.pool.begin().await {
                    Ok(mut tx) => {
                        // Remove any existing sessions for this user
                        match sqlx::query("DELETE FROM sessions WHERE user_id = ?")
                            .bind(&user_id)
                            .execute(&mut *tx)
                            .await
                        {
                            Ok(r) => println!("[AUTH] Deleted {} old sessions for user {} during login", r.rows_affected(), user_id),
                            Err(e) => println!("[AUTH] Failed deleting old sessions for {}: {}", user_id, e),
                        }

                        // Set user online
                        match sqlx::query("UPDATE users SET is_online = 1 WHERE id = ?")
                            .bind(&user_id)
                            .execute(&mut *tx)
                            .await
                        {
                            Ok(_) => println!("[AUTH] Set is_online=1 for user {} (transaction)", user_id),
                            Err(e) => println!("[AUTH] Failed to set is_online for {}: {}", user_id, e),
                        }

                        // Create new session token
                        let session_token = generate_session_token();
                        let now = chrono::Utc::now().timestamp();
                        let expires = now + 60*60*24*config.session_expiry_days as i64;
                        match sqlx::query("INSERT INTO sessions (user_id, session_token, created_at, expires_at) VALUES (?, ?, ?, ?)")
                            .bind(&user_id)
                            .bind(&session_token)
                            .bind(now)
                            .bind(expires)
                            .execute(&mut *tx)
                            .await
                        {
                            Ok(_) => println!("[AUTH] Inserted new session for user {} token={}", user_id, session_token),
                            Err(e) => println!("[AUTH] Failed inserting session for {}: {}", user_id, e),
                        }

                        // Record login event
                        let _ = sqlx::query("INSERT INTO session_events (user_id, event_type, created_at) VALUES (?, ?, ?)")
                            .bind(&user_id)
                            .bind("login_success")
                            .bind(now)
                            .execute(&mut *tx)
                            .await;

                        // Commit
                        if let Err(e) = tx.commit().await {
                            println!("[AUTH] Failed to commit login transaction for {}: {}", user_id, e);
                            return format!("ERR: Login failed: {}", e);
                        }

                        println!("[AUTH] Login success for {} (id={})", username, user_id);
                        format!("OK: Logged in as {} SESSION: {}", username, session_token)
                    }
                    Err(e) => {
                        println!("[AUTH] Failed to start transaction for login {}: {}", username, e);
                        format!("ERR: Login failed: {}", e)
                    }
                }
            } else {
                println!("[AUTH] Login failed for {}: wrong password", username);
                "ERR: Wrong password".to_string()
            }
        }
        Ok(None) => {
            println!("[AUTH] Login failed for {}: user not found", username);
            "ERR: User not found".to_string()
        }
        Err(e) => {
            println!("[AUTH] Login failed for {}: {}", username, e);
            format!("ERR: Login failed: {}", e)
        }
    }
}

pub async fn validate_session(db: Arc<Database>, session_token: &str) -> Option<String> {
    let now = chrono::Utc::now().timestamp();
    let row = sqlx::query("SELECT user_id FROM sessions WHERE session_token = ? AND expires_at > ?")
        .bind(session_token)
        .bind(now)
        .fetch_optional(&db.pool)
        .await
        .ok()?;
    
    if let Some(row) = row {
        let user_id: String = row.get("user_id");
        println!("[AUTH] validate_session: token {} is valid for user {}", session_token, user_id);
        
        // Set user online when session is validated (for auto-login scenarios)
        let _ = sqlx::query("UPDATE users SET is_online = 1 WHERE id = ?")
            .bind(&user_id)
            .execute(&db.pool)
            .await;
        println!("[AUTH] Set is_online=1 for user {} due to session validation", user_id);
        
        Some(user_id)
    } else {
        println!("[AUTH] validate_session: token {} is invalid or expired", session_token);
        None
    }
}

/// Rimuove le sessioni scadute dal DB. Idempotente e sicuro da eseguire periodicamente.
pub async fn cleanup_expired_sessions(db: Arc<Database>) {
    let now = chrono::Utc::now().timestamp();
    match sqlx::query("DELETE FROM sessions WHERE expires_at <= ?")
        .bind(now)
        .execute(&db.pool)
        .await
    {
        Ok(res) => println!("[AUTH] Cleaned up {} expired sessions", res.rows_affected()),
        Err(e) => println!("[AUTH] Failed to cleanup sessions: {}", e),
    }
}
