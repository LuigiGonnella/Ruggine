use crate::server::database::Database;
use std::sync::Arc;
use sqlx::Row;

pub async fn create_group(db: Arc<Database>, user_id: &str, group_name: &str) -> String {
    println!("[GROUPS] Create group '{}' by user {}", group_name, user_id);
    let group_id = uuid::Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().timestamp();
    let tx = db.pool.begin().await;
    match tx {
        Ok(mut tx) => {
            let res = sqlx::query("INSERT INTO groups (id, name, created_by, created_at) VALUES (?, ?, ?, ?)")
                .bind(&group_id)
                .bind(group_name)
                .bind(user_id)
                .bind(created_at)
                .execute(&mut *tx)
                .await;
            if let Err(e) = res {
                println!("[GROUPS] Error creating group: {}", e);
                return format!("ERR: Could not create group: {}", e);
            }
            let res2 = sqlx::query("INSERT INTO group_members (group_id, user_id, joined_at) VALUES (?, ?, ?)")
                .bind(&group_id)
                .bind(user_id)
                .bind(created_at)
                .execute(&mut *tx)
                .await;
            if let Err(e) = res2 {
                println!("[GROUPS] Error adding creator as member: {}", e);
                return format!("ERR: Could not add creator as member: {}", e);
            }
            tx.commit().await.ok();
            println!("[GROUPS] Group '{}' created with id {}", group_name, group_id);
            format!("OK: Group '{}' created", group_name)
        }
        Err(e) => {
            println!("[GROUPS] Error starting transaction: {}", e);
            format!("ERR: Could not create group: {}", e)
        }
    }
}

pub async fn my_groups(db: Arc<Database>, user_id: &str) -> String {
    println!("[GROUPS] List groups for user {}", user_id);
    let rows = sqlx::query("SELECT g.id, g.name FROM groups g JOIN group_members m ON g.id = m.group_id WHERE m.user_id = ?")
        .bind(user_id)
        .fetch_all(&db.pool)
        .await;
    match rows {
        Ok(rows) => {
            let groups: Vec<String> = rows.iter().map(|r| format!("{}:{}", r.get::<String,_>("id"), r.get::<String,_>("name"))).collect();
            format!("OK: My groups: {}", groups.join(", "))
        }
        Err(e) => {
            println!("[GROUPS] Error listing groups: {}", e);
            format!("ERR: {}", e)
        }
    }
}

pub async fn invite(db: Arc<Database>, from_user: &str, to_user: &str, group_name: &str) -> String {
    println!("[GROUPS] Invite {} to group '{}' by {}", to_user, group_name, from_user);
    // Trova group_id
    let group_row = sqlx::query("SELECT id FROM groups WHERE name = ?")
        .bind(group_name)
        .fetch_optional(&db.pool)
        .await;
    let group_id = match group_row {
        Ok(Some(row)) => row.get::<String,_>("id"),
        _ => return "ERR: Group not found".to_string(),
    };
    // Verifica che from_user sia membro
    let is_member = sqlx::query("SELECT 1 FROM group_members WHERE group_id = ? AND user_id = ?")
        .bind(&group_id)
        .bind(from_user)
        .fetch_optional(&db.pool)
        .await
        .ok()
        .flatten()
        .is_some();
    if !is_member {
        return "ERR: Only group members can invite".to_string();
    }
    // Crea invito
    let created_at = chrono::Utc::now().timestamp();
    let res = sqlx::query("INSERT INTO group_invites (group_id, invited_user_id, invited_by, created_at, status) VALUES (?, ?, ?, ?, 'pending')")
        .bind(&group_id)
        .bind(to_user)
        .bind(from_user)
        .bind(created_at)
        .execute(&db.pool)
        .await;
    match res {
        Ok(_) => {
            println!("[GROUPS] Invite sent to {} for group {}", to_user, group_id);
            "OK: Invite sent".to_string()
        }
        Err(e) => {
            println!("[GROUPS] Error sending invite: {}", e);
            format!("ERR: Could not send invite: {}", e)
        }
    }
}

pub async fn my_invites(db: Arc<Database>, user_id: &str) -> String {
    println!("[GROUPS] List invites for user {}", user_id);
    let rows = sqlx::query("SELECT id, group_id FROM group_invites WHERE invited_user_id = ? AND status = 'pending'")
        .bind(user_id)
        .fetch_all(&db.pool)
        .await;
    match rows {
        Ok(rows) => {
            let invites: Vec<String> = rows.iter().map(|r| format!("{}:{}", r.get::<i64,_>("id"), r.get::<String,_>("group_id"))).collect();
            format!("OK: My invites: {}", invites.join(", "))
        }
        Err(e) => {
            println!("[GROUPS] Error listing invites: {}", e);
            format!("ERR: {}", e)
        }
    }
}

pub async fn accept_invite(db: Arc<Database>, user_id: &str, invite_id: &str) -> String {
    println!("[GROUPS] Accept invite {} by user {}", invite_id, user_id);
    // Trova invito
    let row = sqlx::query("SELECT group_id FROM group_invites WHERE id = ? AND invited_user_id = ? AND status = 'pending'")
        .bind(invite_id)
        .bind(user_id)
        .fetch_optional(&db.pool)
        .await;
    let group_id = match row {
        Ok(Some(row)) => row.get::<String,_>("group_id"),
        _ => return "ERR: Invite not found or already handled".to_string(),
    };
    // Aggiorna invito
    let res = sqlx::query("UPDATE group_invites SET status = 'accepted' WHERE id = ?")
        .bind(invite_id)
        .execute(&db.pool)
        .await;
    if res.is_err() {
        return "ERR: Could not update invite".to_string();
    }
    // Aggiungi a group_members
    let joined_at = chrono::Utc::now().timestamp();
    let res2 = sqlx::query("INSERT OR IGNORE INTO group_members (group_id, user_id, joined_at) VALUES (?, ?, ?)")
        .bind(&group_id)
        .bind(user_id)
        .bind(joined_at)
        .execute(&db.pool)
        .await;
    match res2 {
        Ok(_) => {
            println!("[GROUPS] User {} joined group {} via invite", user_id, group_id);
            "OK: Invite accepted".to_string()
        }
        Err(e) => {
            println!("[GROUPS] Error adding member: {}", e);
            format!("ERR: Could not join group: {}", e)
        }
    }
}

pub async fn reject_invite(db: Arc<Database>, user_id: &str, invite_id: &str) -> String {
    println!("[GROUPS] Reject invite {} by user {}", invite_id, user_id);
    let res = sqlx::query("UPDATE group_invites SET status = 'rejected' WHERE id = ? AND invited_user_id = ? AND status = 'pending'")
        .bind(invite_id)
        .bind(user_id)
        .execute(&db.pool)
        .await;
    match res {
        Ok(r) if r.rows_affected() > 0 => {
            println!("[GROUPS] Invite {} rejected by user {}", invite_id, user_id);
            "OK: Invite rejected".to_string()
        }
        _ => {
            println!("[GROUPS] Error rejecting invite {} by user {}", invite_id, user_id);
            "ERR: Could not reject invite".to_string()
        }
    }
}

pub async fn join_group(db: Arc<Database>, user_id: &str, group_name: &str) -> String {
    println!("[GROUPS] User {} joins group '{}'", user_id, group_name);
    // Trova group_id
    let group_row = sqlx::query("SELECT id FROM groups WHERE name = ?")
        .bind(group_name)
        .fetch_optional(&db.pool)
        .await;
    let group_id = match group_row {
        Ok(Some(row)) => row.get::<String,_>("id"),
        _ => return "ERR: Group not found".to_string(),
    };
    // Aggiungi a group_members
    let joined_at = chrono::Utc::now().timestamp();
    let res = sqlx::query("INSERT OR IGNORE INTO group_members (group_id, user_id, joined_at) VALUES (?, ?, ?)")
        .bind(&group_id)
        .bind(user_id)
        .bind(joined_at)
        .execute(&db.pool)
        .await;
    match res {
        Ok(_) => {
            println!("[GROUPS] User {} joined group {}", user_id, group_id);
            "OK: Joined group".to_string()
        }
        Err(e) => {
            println!("[GROUPS] Error joining group: {}", e);
            format!("ERR: Could not join group: {}", e)
        }
    }
}

pub async fn leave_group(db: Arc<Database>, user_id: &str, group_name: &str) -> String {
    println!("[GROUPS] User {} leaves group '{}'", user_id, group_name);
    // Trova group_id
    let group_row = sqlx::query("SELECT id FROM groups WHERE name = ?")
        .bind(group_name)
        .fetch_optional(&db.pool)
        .await;
    let group_id = match group_row {
        Ok(Some(row)) => row.get::<String,_>("id"),
        _ => return "ERR: Group not found".to_string(),
    };
    // Rimuovi da group_members
    let res = sqlx::query("DELETE FROM group_members WHERE group_id = ? AND user_id = ?")
        .bind(&group_id)
        .bind(user_id)
        .execute(&db.pool)
        .await;
    match res {
        Ok(_) => {
            println!("[GROUPS] User {} left group {}", user_id, group_id);
            "OK: Left group".to_string()
        }
        Err(e) => {
            println!("[GROUPS] Error leaving group: {}", e);
            format!("ERR: Could not leave group: {}", e)
        }
    }
}
