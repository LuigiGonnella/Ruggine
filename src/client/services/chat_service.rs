use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, timeout};
use crate::client::services::message_parser;
use crate::client::services::websocket_client::{WebSocketClient, WebSocketMessage};

#[derive(Debug)]
pub enum CommandType {
    SingleLine(String),
    MultiLine(String),
}

#[derive(Default)]
pub struct ChatService {
    /// Sender used by the app to request the background task to send a command and
    /// wait for a response.
    pub tx: Option<mpsc::UnboundedSender<(CommandType, oneshot::Sender<String>)>>,
    /// Keep the background task handle so it stays alive for the lifetime of the service
    pub _bg: Option<tokio::task::JoinHandle<()>>,
    /// WebSocket client for real-time messaging
    pub websocket: Option<WebSocketClient>,
    /// Current user information
    pub current_user: Option<String>,
    /// Receiver per messaggi WebSocket
    pub websocket_receiver: Option<mpsc::UnboundedReceiver<WebSocketMessage>>,
}

impl ChatService {
    pub fn new() -> Self {
        Self { 
            tx: None, 
            _bg: None, 
            websocket: None,
            current_user: None,
            websocket_receiver: None,
        }
    }
    
    /// Reset the service by dropping existing connections and background tasks
    pub async fn reset(&mut self) {
        println!("[CHAT_SERVICE] 🔄 Resetting ChatService - dropping all connections");
        self.tx = None;
        self._bg = None;
        self.websocket = None;
        self.current_user = None;
        self.websocket_receiver = None;
        println!("[CHAT_SERVICE] ✅ Reset completed");
    }

    /// Logout from server and reset local state
    pub async fn logout(&mut self, host: &str, session_token: &str) -> anyhow::Result<()> {
        println!("[CHAT_SERVICE] 🚪 Logging out from server");
        
        // Call server logout command first
        let cmd = format!("/logout {}", session_token);
        match self.send_command(host, cmd).await {
            Ok(response) => {
                println!("[CHAT_SERVICE] 🚪 Server logout response: {}", response);
            }
            Err(e) => {
                println!("[CHAT_SERVICE] ⚠️ Server logout failed: {}, continuing with local cleanup", e);
            }
        }
        
        // Then reset local state
        self.reset().await;
        println!("[CHAT_SERVICE] 🚪 Logout completed");
        Ok(())
    }

    /// Initialize WebSocket connection
    pub async fn connect_websocket(&mut self, ws_host: &str, ws_port: u16, session_token: &str) -> anyhow::Result<()> {
        let ws_url = format!("ws://{}:{}", ws_host, ws_port);
        println!("[CHAT_SERVICE] 🔌 Starting WebSocket connection to {}", ws_url);
        
        // Reset any existing WebSocket connection
        self.reset().await;
        
        // Create new WebSocket client
        let mut ws_client = WebSocketClient::new(ws_url.clone());
        ws_client.set_session_token(session_token.to_string());
        
        // Get the receiver before connecting
        self.websocket_receiver = ws_client.take_receiver();
        println!("[CHAT_SERVICE] 📥 WebSocket receiver created");
        
        // Connect and authenticate
        ws_client.connect_with_auth().await.map_err(|e| anyhow::anyhow!("WebSocket connection failed: {}", e))?;
        
        // Store the connected client
        self.websocket = Some(ws_client);
        
        println!("[CHAT_SERVICE] ✅ WebSocket fully connected and authenticated to {}", ws_url);
        Ok(())
    }

    /// Get the next WebSocket message if available
    pub async fn try_receive_websocket_message(&mut self) -> Option<WebSocketMessage> {
        if let Some(ref mut receiver) = self.websocket_receiver {
            match receiver.try_recv() {
                Ok(msg) => {
                    println!("[CHAT_SERVICE] 📩 Received WebSocket message: {:?}", msg);
                    Some(msg)
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                    // No messages available right now - don't log this to reduce spam
                    None
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    println!("[CHAT_SERVICE] ❌ WebSocket receiver disconnected!");
                    None
                }
            }
        } else {
            // No receiver available - WebSocket not connected, don't spam logs
            None
        }
    }

    /// Wait for the next WebSocket message
    pub async fn receive_websocket_message(&mut self) -> Option<WebSocketMessage> {
        if let Some(ref mut receiver) = self.websocket_receiver {
            receiver.recv().await
        } else {
            None
        }
    }

    /// Check if WebSocket is connected
    pub async fn is_websocket_connected(&self) -> bool {
        self.websocket.is_some()
    }

    /// Update the session token of an existing WebSocket connection
    pub async fn update_websocket_token(&mut self, new_token: &str) -> anyhow::Result<()> {
        if let Some(ref mut ws_client) = self.websocket {
            ws_client.set_session_token(new_token.to_string());
            println!("[CHAT_SERVICE] Updated WebSocket session token");
            Ok(())
        } else {
            Err(anyhow::anyhow!("No WebSocket connection to update"))
        }
    }

    /// Ensure there is an active background task connected to `host`.
    pub async fn ensure_connected(&mut self, host: &str) -> anyhow::Result<()> {
        if self.tx.is_some() {
            return Ok(());
        }

        let host = host.to_string();
        let stream = TcpStream::connect(&host).await?;
        let (reader, writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut writer = BufWriter::new(writer);

        let (tx, mut rx) = mpsc::unbounded_channel::<(CommandType, oneshot::Sender<String>)>();

        // Spawn background task that processes outgoing requests sequentially.
        // The task will transparently reconnect and resend the current command
        // if the connection is closed by the server (for example after logout).
        let handle = tokio::spawn(async move {
            let mut server_line = String::new();
            // current reader/writer are in scope and may be replaced on reconnect
            loop {
                // Wait for the next outgoing command. If channel closed, exit cleanly.
                let (cmd_type, resp_tx) = match rx.recv().await {
                    Some(pair) => pair,
                    None => break,
                };

                let (cmd, is_multiline) = match cmd_type {
                    CommandType::SingleLine(cmd) => (cmd, false),
                    CommandType::MultiLine(cmd) => (cmd, true),
                };


                // Attempt to send this command and receive a response.
                // If the connection is dropped at any point, try to reconnect and
                // then resend the same command. This loop keeps retrying until
                // we either get a response or the response sender is dropped.
                loop {
                    // Try writing the command
                    if let Err(e) = writer.write_all(cmd.as_bytes()).await {
                        // write failed -> need to reconnect
                        eprintln!("[CLIENT:SVC] write failed: {}, reconnecting...", e);
                        // perform reconnect
                        match TcpStream::connect(&host).await {
                            Ok(s) => {
                                let (r, w) = s.into_split();
                                reader = BufReader::new(r);
                                writer = BufWriter::new(w);
                                // retry sending
                                continue;
                            }
                            Err(e) => {
                                // Can't reconnect right now; notify caller and drop
                                let _ = resp_tx.send(format!("ERR: reconnect failed: {}", e));
                                break;
                            }
                        }
                    }
                    if let Err(e) = writer.write_all(b"\n").await {
                        eprintln!("[CLIENT:SVC] write newline failed: {}, reconnecting...", e);
                        match TcpStream::connect(&host).await {
                            Ok(s) => {
                                let (r, w) = s.into_split();
                                reader = BufReader::new(r);
                                writer = BufWriter::new(w);
                                continue;
                            }
                            Err(e) => {
                                let _ = resp_tx.send(format!("ERR: reconnect failed: {}", e));
                                break;
                            }
                        }
                    }
                    if let Err(e) = writer.flush().await {
                        eprintln!("[CLIENT:SVC] flush failed: {}, reconnecting...", e);
                        match TcpStream::connect(&host).await {
                            Ok(s) => {
                                let (r, w) = s.into_split();
                                reader = BufReader::new(r);
                                writer = BufWriter::new(w);
                                continue;
                            }
                            Err(e) => {
                                let _ = resp_tx.send(format!("ERR: reconnect failed: {}", e));
                                break;
                            }
                        }
                    }

                    // Try to read the response based on type
                    if is_multiline {
                        // For multiline responses, read all available lines
                        let mut response = String::new();
                        server_line.clear();
                        
                        // Read the first line (should be "OK: Messages:")
                        match reader.read_line(&mut server_line).await {
                            Ok(0) => {
                                // Connection closed by peer. Reconnect and retry.
                                eprintln!("[CLIENT:SVC] server closed connection, reconnecting...");
                                match TcpStream::connect(&host).await {
                                    Ok(s) => {
                                        let (r, w) = s.into_split();
                                        reader = BufReader::new(r);
                                        writer = BufWriter::new(w);
                                        continue;
                                    }
                                    Err(e) => {
                                        let _ = resp_tx.send(format!("ERR: reconnect failed: {}", e));
                                        break;
                                    }
                                }
                            }
                            Ok(_) => {
                                response.push_str(&server_line);
                                
                                // For get_private_messages, read all lines until timeout or empty line
                                loop {
                                    server_line.clear();
                                    
                                    // Use timeout to avoid blocking forever
                                    match timeout(Duration::from_millis(100), reader.read_line(&mut server_line)).await {
                                        Ok(Ok(0)) => {
                                            // Connection closed
                                            break;
                                        }
                                        Ok(Ok(_)) => {
                                            let trimmed = server_line.trim();
                                            if trimmed.is_empty() {
                                                // Empty line indicates end of response
                                                break;
                                            }
                                            response.push_str(&server_line);
                                        }
                                        Ok(Err(e)) => {
                                            eprintln!("[CLIENT:SVC] read failed during multiline: {}", e);
                                            break;
                                        }
                                        Err(_) => {
                                            // Timeout - assume end of response
                                            break;
                                        }
                                    }
                                }
                                
                                let _ = resp_tx.send(response.trim().to_string());
                                break;
                            }
                            Err(e) => {
                                eprintln!("[CLIENT:SVC] read failed: {}, reconnecting...", e);
                                match TcpStream::connect(&host).await {
                                    Ok(s) => {
                                        let (r, w) = s.into_split();
                                        reader = BufReader::new(r);
                                        writer = BufWriter::new(w);
                                        continue;
                                    }
                                    Err(e) => {
                                        let _ = resp_tx.send(format!("ERR: reconnect failed: {}", e));
                                        break;
                                    }
                                }
                            }
                        }
                    } else {
                        // Single line response (existing logic)
                        server_line.clear();
                        match reader.read_line(&mut server_line).await {
                            Ok(0) => {
                                // Connection closed by peer. Reconnect and retry sending the same command.
                                eprintln!("[CLIENT:SVC] server closed connection, reconnecting...");
                                match TcpStream::connect(&host).await {
                                    Ok(s) => {
                                        let (r, w) = s.into_split();
                                        reader = BufReader::new(r);
                                        writer = BufWriter::new(w);
                                        // retry send/receive loop
                                        continue;
                                    }
                                    Err(e) => {
                                        let _ = resp_tx.send(format!("ERR: reconnect failed: {}", e));
                                        break;
                                    }
                                }
                            }
                            Ok(_) => {
                                let resp = server_line.trim().to_string();
                                let _ = resp_tx.send(resp);
                                break;
                            }
                            Err(e) => {
                                eprintln!("[CLIENT:SVC] read failed: {}, reconnecting...", e);
                                match TcpStream::connect(&host).await {
                                    Ok(s) => {
                                        let (r, w) = s.into_split();
                                        reader = BufReader::new(r);
                                        writer = BufWriter::new(w);
                                        continue;
                                    }
                                    Err(e) => {
                                        let _ = resp_tx.send(format!("ERR: reconnect failed: {}", e));
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                // finished handling this command (either responded or failed)
            }
        });

        self.tx = Some(tx);
        self._bg = Some(handle);
        Ok(())
    }

    /// Send a command and wait for the single-line response from the server.
    pub async fn send_command(&mut self, host: &str, cmd: String) -> anyhow::Result<String> {
        // Ensure background task is running; it will manage reconnects and resends.
        self.ensure_connected(host).await?;
        if let Some(tx) = &self.tx {
            let (resp_tx, resp_rx) = oneshot::channel();
            tx.send((CommandType::SingleLine(cmd), resp_tx)).map_err(|_| anyhow::anyhow!("send failed: background task ended"))?;
            let resp = resp_rx.await.map_err(|_| anyhow::anyhow!("response channel closed before response"))?;
            Ok(resp)
        } else {
            Err(anyhow::anyhow!("not connected"))
        }
    }

    /// Send a command and wait for the multi-line response from the server.
    pub async fn send_multiline_command(&mut self, host: &str, cmd: String) -> anyhow::Result<String> {
        // Ensure background task is running; it will manage reconnects and resends.
        self.ensure_connected(host).await?;
        if let Some(tx) = &self.tx {
            let (resp_tx, resp_rx) = oneshot::channel();
            tx.send((CommandType::MultiLine(cmd), resp_tx)).map_err(|_| anyhow::anyhow!("send failed: background task ended"))?;
            let resp = resp_rx.await.map_err(|_| anyhow::anyhow!("response channel closed before response"))?;
            Ok(resp)
        } else {
            Err(anyhow::anyhow!("not connected"))
        }
    }

    // Placeholder methods for later
    /// Send a private message using WebSocket if available, fallback to TCP.
    /// Returns the raw server response.
    pub async fn send_private_message(&mut self, host: &str, session_token: &str, to: &str, msg: &str) -> anyhow::Result<String> {
        // Try WebSocket first if connected
        if let Some(ref websocket) = self.websocket {
            if websocket.is_connected() {
                match websocket.send_private_message(to, msg).await {
                    Ok(()) => {
                        println!("[CHAT_SERVICE] Message sent via WebSocket to {}", to);
                        return Ok("OK: Message sent via WebSocket".to_string());
                    }
                    Err(e) => {
                        println!("[CHAT_SERVICE] WebSocket send failed: {}, falling back to TCP", e);
                        // Fall through to TCP
                    }
                }
            }
        }
        
        // Fallback to TCP
        let cmd = format!("/send_private_message {} {} {}", session_token, to, msg);
        let resp = self.send_command(host, cmd).await?;
        Ok(resp)
    }

    /// Retrieve private messages with another user and return them parsed as Vec<String>.
    pub async fn get_private_messages(&mut self, host: &str, session_token: &str, with: &str) -> anyhow::Result<Vec<crate::client::models::app_state::ChatMessage>> {
        let cmd = format!("/get_private_messages {} {}", session_token, with);
        let resp = self.send_multiline_command(host, cmd).await?;
        
        println!("[CHAT_SERVICE] Raw response: {}", resp);
        
        // For private messages, participants are current user and the other user
        let participants = if let Some(current_user) = &self.current_user {
            vec![current_user.clone(), with.to_string()]
        } else {
            vec![with.to_string()]
        };
        
        let msgs = message_parser::parse_private_messages_with_participants(&resp, &participants)
            .map_err(|e| anyhow::anyhow!(e))?;
        
        println!("[CHAT_SERVICE] Parsed {} messages", msgs.len());
        for (i, msg) in msgs.iter().enumerate() {
            println!("[CHAT_SERVICE] Message {}: {} -> {}", i, msg.sender, msg.content);
        }
        
        Ok(msgs)
    }

    /// Send a group message using WebSocket if available, fallback to TCP.
    /// Returns the raw server response.
    pub async fn send_group_message(&mut self, host: &str, session_token: &str, group_id: &str, msg: &str) -> anyhow::Result<String> {
        // Try WebSocket first if connected
        if let Some(ref websocket) = self.websocket {
            if websocket.is_connected() {
                match websocket.send_group_message(group_id, msg).await {
                    Ok(()) => {
                        println!("[CHAT_SERVICE] Group message sent via WebSocket to group {}", group_id);
                        return Ok("OK: Message sent via WebSocket".to_string());
                    }
                    Err(e) => {
                        println!("[CHAT_SERVICE] WebSocket group send failed: {}, falling back to TCP", e);
                        // Fall through to TCP
                    }
                }
            }
        }
        
        // Fallback to TCP
        let cmd = format!("/send_group_message {} {} {}", session_token, group_id, msg);
        let resp = self.send_command(host, cmd).await?;
        Ok(resp)
    }

    /// Check for new messages via WebSocket (non-blocking)
    /// Returns messages if available, empty vector otherwise
    pub async fn poll_websocket_messages(&mut self) -> Vec<crate::client::models::app_state::ChatMessage> {
        // This method is now obsolete - messages are handled through the WebSocket receiver
        // in real-time via try_receive_websocket_message() and receive_websocket_message()
        // Keep for backward compatibility but return empty vector
        Vec::new()
    }

    /// Get the current user (needed for WebSocket message processing)
    pub fn get_current_user(&self) -> Option<&String> {
        self.current_user.as_ref()
    }

    /// Set the current user (call after successful login)
    pub fn set_current_user(&mut self, username: String) {
        self.current_user = Some(username);
    }

    /// Get group members for proper message decryption
    pub async fn get_group_members(&mut self, host: &str, session_token: &str, group_id: &str) -> anyhow::Result<Vec<String>> {
        let cmd = format!("/group_members {} {}", session_token, group_id);
        let resp = self.send_command(host, cmd).await?;
        
        // Parse response format: "OK: Group members: user1, user2, user3"
        if resp.starts_with("OK: Group members: ") {
            let members_str = resp.strip_prefix("OK: Group members: ").unwrap_or("");
            if members_str.is_empty() {
                Ok(vec![])
            } else {
                let members: Vec<String> = members_str
                    .split(", ")
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                Ok(members)
            }
        } else if resp.starts_with("ERR: Not a group member") {
            // User left the group - return specific error
            Err(anyhow::anyhow!("NOT_A_MEMBER"))
        } else {
            // Other error or unexpected format
            println!("[CHAT_SERVICE] Failed to get group members: {}", resp);
            Err(anyhow::anyhow!("Failed to get group members: {}", resp))
        }
    }
}



impl ChatService {
    /// Retrieve group messages and return them parsed as Vec<ChatMessage>.
    pub async fn get_group_messages(&mut self, host: &str, session_token: &str, group_id: &str) -> anyhow::Result<Vec<crate::client::models::app_state::ChatMessage>> {
        // First get the group members for proper decryption
        let participants = match self.get_group_members(host, session_token, group_id).await {
            Ok(members) => {
                println!("[CHAT_SERVICE] Got {} members for group {}: {:?}", members.len(), group_id, members);
                members
            }
            Err(e) if e.to_string().contains("NOT_A_MEMBER") => {
                // User is no longer a member of this group
                println!("[CHAT_SERVICE] User is not a member of group {}, stopping polling", group_id);
                return Err(anyhow::anyhow!("NOT_A_MEMBER"));
            }
            Err(e) => {
                println!("[CHAT_SERVICE] Failed to get group members for {}: {}, using empty participants", group_id, e);
                vec![]
            }
        };

        // Then get the group messages
        let cmd = format!("/get_group_messages {} {}", session_token, group_id);
        let resp = self.send_multiline_command(host, cmd).await?;
        
        // Check if user is not a member
        if resp.starts_with("ERR: Not a group member") {
            return Err(anyhow::anyhow!("NOT_A_MEMBER"));
        }
        
        // Parse messages with proper participants for decryption
        let msgs = message_parser::parse_group_messages_with_participants(&resp, &participants)
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(msgs)
    }
}