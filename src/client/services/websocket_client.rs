use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use serde::{Serialize, Deserialize};
use anyhow::Result;

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

#[derive(Debug, Clone)]
pub enum WebSocketError {
    ConnectionFailed(String),
    AuthenticationFailed(String),
    MessageSendFailed(String),
    Disconnected,
    InvalidMessage(String),
    Timeout,
}

impl std::fmt::Display for WebSocketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebSocketError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            WebSocketError::AuthenticationFailed(msg) => write!(f, "Authentication failed: {}", msg),
            WebSocketError::MessageSendFailed(msg) => write!(f, "Message send failed: {}", msg),
            WebSocketError::Disconnected => write!(f, "WebSocket disconnected"),
            WebSocketError::InvalidMessage(msg) => write!(f, "Invalid message: {}", msg),
            WebSocketError::Timeout => write!(f, "Operation timed out"),
        }
    }
}

impl std::error::Error for WebSocketError {}

pub struct WebSocketClient {
    url: String,
    session_token: Option<String>,
    connection_retry_attempts: u32,
    max_retry_attempts: u32,
    retry_delay: tokio::time::Duration,
}

impl WebSocketClient {
    pub fn new(url: String) -> Self {
        Self {
            url,
            session_token: None,
            connection_retry_attempts: 0,
            max_retry_attempts: 5,
            retry_delay: tokio::time::Duration::from_secs(2),
        }
    }

    pub fn set_session_token(&mut self, token: String) {
        self.session_token = Some(token);
    }

    pub async fn connect_with_auth(&mut self) -> Result<(), WebSocketError> {
        for attempt in 1..=self.max_retry_attempts {
            match self.try_connect().await {
                Ok(()) => {
                    self.connection_retry_attempts = 0;
                    println!("[WS:CLIENT] Successfully connected and authenticated");
                    return Ok(());
                }
                Err(e) => {
                    self.connection_retry_attempts = attempt;
                    println!("[WS:CLIENT] Connection attempt {} failed: {}", attempt, e);
                    
                    if attempt < self.max_retry_attempts {
                        println!("[WS:CLIENT] Retrying in {:?}...", self.retry_delay);
                        tokio::time::sleep(self.retry_delay).await;
                        // Exponential backoff
                        self.retry_delay = std::cmp::min(
                            self.retry_delay * 2,
                            tokio::time::Duration::from_secs(30)
                        );
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        
        Err(WebSocketError::ConnectionFailed("Max retry attempts exceeded".to_string()))
    }

    async fn try_connect(&self) -> Result<(), WebSocketError> {
        // Connect to WebSocket
        let (ws_stream, _) = connect_async(&self.url)
            .await
            .map_err(|e| WebSocketError::ConnectionFailed(format!("Failed to connect: {}", e)))?;

        println!("[WS:CLIENT] Connected to {}", self.url);

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Send authentication message
        let auth_message = AuthMessage {
            message_type: "auth".to_string(),
            session_token: self.session_token.clone()
                .ok_or_else(|| WebSocketError::AuthenticationFailed("No session token provided".to_string()))?,
        };

        let auth_json = serde_json::to_string(&auth_message)
            .map_err(|e| WebSocketError::AuthenticationFailed(format!("Failed to serialize auth message: {}", e)))?;

        ws_sender
            .send(Message::Text(auth_json))
            .await
            .map_err(|e| WebSocketError::AuthenticationFailed(format!("Failed to send auth message: {}", e)))?;

        // Wait for authentication response
        let auth_timeout = tokio::time::timeout(
            tokio::time::Duration::from_secs(10),
            ws_receiver.next()
        ).await;

        let auth_response = match auth_timeout {
            Ok(Some(Ok(Message::Text(text)))) => {
                serde_json::from_str::<AuthResponse>(&text)
                    .map_err(|e| WebSocketError::AuthenticationFailed(format!("Invalid auth response: {}", e)))?
            }
            Ok(Some(Ok(Message::Close(_)))) => {
                return Err(WebSocketError::AuthenticationFailed("Server closed connection during auth".to_string()));
            }
            Ok(Some(Ok(_))) => {
                return Err(WebSocketError::AuthenticationFailed("Unexpected message type during auth".to_string()));
            }
            Ok(Some(Err(e))) => {
                return Err(WebSocketError::AuthenticationFailed(format!("WebSocket error during auth: {}", e)));
            }
            Ok(None) => {
                return Err(WebSocketError::AuthenticationFailed("Connection closed during auth".to_string()));
            }
            Err(_) => {
                return Err(WebSocketError::Timeout);
            }
        };

        if auth_response.success {
            println!("[WS:CLIENT] Authentication successful for user: {:?}", auth_response.user_id);
            // Here you would start the message handling loop
            // For now, we just return success
            Ok(())
        } else {
            let error_msg = auth_response.error.unwrap_or_else(|| "Unknown authentication error".to_string());
            Err(WebSocketError::AuthenticationFailed(error_msg))
        }
    }

    pub fn reset_retry_delay(&mut self) {
        self.retry_delay = tokio::time::Duration::from_secs(2);
    }

    pub fn get_retry_attempts(&self) -> u32 {
        self.connection_retry_attempts
    }
}
