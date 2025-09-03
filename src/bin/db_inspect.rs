use ruggine_modulare::server::database::Database;
use sqlx::Row;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_path = "sqlite:data/ruggine_modulare.db";
    println!("Connecting to {}", db_path);
    let db = Database::connect(db_path).await?;

    println!("\n-- groups --");
    let rows = sqlx::query("SELECT id, name, created_by, created_at FROM groups")
        .fetch_all(&db.pool)
        .await?;
    for r in rows.iter() {
        let id: String = r.try_get("id").unwrap_or_default();
        let name: String = r.try_get("name").unwrap_or_default();
        let created_by: String = r.try_get("created_by").unwrap_or_default();
        let created_at: i64 = r.try_get("created_at").unwrap_or(0);
        println!("id={} name={} created_by={} created_at={}", id, name, created_by, created_at);
    }

    println!("\n-- group_members --");
    let rows = sqlx::query("SELECT group_id, user_id, joined_at FROM group_members")
        .fetch_all(&db.pool)
        .await?;
    for r in rows.iter() {
        let group_id: String = r.try_get("group_id").unwrap_or_default();
        let user_id: String = r.try_get("user_id").unwrap_or_default();
        let joined_at: i64 = r.try_get("joined_at").unwrap_or(0);
        println!("group_id={} user_id={} joined_at={}", group_id, user_id, joined_at);
    }

    println!("\n-- encrypted_messages (last 10) --");
    let rows = sqlx::query("SELECT id, chat_id, sender_id, message, sent_at FROM encrypted_messages ORDER BY sent_at DESC LIMIT 10")
        .fetch_all(&db.pool)
        .await?;
    for r in rows.iter() {
        let id: i64 = r.try_get("id").unwrap_or(0);
        let chat_id: String = r.try_get("chat_id").unwrap_or_default();
        let sender_id: String = r.try_get("sender_id").unwrap_or_default();
        let message: String = r.try_get("message").unwrap_or_default();
        let sent_at: i64 = r.try_get("sent_at").unwrap_or(0);
        println!("id={} chat_id={} sender_id={} message_len={} sent_at={}", 
                 id, chat_id, sender_id, message.len(), sent_at);
    }

    Ok(())
}
