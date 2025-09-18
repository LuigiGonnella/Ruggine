use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use url::Url;

// Re-export the WebSocket message types from server for client use
pub use crate::server::websocket::{WebSocketMessage, MessageType};

#[derive(Debug, Clone)]
pub struct WebSocketService {
    sender: Arc<Mutex<Option<mpsc::UnboundedSender<WebSocketMessage>>>>,
    receiver: Arc<Mutex<Option<mpsc::UnboundedReceiver<WebSocketMessage>>>>,
    is_connected: Arc<Mutex<bool>>,
}

impl WebSocketService {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            sender: Arc::new(Mutex::new(Some(tx))),
            receiver: Arc::new(Mutex::new(Some(rx))),
            is_connected: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn connect(&self, ws_url: &str, _user_id: String) -> anyhow::Result<()> {
        let url = Url::parse(ws_url)?;
        let (ws_stream, _) = connect_async(url).await?;
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        *self.is_connected.lock().await = true;

        let sender_clone = self.sender.clone();
        let _receiver_clone = self.receiver.clone();
        let is_connected_clone = self.is_connected.clone();

        // Channel per comunicazione interna
        let (internal_tx, mut internal_rx) = mpsc::unbounded_channel::<WebSocketMessage>();
        
        // Sostituisci il sender con quello interno
        {
            let mut sender_guard = sender_clone.lock().await;
            *sender_guard = Some(internal_tx);
        }

        // Task per inviare messaggi al server WebSocket
        let send_task = tokio::spawn(async move {
            while let Some(message) = internal_rx.recv().await {
                let json_message = match serde_json::to_string(&message) {
                    Ok(json) => json,
                    Err(_) => continue,
                };
                
                if ws_sender.send(Message::Text(json_message)).await.is_err() {
                    break;
                }
            }
        });

        // Task per ricevere messaggi dal server WebSocket
        let receive_task = tokio::spawn(async move {
            while let Some(message) = ws_receiver.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        if let Ok(ws_message) = serde_json::from_str::<WebSocketMessage>(&text) {
                            // Invece di usare il receiver come sender, salviamo il messaggio in un buffer
                            // Per ora, stampiamo il messaggio ricevuto
                            println!("[WS:CLIENT] Received message: {:?}", ws_message);
                            
                        }
                    }
                    Ok(Message::Close(_)) => {
                        *is_connected_clone.lock().await = false;
                        break;
                    }
                    Err(_) => {
                        *is_connected_clone.lock().await = false;
                        break;
                    }
                    _ => {}
                }
            }
        });

        // Avvia entrambi i task
        tokio::spawn(async move {
            tokio::select! {
                _ = send_task => {},
                _ = receive_task => {},
            }
        });

        Ok(())
    }

    pub async fn send_private_message(&self, to: &str, content: &str) -> anyhow::Result<()> {
        // For now, we don't have the sender info in this context
        // This should be set when the user authenticates
        let sender = "unknown"; 
        
        let message = WebSocketMessage {
            id: uuid::Uuid::new_v4().to_string(),
            message_type: MessageType::PrivateMessage,
            sender: sender.to_string(),
            target: to.to_string(),
            content: content.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        };

        self.send_message(message).await
    }

    pub async fn send_group_message(&self, group_id: &str, content: &str) -> anyhow::Result<()> {
        // For now, we don't have the sender info in this context
        let sender = "unknown"; 
        
        let message = WebSocketMessage {
            id: uuid::Uuid::new_v4().to_string(),
            message_type: MessageType::GroupMessage,
            sender: sender.to_string(),
            target: group_id.to_string(),
            content: content.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        };

        self.send_message(message).await
    }

    /// Receive multiple messages non-blocking (for polling)
    pub async fn receive_messages(&self) -> anyhow::Result<Vec<WebSocketMessage>> {
        let mut messages = Vec::new();
        
        // Try to receive multiple messages without blocking
        if let Some(ref mut receiver) = *self.receiver.lock().await {
            while let Ok(message) = receiver.try_recv() {
                messages.push(message);
            }
        }
        
        Ok(messages)
    }

    async fn send_message(&self, message: WebSocketMessage) -> anyhow::Result<()> {
        if let Some(ref sender) = *self.sender.lock().await {
            sender.send(message)?;
        }
        Ok(())
    }

    pub async fn receive_message(&self) -> Option<WebSocketMessage> {
        if let Some(ref mut receiver) = *self.receiver.lock().await {
            receiver.recv().await
        } else {
            None
        }
    }

    pub async fn is_connected(&self) -> bool {
        *self.is_connected.lock().await
    }

    pub async fn disconnect(&self) {
        *self.is_connected.lock().await = false;
        // I task si chiuderanno automaticamente quando il WebSocket si disconnette
    }
}

impl Default for WebSocketService {
    fn default() -> Self {
        Self::new()
    }
}
