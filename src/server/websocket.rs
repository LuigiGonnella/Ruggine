use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use redis::aio::ConnectionManager;
use crate::server::database::Database;
use sqlx::Row;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    pub id: String,
    pub message_type: MessageType,
    pub sender: String,
    pub target: String, // user_id or group_id
    pub content: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthMessage {
    pub message_type: String, // "auth"
    pub session_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub message_type: String, // "auth_response"
    pub success: bool,
    pub user_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    PrivateMessage,
    GroupMessage,
    UserJoined,
    UserLeft,
    Notification,
    System,
}

pub type ClientId = String;
pub type UserId = String;

pub struct WebSocketConnection {
    pub client_id: ClientId,
    pub user_id: UserId,
    pub sender: tokio::sync::mpsc::UnboundedSender<Message>,
}

pub struct ChatWebSocketManager {
    // Mappa client_id -> connection info
    connections: Arc<Mutex<HashMap<ClientId, WebSocketConnection>>>,
    // Mappa user_id -> client_id (per trovare rapidamente la connessione di un utente)
    user_connections: Arc<Mutex<HashMap<UserId, ClientId>>>,
    // Broadcaster per messaggi globali
    message_broadcaster: broadcast::Sender<WebSocketMessage>,
    // Redis connection per pub/sub tra istanze server
    redis_manager: Arc<Mutex<ConnectionManager>>,
}

impl ChatWebSocketManager {
    pub async fn new(redis_url: &str) -> anyhow::Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let redis_manager = ConnectionManager::new(client).await?;
        
        let (message_broadcaster, _) = broadcast::channel(1000);
        
        Ok(Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            user_connections: Arc::new(Mutex::new(HashMap::new())),
            message_broadcaster,
            redis_manager: Arc::new(Mutex::new(redis_manager)),
        })
    }

    /// Validate session token and return user_id if valid
    pub async fn authenticate_session(&self, session_token: &str, db: &Database) -> Option<String> {
        println!("[WS:AUTH] Validating session token: {}", &session_token[..8]);
        
        let query = "SELECT user_id FROM sessions WHERE session_token = ? AND expires_at > ?";
        let now = chrono::Utc::now().timestamp();
        
        match sqlx::query(query)
            .bind(session_token)
            .bind(now)
            .fetch_optional(&db.pool)
            .await
        {
            Ok(Some(row)) => {
                let user_id: String = row.get("user_id");
                println!("[WS:AUTH] Session valid for user: {}", user_id);
                Some(user_id)
            }
            Ok(None) => {
                println!("[WS:AUTH] Session not found or expired");
                None
            }
            Err(e) => {
                println!("[WS:AUTH] Database error validating session: {}", e);
                None
            }
        }
    }

    pub async fn handle_authenticated_connection(
        &self,
        ws_stream: WebSocketStream<tokio::net::TcpStream>,
        db: Arc<Database>,
    ) -> anyhow::Result<()> {
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        
        // Wait for authentication message
        println!("[WS:AUTH] Waiting for authentication from client...");
        
        let auth_timeout = tokio::time::timeout(
            tokio::time::Duration::from_secs(30),
            ws_receiver.next()
        ).await;
        
        let auth_message = match auth_timeout {
            Ok(Some(Ok(Message::Text(text)))) => {
                match serde_json::from_str::<AuthMessage>(&text) {
                    Ok(auth) if auth.message_type == "auth" => auth,
                    Ok(_) => {
                        let error_response = AuthResponse {
                            message_type: "auth_response".to_string(),
                            success: false,
                            user_id: None,
                            error: Some("Invalid message type, expected 'auth'".to_string()),
                        };
                        let _ = ws_sender.send(Message::Text(serde_json::to_string(&error_response)?)).await;
                        return Err(anyhow::anyhow!("Invalid auth message type"));
                    }
                    Err(e) => {
                        let error_response = AuthResponse {
                            message_type: "auth_response".to_string(),
                            success: false,
                            user_id: None,
                            error: Some(format!("Invalid JSON: {}", e)),
                        };
                        let _ = ws_sender.send(Message::Text(serde_json::to_string(&error_response)?)).await;
                        return Err(anyhow::anyhow!("Invalid JSON in auth message"));
                    }
                }
            }
            Ok(Some(Ok(Message::Close(_)))) => {
                println!("[WS:AUTH] Client closed connection during auth");
                return Ok(());
            }
            Ok(Some(Ok(_))) => {
                println!("[WS:AUTH] Unexpected message type during auth");
                let error_response = AuthResponse {
                    message_type: "auth_response".to_string(),
                    success: false,
                    user_id: None,
                    error: Some("Expected text message for authentication".to_string()),
                };
                let _ = ws_sender.send(Message::Text(serde_json::to_string(&error_response)?)).await;
                return Err(anyhow::anyhow!("Unexpected message type during auth"));
            }
            Ok(Some(Err(e))) => {
                println!("[WS:AUTH] WebSocket error during auth: {}", e);
                return Err(anyhow::anyhow!("WebSocket error during auth"));
            }
            Ok(None) => {
                println!("[WS:AUTH] Connection closed during auth");
                return Ok(());
            }
            Err(_) => {
                println!("[WS:AUTH] Authentication timeout");
                let error_response = AuthResponse {
                    message_type: "auth_response".to_string(),
                    success: false,
                    user_id: None,
                    error: Some("Authentication timeout".to_string()),
                };
                let _ = ws_sender.send(Message::Text(serde_json::to_string(&error_response)?)).await;
                return Err(anyhow::anyhow!("Authentication timeout"));
            }
        };
        
        // Validate session token
        if let Some(user_id) = self.authenticate_session(&auth_message.session_token, &db).await {
            // Authentication successful
            let success_response = AuthResponse {
                message_type: "auth_response".to_string(),
                success: true,
                user_id: Some(user_id.clone()),
                error: None,
            };
            
            let _ = ws_sender.send(Message::Text(serde_json::to_string(&success_response)?)).await;
            println!("[WS:AUTH] Authentication successful for user: {}", user_id);
            
            // Rebuild WebSocket stream and proceed with authenticated connection
            let rebuilt_stream = ws_sender.reunite(ws_receiver)
                .map_err(|e| anyhow::anyhow!("Failed to reunite WebSocket stream: {}", e))?;
            
            return self.add_connection(rebuilt_stream, user_id).await;
        } else {
            // Authentication failed
            let error_response = AuthResponse {
                message_type: "auth_response".to_string(),
                success: false,
                user_id: None,
                error: Some("Invalid or expired session token".to_string()),
            };
            
            let _ = ws_sender.send(Message::Text(serde_json::to_string(&error_response)?)).await;
            println!("[WS:AUTH] Authentication failed for token: {}", &auth_message.session_token[..8]);
            
            return Err(anyhow::anyhow!("Authentication failed"));
        }
    }

    pub async fn add_connection(
        &self,
        ws_stream: WebSocketStream<tokio::net::TcpStream>,
        user_id: UserId,
    ) -> anyhow::Result<()> {
        let client_id = Uuid::new_v4().to_string();
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        // Aggiungi connessione alle mappe
        {
            let mut connections = self.connections.lock().await;
            let mut user_connections = self.user_connections.lock().await;
            
            connections.insert(client_id.clone(), WebSocketConnection {
                client_id: client_id.clone(),
                user_id: user_id.clone(),
                sender: tx,
            });
            
            user_connections.insert(user_id.clone(), client_id.clone());
        }

        let connections_clone = self.connections.clone();
        let user_connections_clone = self.user_connections.clone();
        let client_id_clone = client_id.clone();
        let user_id_clone = user_id.clone();
        let message_broadcaster = self.message_broadcaster.clone();
        let redis_manager = self.redis_manager.clone();

        // Task per inviare messaggi al client
        let send_task = tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if ws_sender.send(message).await.is_err() {
                    break;
                }
            }
        });

        // Task per ricevere messaggi dal client
        let receive_task = tokio::spawn(async move {
            while let Some(message) = ws_receiver.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        if let Ok(ws_message) = serde_json::from_str::<WebSocketMessage>(&text) {
                            // Invia il messaggio tramite broadcaster locale
                            let _ = message_broadcaster.send(ws_message.clone());
                            
                            // Pubblica su Redis per altre istanze server
                            let mut redis_conn = redis_manager.lock().await;
                            let channel = match ws_message.message_type {
                                MessageType::PrivateMessage => format!("private:{}", ws_message.target),
                                MessageType::GroupMessage => format!("group:{}", ws_message.target),
                                _ => "system".to_string(),
                            };
                                
                            let serialized = serde_json::to_string(&ws_message).unwrap_or_default();
                            let _: Result<(), _> = redis::cmd("PUBLISH")
                                .arg(&channel)
                                .arg(&serialized)
                                .query_async(&mut *redis_conn)
                                .await;
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Err(_) => break,
                    _ => {}
                }
            }

            // Cleanup quando la connessione si chiude
            {
                let mut connections = connections_clone.lock().await;
                let mut user_connections = user_connections_clone.lock().await;
                
                connections.remove(&client_id_clone);
                user_connections.remove(&user_id_clone);
            }
        });

        // Aspetta che uno dei task finisca (disconnessione)
        tokio::select! {
            _ = send_task => {},
            _ = receive_task => {},
        }

        Ok(())
    }

    pub async fn send_to_user(&self, user_id: &str, message: WebSocketMessage) -> anyhow::Result<()> {
        let connections = self.connections.lock().await;
        let user_connections = self.user_connections.lock().await;
        
        if let Some(client_id) = user_connections.get(user_id) {
            if let Some(connection) = connections.get(client_id) {
                let json_message = serde_json::to_string(&message)?;
                let _ = connection.sender.send(Message::Text(json_message));
            }
        }
        
        Ok(())
    }

    pub async fn send_to_group(&self, _group_id: &str, message: WebSocketMessage, exclude_user: Option<&str>) -> anyhow::Result<()> {
        // In una implementazione completa, dovresti avere una mappa group_id -> Vec<user_id>
        // Per ora inviamo a tutti gli utenti connessi (da migliorare)
        let connections = self.connections.lock().await;
        let json_message = serde_json::to_string(&message)?;
        
        for connection in connections.values() {
            if let Some(exclude) = exclude_user {
                if connection.user_id == exclude {
                    continue;
                }
            }
            let _ = connection.sender.send(Message::Text(json_message.clone()));
        }
        
        Ok(())
    }

    pub async fn broadcast_message(&self, message: WebSocketMessage) -> anyhow::Result<()> {
        let _ = self.message_broadcaster.send(message);
        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<WebSocketMessage> {
        self.message_broadcaster.subscribe()
    }

    pub async fn start_redis_subscriber(&self) -> anyhow::Result<()> {
        let _redis_manager = self.redis_manager.clone();
        let message_broadcaster = self.message_broadcaster.clone();
        let connections = self.connections.clone();
        let user_connections = self.user_connections.clone();
        
        tokio::spawn(async move {
            println!("[WS:REDIS] Starting Redis pub/sub subscriber...");
            
            let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
            
            loop {
                match redis::Client::open(redis_url.as_str()) {
                    Ok(client) => {
                        match client.get_async_connection().await {
                            Ok(con) => {
                                println!("[WS:REDIS] Connected to Redis for pub/sub");
                                
                                // Subscribe to relevant channels
                                let mut pubsub = con.into_pubsub();
                                let _ = pubsub.subscribe("private:*").await;
                                let _ = pubsub.subscribe("group:*").await;
                                let _ = pubsub.subscribe("system").await;
                                let _ = pubsub.subscribe("notifications").await;
                                
                                println!("[WS:REDIS] Subscribed to channels: private:*, group:*, system, notifications");
                                
                                // Listen for messages
                                let mut stream = pubsub.on_message();
                                loop {
                                    match stream.next().await {
                                        Some(msg) => {
                                            let channel: String = msg.get_channel_name().to_string();
                                            let payload: String = match msg.get_payload() {
                                                Ok(p) => p,
                                                Err(_) => continue,
                                            };
                                            
                                            println!("[WS:REDIS] Received message on channel '{}': {}", channel, payload);
                                            
                                            if let Ok(ws_message) = serde_json::from_str::<WebSocketMessage>(&payload) {
                                                // Route message based on type
                                                match ws_message.message_type {
                                                    MessageType::PrivateMessage => {
                                                        // Send to specific user
                                                        let user_connections_guard = user_connections.lock().await;
                                                        let connections_guard = connections.lock().await;
                                                        
                                                        if let Some(client_id) = user_connections_guard.get(&ws_message.target) {
                                                            if let Some(connection) = connections_guard.get(client_id) {
                                                                let json_msg = serde_json::to_string(&ws_message).unwrap_or_default();
                                                                let _ = connection.sender.send(tokio_tungstenite::tungstenite::Message::Text(json_msg));
                                                                println!("[WS:REDIS] Delivered private message to user {}", ws_message.target);
                                                            }
                                                        }
                                                    }
                                                    MessageType::GroupMessage => {
                                                        // Broadcast to all connected users (would filter by group in production)
                                                        let connections_guard = connections.lock().await;
                                                        let json_msg = serde_json::to_string(&ws_message).unwrap_or_default();
                                                        
                                                        for connection in connections_guard.values() {
                                                            if connection.user_id != ws_message.sender {
                                                                let _ = connection.sender.send(tokio_tungstenite::tungstenite::Message::Text(json_msg.clone()));
                                                            }
                                                        }
                                                        println!("[WS:REDIS] Broadcasted group message from {}", ws_message.sender);
                                                    }
                                                    MessageType::Notification | MessageType::System => {
                                                        // Broadcast to all connected users
                                                        let connections_guard = connections.lock().await;
                                                        let json_msg = serde_json::to_string(&ws_message).unwrap_or_default();
                                                        
                                                        for connection in connections_guard.values() {
                                                            let _ = connection.sender.send(tokio_tungstenite::tungstenite::Message::Text(json_msg.clone()));
                                                        }
                                                        println!("[WS:REDIS] Broadcasted {} message", if matches!(ws_message.message_type, MessageType::Notification) { "notification" } else { "system" });
                                                    }
                                                    _ => {}
                                                }
                                                
                                                // Also broadcast locally
                                                let _ = message_broadcaster.send(ws_message);
                                            }
                                        }
                                        None => {
                                            println!("[WS:REDIS] Redis stream ended");
                                            break;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                println!("[WS:REDIS] Failed to connect to Redis: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("[WS:REDIS] Failed to create Redis client: {}", e);
                    }
                }
                
                println!("[WS:REDIS] Redis subscriber disconnected, retrying in 5 seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        });
        
        Ok(())
    }
}
