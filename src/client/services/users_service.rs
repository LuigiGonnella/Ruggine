use crate::client::services::chat_service::ChatService;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Default)]
pub struct UsersService;

impl UsersService {
    pub fn new() -> Self { Self {} }

    /// List online users. Returns Vec<String> of usernames on success.
    pub async fn list_online(svc: &Arc<Mutex<ChatService>>, host: &str, session_token: &str) -> anyhow::Result<Vec<String>> {
        let mut guard = svc.lock().await;
        let cmd = format!("/online_users {}", session_token);
        let resp = guard.send_command(host, cmd).await?;
        if !resp.starts_with("OK:") {
            return Err(anyhow::anyhow!(resp));
        }
        // expected: "OK: Online users: alice, bob" or "OK: alice, bob"
        // Try to extract the part after the second ':' if present, otherwise after the first ':'
        let after = if resp.matches(':').count() >= 2 {
            resp.splitn(3, ':').nth(2).unwrap_or("")
        } else {
            resp.split_once(':').map(|x| x.1).unwrap_or("")
        };
        let list = after.trim().split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        Ok(list)
    }

    /// List all users. Returns Vec<String> of usernames on success.
    pub async fn list_all(svc: &Arc<Mutex<ChatService>>, host: &str) -> anyhow::Result<Vec<String>> {
        let mut guard = svc.lock().await;
        let cmd = "/all_users".to_string();
        let resp = guard.send_command(host, cmd).await?;
        if !resp.starts_with("OK:") {
            return Err(anyhow::anyhow!(resp));
        }
        let after = if resp.matches(':').count() >= 2 {
            resp.splitn(3, ':').nth(2).unwrap_or("")
        } else {
            resp.split_once(':').map(|x| x.1).unwrap_or("")
        };
        let list = after.trim().split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        Ok(list)
    }
}
