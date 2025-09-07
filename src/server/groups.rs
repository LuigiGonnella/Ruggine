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

pub async fn create_group_with_participants(db: Arc<Database>, user_id: &str, group_name: &str, participants: Option<&str>) -> String {
    println!("[GROUPS] Create group '{}' by user {} with participants: {:?}", group_name, user_id, participants);
    let group_id = uuid::Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().timestamp();
    let tx = db.pool.begin().await;
    match tx {
        Ok(mut tx) => {
            // Create group
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
            
            // Add creator as member
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
            
            // Send invites to participants if provided (don't add them directly)
            if let Some(participants_str) = participants {
                let participant_usernames: Vec<&str> = participants_str.split(',').collect();
                for username in participant_usernames {
                    let username = username.trim();
                    if !username.is_empty() && username != user_id {
                        // Get user_id from username
                        if let Ok(Some(row)) = sqlx::query("SELECT id FROM users WHERE username = ?")
                            .bind(username)
                            .fetch_optional(&mut *tx)
                            .await
                        {
                            let participant_id: String = row.get("id");
                            // Create invite instead of adding directly to group
                            let _ = sqlx::query("INSERT INTO group_invites (group_id, invited_user_id, invited_by, created_at, status) VALUES (?, ?, ?, ?, 'pending')")
                                .bind(&group_id)
                                .bind(&participant_id)
                                .bind(user_id)
                                .bind(created_at)
                                .execute(&mut *tx)
                                .await;
                            println!("[GROUPS] Sent invite to participant {} for group {}", username, group_id);
                        }
                    }
                }
            }
            
            tx.commit().await.ok();
            println!("[GROUPS] Group '{}' created with id {}", group_name, group_id);
            format!("OK: Group '{}' created with ID: {}", group_name, group_id)
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

pub async fn invite_user_to_group(db: Arc<Database>, from_user_id: &str, to_username: &str, group_id: &str) -> String {
    println!("[GROUPS] Invite {} to group '{}' by {}", to_username, group_id, from_user_id);
    
    // Verify group exists
    let group_row = sqlx::query("SELECT id FROM groups WHERE id = ?")
        .bind(group_id)
        .fetch_optional(&db.pool)
        .await;
    if group_row.is_err() || group_row.unwrap().is_none() {
        return "ERR: Group not found".to_string();
    }
    
    // Get user_id from username
    let user_row = sqlx::query("SELECT id FROM users WHERE username = ?")
        .bind(to_username)
        .fetch_optional(&db.pool)
        .await;
    let to_user_id = match user_row {
        Ok(Some(row)) => row.get::<String,_>("id"),
        _ => return "ERR: User not found".to_string(),
    };
    
    // Verify that from_user is member of the group
    let is_member = sqlx::query("SELECT 1 FROM group_members WHERE group_id = ? AND user_id = ?")
        .bind(group_id)
        .bind(from_user_id)
        .fetch_optional(&db.pool)
        .await
        .ok()
        .flatten()
        .is_some();
    if !is_member {
        return "ERR: Only group members can invite".to_string();
    }
    
    // Check if user is already a member
    let already_member = sqlx::query("SELECT 1 FROM group_members WHERE group_id = ? AND user_id = ?")
        .bind(group_id)
        .bind(&to_user_id)
        .fetch_optional(&db.pool)
        .await
        .ok()
        .flatten()
        .is_some();
    if already_member {
        return "ERR: User is already a member of this group".to_string();
    }
    
    // Check if there's already a pending invite
    let existing_invite = sqlx::query("SELECT 1 FROM group_invites WHERE group_id = ? AND invited_user_id = ? AND status = 'pending'")
        .bind(group_id)
        .bind(&to_user_id)
        .fetch_optional(&db.pool)
        .await
        .ok()
        .flatten()
        .is_some();
    if existing_invite {
        return "ERR: User already has a pending invite to this group".to_string();
    }
    
    // Create group invite
    let created_at = chrono::Utc::now().timestamp();
    let res = sqlx::query("INSERT INTO group_invites (group_id, invited_user_id, invited_by, created_at, status) VALUES (?, ?, ?, ?, 'pending')")
        .bind(group_id)
        .bind(&to_user_id)
        .bind(from_user_id)
        .bind(created_at)
        .execute(&db.pool)
        .await;
    match res {
        Ok(_) => {
            println!("[GROUPS] Invite sent to {} for group {}", to_username, group_id);
            format!("OK: Invite sent to {} successfully", to_username)
        }
        Err(e) => {
            println!("[GROUPS] Error sending invite: {}", e);
            format!("ERR: Could not send invite: {}", e)
        }
    }
}

pub async fn get_group_members(db: Arc<Database>, group_id: &str) -> String {
    println!("[GROUPS] Get members for group {}", group_id);
    let rows = sqlx::query("SELECT u.username FROM group_members gm JOIN users u ON gm.user_id = u.id WHERE gm.group_id = ?")
        .bind(group_id)
        .fetch_all(&db.pool)
        .await;
    match rows {
        Ok(rows) => {
            let members: Vec<String> = rows.iter().map(|r| r.get::<String,_>("username")).collect();
            format!("OK: Group members: {}", members.join(", "))
        }
        Err(e) => {
            println!("[GROUPS] Error getting group members: {}", e);
            format!("ERR: {}", e)
        }
    }
}

pub async fn my_invites(db: Arc<Database>, user_id: &str) -> String {
    println!("[GROUPS] List invites for user {}", user_id);
    let rows = sqlx::query("SELECT gi.id, g.name as group_name, u.username as invited_by FROM group_invites gi JOIN groups g ON gi.group_id = g.id JOIN users u ON gi.invited_by = u.id WHERE gi.invited_user_id = ? AND gi.status = 'pending'")
        .bind(user_id)
        .fetch_all(&db.pool)
        .await;
    match rows {
        Ok(rows) => {
            let invites: Vec<String> = rows.iter().map(|r| {
                format!("{}:{}:{}", 
                    r.get::<i64,_>("id"), 
                    r.get::<String,_>("group_name"), 
                    r.get::<String,_>("invited_by")
                )
            }).collect();
            // Remove duplicates by converting to HashSet and back
            let unique_invites: std::collections::HashSet<String> = invites.into_iter().collect();
            let unique_vec: Vec<String> = unique_invites.into_iter().collect();
            format!("OK: Group invites: {}", unique_vec.join(" | "))
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

pub async fn leave_group(db: Arc<Database>, user_id: &str, group_ident: &str) -> String {
    println!("[GROUPS] User {} leaves group '{}'", user_id, group_ident);
    // Try to resolve the provided identifier as a group id first, then fall back to name
    let group_row_by_id = sqlx::query("SELECT id FROM groups WHERE id = ?")
        .bind(group_ident)
        .fetch_optional(&db.pool)
        .await;

    let group_id = match group_row_by_id {
        Ok(Some(row)) => row.get::<String,_>("id"),
        _ => {
            // Fallback: try by name but prefer a group the user is actually member of
            // This avoids ambiguity when multiple groups share the same name.
            let group_row_by_name = sqlx::query("SELECT g.id FROM groups g JOIN group_members m ON g.id = m.group_id WHERE g.name = ? AND m.user_id = ? LIMIT 1")
                .bind(group_ident)
                .bind(user_id)
                .fetch_optional(&db.pool)
                .await;
            match group_row_by_name {
                Ok(Some(row)) => {
                    let gid: String = row.get("id");
                    println!("[GROUPS] Resolved group name '{}' to id {} (user member)", group_ident, gid);
                    gid
                }
                _ => {
                    // As a last resort, try global lookup by name (may still be ambiguous)
                    let group_row_global = sqlx::query("SELECT id FROM groups WHERE name = ? LIMIT 1")
                        .bind(group_ident)
                        .fetch_optional(&db.pool)
                        .await;
                    match group_row_global {
                        Ok(Some(row)) => {
                            let gid: String = row.get("id");
                            println!("[GROUPS] Resolved group name '{}' to id {} (global lookup)", group_ident, gid);
                            gid
                        }
                        _ => return "ERR: Group not found".to_string(),
                    }
                }
            }
        }
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
