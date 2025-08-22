use crate::server::{database::Database, auth};
use std::sync::Arc;
use sqlx::Row;

use crate::server::config::ServerConfig;

pub async fn send_group_message(db: Arc<Database>, session_token: &str, group_name: &str, message: &str, config: &ServerConfig) -> String {
    if message.len() > config.max_message_length {
        return format!("ERR: Message too long (max {} chars)", config.max_message_length);
    }
    let user_id = match auth::validate_session(db.clone(), session_token).await {
        Some(uid) => uid,
        None => return "ERR: Invalid session".to_string(),
    };
    let group_row = sqlx::query("SELECT id FROM groups WHERE name = ?")
        .bind(group_name)
        .fetch_optional(&db.pool)
        .await;
    let group_id = match group_row {
        Ok(Some(row)) => row.get::<String,_>("id"),
        _ => return "ERR: Group not found".to_string(),
    };
    let is_member = sqlx::query("SELECT 1 FROM group_members WHERE group_id = ? AND user_id = ?")
        .bind(&group_id)
        .bind(&user_id)
        .fetch_optional(&db.pool)
        .await
        .ok()
        .flatten()
        .is_some();
    if !is_member {
        return "ERR: Not a group member".to_string();
    }
    let sent_at = chrono::Utc::now().timestamp();
    let chat_id = format!("group:{}", group_id);
    let res = sqlx::query("INSERT INTO encrypted_messages (chat_id, sender_id, message, sent_at) VALUES (?, ?, ?, ?)")
        .bind(&chat_id)
        .bind(&user_id)
        .bind(message)
        .bind(sent_at)
        .execute(&db.pool)
        .await;
    match res {
        Ok(_) => {
            println!("[MSG] Group message sent to {} by {}", group_name, user_id);
            "OK: Message sent".to_string()
        }
        Err(e) => {
            println!("[MSG] Error sending group message: {}", e);
            format!("ERR: {}", e)
        }
    }
}

pub async fn send_private_message(db: Arc<Database>, session_token: &str, to_username: &str, message: &str, config: &ServerConfig) -> String {
    if message.len() > config.max_message_length {
        return format!("ERR: Message too long (max {} chars)", config.max_message_length);
    }
    let user_id = match auth::validate_session(db.clone(), session_token).await {
        Some(uid) => uid,
        None => return "ERR: Invalid session".to_string(),
    };
    let to_row = sqlx::query("SELECT id FROM users WHERE username = ?")
        .bind(to_username)
        .fetch_optional(&db.pool)
        .await;
    let to_id = match to_row {
        Ok(Some(row)) => row.get::<String,_>("id"),
        _ => return "ERR: User not found".to_string(),
    };
    let mut ids = vec![user_id.clone(), to_id.clone()];
    ids.sort();
    let chat_id = format!("private:{}-{}", ids[0], ids[1]);
    let sent_at = chrono::Utc::now().timestamp();
    let res = sqlx::query("INSERT INTO encrypted_messages (chat_id, sender_id, message, sent_at) VALUES (?, ?, ?, ?)")
        .bind(&chat_id)
        .bind(&user_id)
        .bind(message)
        .bind(sent_at)
        .execute(&db.pool)
        .await;
    match res {
        Ok(_) => {
            println!("[MSG] Private message sent to {} by {}", to_username, user_id);
            "OK: Message sent".to_string()
        }
        Err(e) => {
            println!("[MSG] Error sending private message: {}", e);
            format!("ERR: {}", e)
        }
    }
}

pub async fn get_group_messages(db: Arc<Database>, session_token: &str, group_name: &str) -> String {
    let user_id = match auth::validate_session(db.clone(), session_token).await {
        Some(uid) => uid,
        None => return "ERR: Invalid session".to_string(),
    };
    let group_row = sqlx::query("SELECT id FROM groups WHERE name = ?")
        .bind(group_name)
        .fetch_optional(&db.pool)
        .await;
    let group_id = match group_row {
        Ok(Some(row)) => row.get::<String,_>("id"),
        _ => return "ERR: Group not found".to_string(),
    };
    let is_member = sqlx::query("SELECT 1 FROM group_members WHERE group_id = ? AND user_id = ?")
        .bind(&group_id)
        .bind(&user_id)
        .fetch_optional(&db.pool)
        .await
        .ok()
        .flatten()
        .is_some();
    if !is_member {
        return "ERR: Not a group member".to_string();
    }
    let chat_id = format!("group:{}", group_id);
    let rows = sqlx::query("SELECT sender_id, message, sent_at FROM encrypted_messages WHERE chat_id = ? ORDER BY sent_at ASC")
        .bind(&chat_id)
        .fetch_all(&db.pool)
        .await;
    match rows {
        Ok(rows) => {
            let msgs: Vec<String> = rows.iter().map(|r| {
                let sender: String = r.get("sender_id");
                let msg: String = r.get("message");
                let ts: i64 = r.get("sent_at");
                format!("[{}] {}: {}", ts, sender, msg)
            }).collect();
            format!("OK: Messages:\n{}", msgs.join("\n"))
        }
        Err(e) => {
            println!("[MSG] Error getting group messages: {}", e);
            format!("ERR: {}", e)
        }
    }
}

pub async fn get_private_messages(db: Arc<Database>, session_token: &str, other_username: &str) -> String {
    let user_id = match auth::validate_session(db.clone(), session_token).await {
        Some(uid) => uid,
        None => return "ERR: Invalid session".to_string(),
    };
    let to_row = sqlx::query("SELECT id FROM users WHERE username = ?")
        .bind(other_username)
        .fetch_optional(&db.pool)
        .await;
    let to_id = match to_row {
        Ok(Some(row)) => row.get::<String,_>("id"),
        _ => return "ERR: User not found".to_string(),
    };
    let mut ids = vec![user_id.clone(), to_id.clone()];
    ids.sort();
    let chat_id = format!("private:{}-{}", ids[0], ids[1]);
    let rows = sqlx::query("SELECT sender_id, message, sent_at FROM encrypted_messages WHERE chat_id = ? ORDER BY sent_at ASC")
        .bind(&chat_id)
        .fetch_all(&db.pool)
        .await;
    match rows {
        Ok(rows) => {
            let msgs: Vec<String> = rows.iter().map(|r| {
                let sender: String = r.get("sender_id");
                let msg: String = r.get("message");
                let ts: i64 = r.get("sent_at");
                format!("[{}] {}: {}", ts, sender, msg)
            }).collect();
            format!("OK: Messages:\n{}", msgs.join("\n"))
        }
        Err(e) => {
            println!("[MSG] Error getting private messages: {}", e);
            format!("ERR: {}", e)
        }
    }
}

pub async fn delete_group_messages(db: Arc<Database>, session_token: &str, group_id: &str) -> String {
    let user_id = match auth::validate_session(db.clone(), session_token).await {
        Some(uid) => uid,
        None => return "ERR: Invalid session".to_string(),
    };
    let is_member = sqlx::query("SELECT 1 FROM group_members WHERE group_id = ? AND user_id = ?")
        .bind(group_id)
        .bind(&user_id)
        .fetch_optional(&db.pool)
        .await
        .ok()
        .flatten()
        .is_some();
    if !is_member {
        return "ERR: Not a group member".to_string();
    }
    let chat_id = format!("group:{}", group_id);
    let res = sqlx::query("DELETE FROM encrypted_messages WHERE chat_id = ?")
        .bind(&chat_id)
        .execute(&db.pool)
        .await;
    match res {
        Ok(_) => {
            println!("[MSG] Deleted all messages in group {} by {}", group_id, user_id);
            "OK: Messages deleted".to_string()
        }
        Err(e) => {
            println!("[MSG] Error deleting group messages: {}", e);
            format!("ERR: {}", e)
        }
    }
}

pub async fn delete_private_messages(db: Arc<Database>, session_token: &str, other_username: &str) -> String {
    let user_id = match auth::validate_session(db.clone(), session_token).await {
        Some(uid) => uid,
        None => return "ERR: Invalid session".to_string(),
    };
    let to_row = sqlx::query("SELECT id FROM users WHERE username = ?")
        .bind(other_username)
        .fetch_optional(&db.pool)
        .await;
    let to_id = match to_row {
        Ok(Some(row)) => row.get::<String,_>("id"),
        _ => return "ERR: User not found".to_string(),
    };
    let mut ids = vec![user_id.clone(), to_id.clone()];
    ids.sort();
    let chat_id = format!("private:{}-{}", ids[0], ids[1]);
    let res = sqlx::query("DELETE FROM encrypted_messages WHERE chat_id = ?")
        .bind(&chat_id)
        .execute(&db.pool)
        .await;
    match res {
        Ok(_) => {
            println!("[MSG] Deleted all private messages with {} by {}", other_username, user_id);
            "OK: Messages deleted".to_string()
        }
        Err(e) => {
            println!("[MSG] Error deleting private messages: {}", e);
            format!("ERR: {}", e)
        }
    }
}
