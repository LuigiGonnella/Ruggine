use ruggine_modulare::server::database::Database;
use sqlx::Row;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // default path to the project's DB
    let db_path = "sqlite:data/ruggine_modulare.db";
    println!("Connecting to {}", db_path);
    let db = Database::connect(db_path).await?;

    println!("\n-- users --");
    let rows = sqlx::query("SELECT id, username, is_online FROM users")
        .fetch_all(&db.pool)
        .await?;
    for r in rows.iter() {
        let id: String = r.try_get("id").unwrap_or_default();
        let username: Option<String> = r.try_get("username").ok();
        let is_online: i64 = r.try_get("is_online").unwrap_or(0);
        println!("id={} username={:?} is_online={}", id, username, is_online);
    }

    println!("\n-- auth --");
    let rows = sqlx::query("SELECT user_id, password_hash FROM auth")
        .fetch_all(&db.pool)
        .await?;
    for r in rows.iter() {
        let user_id: String = r.try_get("user_id").unwrap_or_default();
        let password_hash: Option<String> = r.try_get("password_hash").ok();
        let len = password_hash.as_ref().map(|s| s.len()).unwrap_or(0);
        println!("user_id={} password_hash=(len={})", user_id, len);
    }

    println!("\n-- sessions --");
    let rows = sqlx::query("SELECT user_id, session_token, created_at, expires_at FROM sessions")
        .fetch_all(&db.pool)
        .await?;
    for r in rows.iter() {
        let user_id: String = r.try_get("user_id").unwrap_or_default();
        let token: Option<String> = r.try_get("session_token").ok();
        let created_at: i64 = r.try_get("created_at").unwrap_or(0);
        let expires_at: i64 = r.try_get("expires_at").unwrap_or(0);
        println!("user_id={} token={:?} created_at={} expires_at={}", user_id, token, created_at, expires_at);
    }

    println!("\n-- session_events (last 20) --");
    let rows = sqlx::query("SELECT id, user_id, event_type, created_at FROM session_events ORDER BY id DESC LIMIT 20")
        .fetch_all(&db.pool)
        .await?;
    for r in rows.iter() {
        let id: i64 = r.try_get("id").unwrap_or(0);
        let user_id: String = r.try_get("user_id").unwrap_or_default();
        let event_type: String = r.try_get("event_type").unwrap_or_default();
        let created_at: i64 = r.try_get("created_at").unwrap_or(0);
        println!("id={} user_id={} event_type={} created_at={}", id, user_id, event_type, created_at);
    }

    Ok(())
}
