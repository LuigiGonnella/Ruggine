use crate::server::{database::Database, auth};
use std::sync::Arc;
use sqlx::Row;
use base64::{Engine as _, engine::general_purpose};
use serde_json;

use crate::server::config::ServerConfig;
use crate::common::crypto::CryptoManager;

/// Encrypts a message for storage in the database
fn encrypt_message_for_storage(message: &str, chat_participants: &[String], config: &ServerConfig) -> Result<String, String> {
    if !config.enable_encryption {
        return Ok(message.to_string());
    }
    
    println!("[CRYPTO] Encrypting message for participants: {:?}", chat_participants);
    
    // Generate chat-specific key from participants and master key
    let chat_key = CryptoManager::generate_chat_key(chat_participants, &config.encryption_master_key);
    
    // Encrypt the message
    match CryptoManager::encrypt_message(message, &chat_key) {
        Ok((ciphertext, nonce)) => {
            // Store as base64 encoded JSON containing ciphertext and nonce
            let encrypted_data = serde_json::json!({
                "ciphertext": general_purpose::STANDARD.encode(&ciphertext),
                "nonce": general_purpose::STANDARD.encode(&nonce)
            });
            println!("[CRYPTO] Successfully encrypted message");
            Ok(encrypted_data.to_string())
        }
        Err(_) => Err("Encryption failed".to_string())
    }
}

/// Decrypts a message from the database
fn decrypt_message_from_storage(encrypted_data: &str, chat_participants: &[String], config: &ServerConfig) -> Result<String, String> {
    if !config.enable_encryption {
        return Ok(encrypted_data.to_string());
    }
    
    // Check if the message is already in encrypted format (JSON with ciphertext and nonce)
    // If it's not JSON, it's probably a legacy plain text message
    if let Ok(data) = serde_json::from_str::<serde_json::Value>(encrypted_data) {
        // This is an encrypted message
        println!("[CRYPTO] Decrypting message for participants: {:?}", chat_participants);
        let ciphertext = general_purpose::STANDARD.decode(data["ciphertext"].as_str().ok_or("Missing ciphertext")?).map_err(|_| "Invalid ciphertext base64")?;
        let nonce = general_purpose::STANDARD.decode(data["nonce"].as_str().ok_or("Missing nonce")?).map_err(|_| "Invalid nonce base64")?;
        
        // Generate chat-specific key from participants and master key
        let chat_key = CryptoManager::generate_chat_key(chat_participants, &config.encryption_master_key);
        
        // Decrypt the message
        match CryptoManager::decrypt_message(&ciphertext, &nonce, &chat_key) {
            Ok(decrypted) => {
                println!("[CRYPTO] Successfully decrypted message");
                Ok(decrypted)
            }
            Err(e) => {
                println!("[CRYPTO] Decryption failed: {:?}", e);
                Err("Decryption failed".to_string())
            }
        }
    } else {
        // This is a legacy plain text message - return as is
        println!("[MSG] Legacy plain text message detected, returning as-is");
        Ok(encrypted_data.to_string())
    }
}

pub async fn send_group_message(db: Arc<Database>, session_token: &str, group_name: &str, message: &str, config: &ServerConfig) -> String {
    if message.len() > config.max_message_length {
        return format!("ERR: Message too long (max {} chars)", config.max_message_length);
    }
    let user_id = match auth::validate_session(db.clone(), session_token).await {
        Some(uid) => uid,
        None => return "ERR: Invalid session".to_string(),
    };
    // group_name is actually group_id in this context
    let group_row = sqlx::query("SELECT id FROM groups WHERE id = ?")
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
    
    // Get all group members for encryption key generation
    let members_rows = sqlx::query("SELECT user_id FROM group_members WHERE group_id = ?")
        .bind(&group_id)
        .fetch_all(&db.pool)
        .await;
    let group_members = match members_rows {
        Ok(rows) => rows.iter().map(|r| r.get::<String, _>("user_id")).collect::<Vec<String>>(),
        Err(e) => {
            println!("[MSG] Error getting group members: {}", e);
            return format!("ERR: Failed to get group members");
        }
    };
    
    // Encrypt the message before storing
    let encrypted_message = match encrypt_message_for_storage(message, &group_members, config) {
        Ok(encrypted) => encrypted,
        Err(e) => return format!("ERR: Encryption failed: {}", e),
    };
    
    let sent_at = chrono::Utc::now().timestamp();
    let chat_id = format!("group:{}", group_id);
    let res = sqlx::query("INSERT INTO encrypted_messages (chat_id, sender_id, message, sent_at) VALUES (?, ?, ?, ?)")
        .bind(&chat_id)
        .bind(&user_id)
        .bind(&encrypted_message)
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
    
    // Encrypt the message before storing
    let encrypted_message = match encrypt_message_for_storage(message, &ids, config) {
        Ok(encrypted) => encrypted,
        Err(e) => return format!("ERR: Encryption failed: {}", e),
    };
    
    let sent_at = chrono::Utc::now().timestamp();
    let res = sqlx::query("INSERT INTO encrypted_messages (chat_id, sender_id, message, sent_at) VALUES (?, ?, ?, ?)")
        .bind(&chat_id)
        .bind(&user_id)
        .bind(&encrypted_message)
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

pub async fn get_group_messages(db: Arc<Database>, session_token: &str, group_name: &str, config: &ServerConfig) -> String {
    let user_id = match auth::validate_session(db.clone(), session_token).await {
        Some(uid) => uid,
        None => return "ERR: Invalid session".to_string(),
    };
    // group_name is actually group_id in this context
    let group_row = sqlx::query("SELECT id FROM groups WHERE id = ?")
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
            // For group messages we need the list of participants to derive the chat key
            let members_rows = sqlx::query("SELECT user_id FROM group_members WHERE group_id = ?")
                .bind(&group_id)
                .fetch_all(&db.pool)
                .await;
            let group_members: Vec<String> = match members_rows {
                Ok(rows) => rows.iter().map(|r| r.get::<String, _>("user_id")).collect::<Vec<String>>(),
                Err(_) => vec![],
            };

            let mut msgs: Vec<String> = Vec::with_capacity(rows.len());
            for r in rows.iter() {
                let sender_id: String = r.get("sender_id");
                // Per i gruppi, converti sender_id in username
                let sender_name = if let Ok(Some(user_row)) = sqlx::query("SELECT username FROM users WHERE id = ?")
                    .bind(&sender_id)
                    .fetch_optional(&db.pool)
                    .await
                {
                    user_row.get::<String, _>("username")
                } else {
                    sender_id.clone() // fallback to ID if username not found
                };
                let msg: String = r.get("message");
                let ts: i64 = r.get("sent_at");
                // Attempt to decrypt; if decryption fails, show placeholder
                let clear = match decrypt_message_from_storage(&msg, &group_members, config) {
                    Ok(s) => s,
                    Err(_) => "[DECRYPTION FAILED]".to_string(),
                };
                msgs.push(format!("[{}] {}: {}", ts, sender_name, clear));
            }
            format!("OK: Messages:\n{}", msgs.join("\n"))
        }
        Err(e) => {
            println!("[MSG] Error getting group messages: {}", e);
            format!("ERR: {}", e)
        }
    }
}

pub async fn get_private_messages(db: Arc<Database>, session_token: &str, other_username: &str, config: &ServerConfig) -> String {
    let user_id = match auth::validate_session(db.clone(), session_token).await {
        Some(uid) => uid,
        None => return "ERR: Invalid session".to_string(),
    };
    
    // Ottieni anche il nostro username per i messaggi
    let my_username = match sqlx::query("SELECT username FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_optional(&db.pool)
        .await
    {
        Ok(Some(row)) => row.get::<String,_>("username"),
        _ => "Unknown".to_string(),
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
            let msgs: Vec<String> = rows.iter().filter_map(|r| {
                let sender: String = r.get("sender_id");
                // Converti sender_id in username
                let sender_name = if sender == user_id {
                    my_username.clone()
                } else {
                    other_username.to_string()
                };
                let msg: String = r.get("message");
                let ts: i64 = r.get("sent_at");
                // For private chats the participants are the two user ids we already computed in `ids`
                let clear = match decrypt_message_from_storage(&msg, &ids, config) {
                    Ok(s) => s,
                    Err(_) => msg.clone(),
                };
                format!("[{}] {}: {}", ts, sender_name, clear).into()
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
