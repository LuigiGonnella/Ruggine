use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use serde::{Serialize, Deserialize};
use anyhow::Result;
use tokio::sync::mpsc;

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

// Messaggio ricevuto dal WebSocket - rappresenta un nuovo messaggio chat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingChatMessage {
    pub message_type: String, // "new_message"
    pub chat_type: String,    // "private" or "group"
    pub from_user: String,
    pub to_user: Option<String>, // per messaggi privati
    pub group_id: Option<String>, // per messaggi di gruppo  
    pub content: String,
    pub timestamp: i64,
}

// Messaggio da inviare tramite WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingChatMessage {
    pub message_type: String, // "send_message"
    pub chat_type: String,    // "private" or "group"
    pub to_user: Option<String>, // per messaggi privati
    pub group_id: Option<String>, // per messaggi di gruppo
    pub content: String,
    pub session_token: String,
}

#[derive(Debug, Clone)]
pub enum WebSocketMessage {
    NewMessage(IncomingChatMessage),
    UserStatusUpdate { user_id: String, online: bool },
    Error(String),
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
    /// Channel per inviare messaggi ricevuti all'applicazione
    pub message_sender: Option<mpsc::UnboundedSender<WebSocketMessage>>,
    /// Receiver per l'applicazione per ricevere i messaggi
    pub message_receiver: Option<mpsc::UnboundedReceiver<WebSocketMessage>>,
    /// Sender per inviare messaggi al WebSocket
    pub outgoing_sender: Option<mpsc::UnboundedSender<OutgoingChatMessage>>,
}

impl WebSocketClient {
    pub fn new(url: String) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            url,
            session_token: None,
            connection_retry_attempts: 0,
            max_retry_attempts: 5,
            retry_delay: tokio::time::Duration::from_secs(2),
            message_sender: Some(tx),
            message_receiver: Some(rx),
            outgoing_sender: None,
        }
    }

    /// Prende il receiver per l'applicazione - può essere chiamato solo una volta
    pub fn take_receiver(&mut self) -> Option<mpsc::UnboundedReceiver<WebSocketMessage>> {
        self.message_receiver.take()
    }

    pub fn set_session_token(&mut self, token: String) {
        self.session_token = Some(token);
    }

    pub async fn connect_with_auth(&mut self) -> Result<(), WebSocketError> {
        for attempt in 1..=self.max_retry_attempts {
            match self.try_connect().await {
                Ok(outgoing_sender) => {
                    self.connection_retry_attempts = 0;
                    self.outgoing_sender = Some(outgoing_sender);
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

    async fn try_connect(&self) -> Result<mpsc::UnboundedSender<OutgoingChatMessage>, WebSocketError> {
        // Connect to WebSocket
        println!("[WS:CLIENT] Connecting to {}", self.url);
        let (ws_stream, _) = connect_async(&self.url)
            .await
            .map_err(|e| {
                println!("[WS:CLIENT] Connection failed: {}", e);
                WebSocketError::ConnectionFailed(format!("Failed to connect: {}", e))
            })?;

        println!("[WS:CLIENT] Connected to {}", self.url);

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Send authentication message
        println!("[WS:CLIENT] Sending authentication message");
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
        println!("[WS:CLIENT] Waiting for authentication response");
        let auth_timeout = tokio::time::timeout(
            tokio::time::Duration::from_secs(10),
            ws_receiver.next()
        ).await;

        let auth_response = match auth_timeout {
            Ok(Some(Ok(Message::Text(text)))) => {
                println!("[WS:CLIENT] Received auth response: {}", text);
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
            
            // Crea channel per messaggi in uscita
            let (outgoing_tx, mut outgoing_rx) = mpsc::unbounded_channel::<OutgoingChatMessage>();
            
            // Avvia il loop di gestione messaggi in background
            if let Some(sender) = &self.message_sender {
                let sender_clone = sender.clone();
                
                // Spawn task per gestire messaggi in arrivo
                tokio::spawn(async move {
                    Self::handle_incoming_messages(ws_receiver, sender_clone).await;
                });
                
                // Spawn task per gestire messaggi in uscita
                tokio::spawn(async move {
                    println!("[WS:CLIENT] Starting outgoing message handler");
                    while let Some(outgoing_msg) = outgoing_rx.recv().await {
                        println!("[WS:CLIENT] Received outgoing message: {:?}", outgoing_msg.message_type);
                        match serde_json::to_string(&outgoing_msg) {
                            Ok(json) => {
                                println!("[WS:CLIENT] Sending JSON: {}", json);
                                if let Err(e) = ws_sender.send(Message::Text(json)).await {
                                    println!("[WS:CLIENT] Failed to send message: {}", e);
                                    break;
                                }
                                println!("[WS:CLIENT] Message sent successfully");
                            }
                            Err(e) => {
                                println!("[WS:CLIENT] Failed to serialize outgoing message: {}", e);
                            }
                        }
                    }
                    println!("[WS:CLIENT] Outgoing message handler ended");
                });
            }
            
            Ok(outgoing_tx)
        } else {
            let error_msg = auth_response.error.unwrap_or_else(|| "Unknown authentication error".to_string());
            println!("[WS:CLIENT] Authentication failed: {}", error_msg);
            Err(WebSocketError::AuthenticationFailed(error_msg))
        }
    }

   




    /// Gestisce i messaggi in arrivo dal WebSocket in background
    async fn handle_incoming_messages(
        mut ws_receiver: futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>,
        sender: mpsc::UnboundedSender<WebSocketMessage>
    ) {
        println!("[WS:CLIENT] Starting incoming message handler");
        while let Some(message) = ws_receiver.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    println!("[WS:CLIENT] Received message: {}", text);
                    match Self::parse_websocket_message(&text) {
                        Ok(ws_msg) => {
                            if let Err(_) = sender.send(ws_msg) {
                                println!("[WS:CLIENT] Failed to send message to application - receiver dropped");
                                break;
                            }
                        }
                        Err(e) => {
                            println!("[WS:CLIENT] Failed to parse message: {} - Raw: {}", e, text);
                            let _ = sender.send(WebSocketMessage::Error(format!("Parse error: {}", e)));
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    println!("[WS:CLIENT] WebSocket connection closed by server");
                    let _ = sender.send(WebSocketMessage::Error("Connection closed".to_string()));
                    break;
                }
                Ok(_) => {
                    // Ignora altri tipi di messaggio (binary, ping, pong)
                }
                Err(e) => {
                    println!("[WS:CLIENT] WebSocket error: {}", e);
                    let _ = sender.send(WebSocketMessage::Error(format!("WebSocket error: {}", e)));
                    break;
                }
            }
        }
        println!("[WS:CLIENT] Message handling loop ended");
    }

    /// Parsa un messaggio JSON dal WebSocket
    fn parse_websocket_message(text: &str) -> Result<WebSocketMessage, String> {
        // Prima prova a parsare come messaggio generico per ottenere il tipo
        let generic: serde_json::Value = serde_json::from_str(text)
            .map_err(|e| format!("Invalid JSON: {}", e))?;
        
        let message_type = generic.get("message_type")
            .and_then(|v| v.as_str())
            .ok_or("Missing message_type field")?;

        match message_type {
            "new_message" => {
                let chat_msg: IncomingChatMessage = serde_json::from_str(text)
                    .map_err(|e| format!("Failed to parse new_message: {}", e))?;
                Ok(WebSocketMessage::NewMessage(chat_msg))
            }
            "user_status" => {
                let user_id = generic.get("user_id")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing user_id in user_status message")?
                    .to_string();
                let online = generic.get("online")
                    .and_then(|v| v.as_bool())
                    .ok_or("Missing online field in user_status message")?;
                Ok(WebSocketMessage::UserStatusUpdate { user_id, online })
            }
            _ => {
                Err(format!("Unknown message type: {}", message_type))
            }
        }
    
    }

    pub fn reset_retry_delay(&mut self) {
        self.retry_delay = tokio::time::Duration::from_secs(2);
    }

    pub fn get_retry_attempts(&self) -> u32 {
        self.connection_retry_attempts
    }

    /// Invia un messaggio privato tramite WebSocket
    pub async fn send_private_message(&self, to_user: &str, content: &str) -> Result<(), WebSocketError> {
        println!("[WS:CLIENT] send_private_message called for user: {}, content: {}", to_user, content);
        
        let session_token = self.session_token.as_ref()
            .ok_or_else(|| WebSocketError::MessageSendFailed("No session token available".to_string()))?;

        let message = OutgoingChatMessage {
            message_type: "send_message".to_string(),
            chat_type: "private".to_string(),
            to_user: Some(to_user.to_string()),
            group_id: None,
            content: content.to_string(),
            session_token: session_token.clone(),
        };

        if let Some(sender) = &self.outgoing_sender {
            println!("[WS:CLIENT] Attempting to send message via WebSocket channel");
            match sender.send(message) {
                Ok(_) => {
                    println!("[WS:CLIENT] Message successfully queued for sending");
                    Ok(())
                }
                Err(_) => {
                    println!("[WS:CLIENT] ERROR: Failed to queue message - channel receiver dropped!");
                    Err(WebSocketError::MessageSendFailed("Failed to queue message for sending - receiver dropped".to_string()))
                }
            }
        } else {
            println!("[WS:CLIENT] ERROR: WebSocket not connected - no outgoing_sender");
            Err(WebSocketError::MessageSendFailed("WebSocket not connected".to_string()))
        }
    }

    /// Invia un messaggio di gruppo tramite WebSocket
    pub async fn send_group_message(&self, group_id: &str, content: &str) -> Result<(), WebSocketError> {
        let session_token = self.session_token.as_ref()
            .ok_or_else(|| WebSocketError::MessageSendFailed("No session token available".to_string()))?;

        let message = OutgoingChatMessage {
            message_type: "send_message".to_string(),
            chat_type: "group".to_string(),
            to_user: None,
            group_id: Some(group_id.to_string()),
            content: content.to_string(),
            session_token: session_token.clone(),
        };

        if let Some(sender) = &self.outgoing_sender {
            sender.send(message)
                .map_err(|_| WebSocketError::MessageSendFailed("Failed to queue message for sending".to_string()))?;
            Ok(())
        } else {
            Err(WebSocketError::MessageSendFailed("WebSocket not connected".to_string()))
        }
    }

    /// Controlla se il WebSocket è connesso e pronto per inviare messaggi
    pub fn is_connected(&self) -> bool {
        self.outgoing_sender.is_some()
    }
}
