use chrono::Utc;
// FRIENDSHIP SYSTEM
pub async fn send_friend_request(db: Arc<Database>, from_user_id: &str, to_username: &str, message: &str) -> String {
    // Trova l'id del destinatario
    let row = sqlx::query("SELECT id FROM users WHERE username = ?")
        .bind(to_username)
        .fetch_optional(&db.pool)
        .await;
    let to_user_id = match row {
        Ok(Some(r)) => r.get::<String,_>("id"),
        Ok(None) => return "ERR: Destinatario non trovato".to_string(),
        Err(e) => return format!("ERR: DB error: {}", e),
    };
    
    // Controlla se sono già amici
    let friendship_check = sqlx::query("SELECT 1 FROM friendships WHERE (user1_id = ? AND user2_id = ?) OR (user1_id = ? AND user2_id = ?)")
        .bind(from_user_id)
        .bind(&to_user_id)
        .bind(&to_user_id)
        .bind(from_user_id)
        .fetch_optional(&db.pool)
        .await;
    if let Ok(Some(_)) = friendship_check {
        return "ERR: Siete già amici".to_string();
    }
    
    
    // Controlla se già esiste una richiesta pendente
    let check = sqlx::query("SELECT id FROM friend_requests WHERE from_user_id = ? AND to_user_id = ? AND status = 'pending'")
        .bind(from_user_id)
        .bind(&to_user_id)
        .fetch_optional(&db.pool)
        .await;
    if let Ok(Some(_)) = check {
        return "ERR: Richiesta già inviata".to_string();
    }
    // Inserisci la richiesta
    let now = Utc::now().timestamp();
    let res = sqlx::query("INSERT INTO friend_requests (from_user_id, to_user_id, message, created_at, status) VALUES (?, ?, ?, ?, 'pending')")
        .bind(from_user_id)
        .bind(&to_user_id)
        .bind(message)
        .bind(now)
        .execute(&db.pool)
        .await;
    match res {
        Ok(_) => "OK: Richiesta inviata".to_string(),
        Err(e) => format!("ERR: DB error: {}", e),
    }
}

pub async fn accept_friend_request(db: Arc<Database>, to_user_id: &str, from_username: &str) -> String {
    // Trova l'id del mittente
    let row = sqlx::query("SELECT id FROM users WHERE username = ?")
        .bind(from_username)
        .fetch_optional(&db.pool)
        .await;
    let from_user_id = match row {
        Ok(Some(r)) => r.get::<String,_>("id"),
        Ok(None) => return "ERR: Mittente non trovato".to_string(),
        Err(e) => return format!("ERR: DB error: {}", e),
    };
    // Aggiorna la richiesta
    let res = sqlx::query("UPDATE friend_requests SET status = 'accepted' WHERE from_user_id = ? AND to_user_id = ? AND status = 'pending'")
        .bind(&from_user_id)
        .bind(to_user_id)
        .execute(&db.pool)
        .await;
    if let Err(e) = res {
        return format!("ERR: DB error: {}", e);
    }
    // Crea la friendship
    let now = Utc::now().timestamp();
    let res2 = sqlx::query("INSERT OR IGNORE INTO friendships (user1_id, user2_id, created_at) VALUES (?, ?, ?)")
        .bind(&from_user_id)
        .bind(to_user_id)
        .bind(now)
        .execute(&db.pool)
        .await;
    match res2 {
        Ok(_) => "OK: Amicizia accettata".to_string(),
        Err(e) => format!("ERR: DB error: {}", e),
    }
}

pub async fn reject_friend_request(db: Arc<Database>, to_user_id: &str, from_username: &str) -> String {
    // Trova l'id del mittente
    let row = sqlx::query("SELECT id FROM users WHERE username = ?")
        .bind(from_username)
        .fetch_optional(&db.pool)
        .await;
    let from_user_id = match row {
        Ok(Some(r)) => r.get::<String,_>("id"),
        Ok(None) => return "ERR: Mittente non trovato".to_string(),
        Err(e) => return format!("ERR: DB error: {}", e),
    };
    // Aggiorna la richiesta
    let res = sqlx::query("UPDATE friend_requests SET status = 'rejected' WHERE from_user_id = ? AND to_user_id = ? AND status = 'pending'")
        .bind(&from_user_id)
        .bind(to_user_id)
        .execute(&db.pool)
        .await;
    match res {
        Ok(_) => "OK: Richiesta rifiutata".to_string(),
        Err(e) => format!("ERR: DB error: {}", e),
    }
}

pub async fn list_friends(db: Arc<Database>, user_id: &str) -> String {
    let rows = sqlx::query("SELECT u.username FROM friendships f JOIN users u ON (u.id = f.user1_id OR u.id = f.user2_id) WHERE (f.user1_id = ? OR f.user2_id = ?) AND u.id != ?")
        .bind(user_id)
        .bind(user_id)
        .bind(user_id)
        .fetch_all(&db.pool)
        .await;
    match rows {
        Ok(rows) => {
            let friends: Vec<String> = rows.iter().map(|r| r.get::<String,_>("username")).collect();
            format!("OK: Friends: {}", friends.join(", "))
        }
        Err(e) => format!("ERR: DB error: {}", e),
    }
}

pub async fn received_friend_requests(db: Arc<Database>, user_id: &str) -> String {
    let rows = sqlx::query("SELECT u.username, fr.message FROM friend_requests fr JOIN users u ON fr.from_user_id = u.id WHERE fr.to_user_id = ? AND fr.status = 'pending'")
        .bind(user_id)
        .fetch_all(&db.pool)
        .await;
    match rows {
        Ok(rows) => {
            let reqs: Vec<String> = rows.iter().map(|r| format!("{}: {}", r.get::<String,_>("username"), r.get::<String,_>("message"))).collect();
            format!("OK: Richieste ricevute: {}", reqs.join(" | "))
        }
        Err(e) => format!("ERR: DB error: {}", e),
    }
}

pub async fn sent_friend_requests(db: Arc<Database>, user_id: &str) -> String {
    let rows = sqlx::query("SELECT u.username, fr.message FROM friend_requests fr JOIN users u ON fr.to_user_id = u.id WHERE fr.from_user_id = ? AND fr.status = 'pending'")
        .bind(user_id)
        .fetch_all(&db.pool)
        .await;
    match rows {
        Ok(rows) => {
            let reqs: Vec<String> = rows.iter().map(|r| format!("{}: {}", r.get::<String,_>("username"), r.get::<String,_>("message"))).collect();
            format!("OK: Richieste inviate: {}", reqs.join(" | "))
        }
        Err(e) => format!("ERR: DB error: {}", e),
    }
}

// HELP
pub async fn help() -> String {
    let help = "Comandi disponibili:\n\
    /register <username> <password>\n\
    /login <username> <password>\n\
    /logout\n\
    /users\n\
    /all_users\n\
    /send_friend_request <username> [message]\n\
    /accept_friend_request <username>\n\
    /reject_friend_request <username>\n\
    /list_friends\n\
    /received_friend_requests\n\
    /sent_friend_requests\n\
    /help\n\
    /quit\n";
    help.to_string()
}
use crate::server::database::Database;
use std::sync::Arc;
use sqlx::Row;

pub async fn list_online(db: Arc<Database>) -> String {
    println!("[USERS] Listing online users");
    let rows = sqlx::query("SELECT username FROM users WHERE is_online = 1")
        .fetch_all(&db.pool)
        .await;
    match rows {
        Ok(rows) => {
            let users: Vec<String> = rows.iter().map(|r| r.get::<String,_>("username")).collect();
            format!("OK: Online users: {}", users.join(", "))
        }
        Err(e) => {
            println!("[USERS] Error listing online users: {}", e);
            format!("ERR: {}", e)
        }
    }
}

pub async fn list_online_excluding_self(db: Arc<Database>, session_token: &str) -> String {
    println!("[USERS] Listing online users excluding current user");
    
    // First validate session and get current user ID
    let current_user_id = match crate::server::auth::validate_session(db.clone(), session_token).await {
        Some(uid) => uid,
        None => return "ERR: Invalid or expired session".to_string(),
    };
    
    // Get current user's username
    let current_username = match sqlx::query("SELECT username FROM users WHERE id = ?")
        .bind(&current_user_id)
        .fetch_optional(&db.pool)
        .await
    {
        Ok(Some(row)) => row.get::<String,_>("username"),
        Ok(None) => return "ERR: User not found".to_string(),
        Err(e) => return format!("ERR: Database error: {}", e),
    };
    
    // Get all online users except current user
    let rows = sqlx::query("SELECT username FROM users WHERE is_online = 1 AND id != ?")
        .bind(&current_user_id)
        .fetch_all(&db.pool)
        .await;
    
    match rows {
        Ok(rows) => {
            let users: Vec<String> = rows.iter().map(|r| r.get::<String,_>("username")).collect();
            println!("[USERS] Found {} online users excluding {}", users.len(), current_username);
            format!("OK: Online users: {}", users.join(", "))
        }
        Err(e) => {
            println!("[USERS] Error listing online users: {}", e);
            format!("ERR: {}", e)
        }
    }
}

pub async fn list_all(db: Arc<Database>, exclude_username: Option<&str>) -> String {
    println!("[USERS] Listing all users");
    let rows = sqlx::query("SELECT username FROM users")
        .fetch_all(&db.pool)
        .await;
    match rows {
        Ok(rows) => {
            let mut users: Vec<String> = rows.iter().map(|r| r.get::<String,_>("username")).collect();
            if let Some(exclude) = exclude_username {
                users.retain(|u| u != exclude);
            }
            format!("OK: All users: {}", users.join(", "))
        }
        Err(e) => {
            println!("[USERS] Error listing all users: {}", e);
            format!("ERR: {}", e)
        }
    }
}
