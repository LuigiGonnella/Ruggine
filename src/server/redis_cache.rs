use redis::aio::ConnectionManager;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedMessage {
    pub id: String,
    pub sender_id: String,
    pub recipient_id: Option<String>, // None for group messages
    pub group_id: Option<String>,     // None for private messages
    pub content: String,
    pub timestamp: i64,
    pub message_type: String, // "private" or "group"
}

pub struct RedisMessageCache {
    redis_manager: Arc<Mutex<ConnectionManager>>,
    message_ttl: u64, // TTL in seconds for cached messages
}

impl RedisMessageCache {
    pub async fn new(redis_url: &str, message_ttl: u64) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let redis_manager = ConnectionManager::new(client).await?;
        
        Ok(Self {
            redis_manager: Arc::new(Mutex::new(redis_manager)),
            message_ttl,
        })
    }

    /// Cache a private message in Redis
    pub async fn cache_private_message(
        &self,
        message_id: &str,
        sender_id: &str,
        recipient_id: &str,
        content: &str,
        timestamp: i64,
    ) -> Result<()> {
        let cached_message = CachedMessage {
            id: message_id.to_string(),
            sender_id: sender_id.to_string(),
            recipient_id: Some(recipient_id.to_string()),
            group_id: None,
            content: content.to_string(),
            timestamp,
            message_type: "private".to_string(),
        };

        let json_data = serde_json::to_string(&cached_message)?;
        let mut conn = self.redis_manager.lock().await;

        // Cache in multiple keys for efficient retrieval
        let message_key = format!("message:{}", message_id);
        let conversation_key = format!("conversation:{}:{}", 
            std::cmp::min(sender_id, recipient_id),
            std::cmp::max(sender_id, recipient_id)
        );

        // Store message data
        let _: () = redis::cmd("SETEX")
            .arg(&message_key)
            .arg(self.message_ttl)
            .arg(&json_data)
            .query_async(&mut *conn)
            .await?;

        // Add to conversation timeline (sorted set by timestamp)
        let _: () = redis::cmd("ZADD")
            .arg(&conversation_key)
            .arg(timestamp)
            .arg(message_id)
            .query_async(&mut *conn)
            .await?;

        // Set TTL on conversation key
        let _: () = redis::cmd("EXPIRE")
            .arg(&conversation_key)
            .arg(self.message_ttl)
            .query_async(&mut *conn)
            .await?;

        println!("[REDIS:CACHE] Cached private message {} between {} and {}", message_id, sender_id, recipient_id);
        Ok(())
    }

    /// Cache a group message in Redis
    pub async fn cache_group_message(
        &self,
        message_id: &str,
        sender_id: &str,
        group_id: &str,
        content: &str,
        timestamp: i64,
    ) -> Result<()> {
        let cached_message = CachedMessage {
            id: message_id.to_string(),
            sender_id: sender_id.to_string(),
            recipient_id: None,
            group_id: Some(group_id.to_string()),
            content: content.to_string(),
            timestamp,
            message_type: "group".to_string(),
        };

        let json_data = serde_json::to_string(&cached_message)?;
        let mut conn = self.redis_manager.lock().await;

        // Cache in multiple keys for efficient retrieval
        let message_key = format!("message:{}", message_id);
        let group_timeline_key = format!("group_timeline:{}", group_id);

        // Store message data
        let _: () = redis::cmd("SETEX")
            .arg(&message_key)
            .arg(self.message_ttl)
            .arg(&json_data)
            .query_async(&mut *conn)
            .await?;

        // Add to group timeline (sorted set by timestamp)
        let _: () = redis::cmd("ZADD")
            .arg(&group_timeline_key)
            .arg(timestamp)
            .arg(message_id)
            .query_async(&mut *conn)
            .await?;

        // Set TTL on group timeline key
        let _: () = redis::cmd("EXPIRE")
            .arg(&group_timeline_key)
            .arg(self.message_ttl)
            .query_async(&mut *conn)
            .await?;

        println!("[REDIS:CACHE] Cached group message {} in group {}", message_id, group_id);
        Ok(())
    }

    /// Retrieve recent private messages from cache
    pub async fn get_private_messages(
        &self,
        user1_id: &str,
        user2_id: &str,
        limit: i64,
    ) -> Result<Vec<CachedMessage>> {
        let mut conn = self.redis_manager.lock().await;
        
        let conversation_key = format!("conversation:{}:{}", 
            std::cmp::min(user1_id, user2_id),
            std::cmp::max(user1_id, user2_id)
        );

        // Get recent message IDs from sorted set (most recent first)
        let message_ids: Vec<String> = redis::cmd("ZREVRANGE")
            .arg(&conversation_key)
            .arg(0)
            .arg(limit - 1)
            .query_async(&mut *conn)
            .await?;

        let mut messages = Vec::new();
        for message_id in message_ids {
            let message_key = format!("message:{}", message_id);
            match redis::cmd("GET")
                .arg(&message_key)
                .query_async::<_, String>(&mut *conn)
                .await
            {
                Ok(json_data) => {
                    if let Ok(cached_message) = serde_json::from_str::<CachedMessage>(&json_data) {
                        messages.push(cached_message);
                    }
                }
                Err(_) => {
                    // Message not found or expired, skip
                }
            }
        }

        println!("[REDIS:CACHE] Retrieved {} cached private messages between {} and {}", messages.len(), user1_id, user2_id);
        Ok(messages)
    }

    /// Retrieve recent group messages from cache
    pub async fn get_group_messages(
        &self,
        group_id: &str,
        limit: i64,
    ) -> Result<Vec<CachedMessage>> {
        let mut conn = self.redis_manager.lock().await;
        
        let group_timeline_key = format!("group_timeline:{}", group_id);

        // Get recent message IDs from sorted set (most recent first)
        let message_ids: Vec<String> = redis::cmd("ZREVRANGE")
            .arg(&group_timeline_key)
            .arg(0)
            .arg(limit - 1)
            .query_async(&mut *conn)
            .await?;

        let mut messages = Vec::new();
        for message_id in message_ids {
            let message_key = format!("message:{}", message_id);
            match redis::cmd("GET")
                .arg(&message_key)
                .query_async::<_, String>(&mut *conn)
                .await
            {
                Ok(json_data) => {
                    if let Ok(cached_message) = serde_json::from_str::<CachedMessage>(&json_data) {
                        messages.push(cached_message);
                    }
                }
                Err(_) => {
                    // Message not found or expired, skip
                }
            }
        }

        println!("[REDIS:CACHE] Retrieved {} cached group messages for group {}", messages.len(), group_id);
        Ok(messages)
    }

    /// Clear old messages from cache (cleanup function)
    pub async fn cleanup_old_messages(&self) -> Result<u64> {
        let mut conn = self.redis_manager.lock().await;
        let current_time = chrono::Utc::now().timestamp();
        let cutoff_time = current_time - (self.message_ttl as i64);
        
        // Find conversation and group keys to clean
        let pattern_keys: Vec<String> = redis::cmd("KEYS")
            .arg("conversation:*")
            .query_async(&mut *conn)
            .await?;
            
        let group_keys: Vec<String> = redis::cmd("KEYS")
            .arg("group_timeline:*")
            .query_async(&mut *conn)
            .await?;

        let mut cleaned_count = 0u64;

        // Clean conversation keys
        for key in pattern_keys {
            let removed: i64 = redis::cmd("ZREMRANGEBYSCORE")
                .arg(&key)
                .arg("-inf")
                .arg(cutoff_time)
                .query_async(&mut *conn)
                .await?;
            cleaned_count += removed as u64;
        }

        // Clean group timeline keys
        for key in group_keys {
            let removed: i64 = redis::cmd("ZREMRANGEBYSCORE")
                .arg(&key)
                .arg("-inf")
                .arg(cutoff_time)
                .query_async(&mut *conn)
                .await?;
            cleaned_count += removed as u64;
        }

        println!("[REDIS:CACHE] Cleaned {} old message references", cleaned_count);
        Ok(cleaned_count)
    }

    /// Health check for Redis connection
    pub async fn health_check(&self) -> Result<bool> {
        let mut conn = self.redis_manager.lock().await;
        let pong: String = redis::cmd("PING")
            .query_async(&mut *conn)
            .await?;
        Ok(pong == "PONG")
    }
}
