use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, oneshot};

// Map user_id -> list of control senders to force disconnect
#[derive(Clone, Default)]
pub struct PresenceRegistry {
    inner: Arc<Mutex<HashMap<String, Vec<oneshot::Sender<()>>>>>,
}

impl PresenceRegistry {
    pub fn new() -> Self { Self { inner: Arc::new(Mutex::new(HashMap::new())) } }

    // Register a connection for user_id; returns a receiver that connection should await
    pub async fn register(&self, user_id: &str) -> oneshot::Receiver<()> {
        let (tx, rx) = oneshot::channel();
        let mut map = self.inner.lock().await;
        map.entry(user_id.to_string()).or_default().push(tx);
        println!("[PRESENCE] Registered connection for user {} (total={})",
            user_id,
            map.get(user_id).map(|v| v.len()).unwrap_or(0)
        );
        rx
    }

    // Remove all connections for user and return number removed
    pub async fn kick_all(&self, user_id: &str) -> usize {
        let mut map = self.inner.lock().await;
        if let Some(vec) = map.remove(user_id) {
            // take length before consuming the vector
            let count = vec.len();
            println!("[PRESENCE] Kicking {} connections for user {}", count, user_id);
            for tx in vec {
                let _ = tx.send(());
            }
            count
        } else { 0 }
    }

    // Remove a single connection by dropping one sender (called when a connection ends)
    pub async fn unregister_one(&self, user_id: &str) {
        let mut map = self.inner.lock().await;
        if let Some(vec) = map.get_mut(user_id) {
            if !vec.is_empty() {
                vec.remove(0);
                println!("[PRESENCE] Unregistered one connection for user {} (remaining={})", user_id, vec.len());
            }
            if vec.is_empty() {
                map.remove(user_id);
                println!("[PRESENCE] No remaining connections for user {}; removed from registry", user_id);
            }
        }
    }

    // Return how many active connections are currently registered for a user
    pub async fn count(&self, user_id: &str) -> usize {
        let map = self.inner.lock().await;
        map.get(user_id).map(|v| v.len()).unwrap_or(0)
    }
}
