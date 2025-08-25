use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, timeout};
use crate::client::services::message_parser;

#[derive(Debug)]
enum CommandType {
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
}

impl ChatService {
    pub fn new() -> Self {
        Self { tx: None, _bg: None }
    }
    
    /// Reset the service by dropping existing connections and background tasks
    pub fn reset(&mut self) {
        self.tx = None;
        self._bg = None;
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

                // Log outgoing
                if cmd.starts_with("/logout") {
                    println!("[CLIENT:SVC] Sending logout command (redacted)");
                } else {
                    println!("[CLIENT:SVC] Sending command: {}", cmd);
                }

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
    /// Send a private message using the existing send_command implementation.
    /// Returns the raw server response.
    pub async fn send_private_message(&mut self, host: &str, session_token: &str, to: &str, msg: &str) -> anyhow::Result<String> {
    // server expects /send_private_message
    let cmd = format!("/send_private_message {} {} {}", session_token, to, msg);
        let resp = self.send_command(host, cmd).await?;
        Ok(resp)
    }

    /// Retrieve private messages with another user and return them parsed as Vec<String>.
    pub async fn get_private_messages(&mut self, host: &str, session_token: &str, with: &str) -> anyhow::Result<Vec<crate::client::models::app_state::ChatMessage>> {
        let cmd = format!("/get_private_messages {} {}", session_token, with);
        let resp = self.send_multiline_command(host, cmd).await?;
        let msgs = message_parser::parse_private_messages(&resp).map_err(|e| anyhow::anyhow!(e))?;
        Ok(msgs)
    }

    /// Send a group message using the existing send_command implementation.
    /// Returns the raw server response.
    pub async fn send_group_message(&mut self, host: &str, session_token: &str, group_id: &str, msg: &str) -> anyhow::Result<String> {
        // server expects /send_group_message
        let cmd = format!("/send_group_message {} {} {}", session_token, group_id, msg);
        let resp = self.send_command(host, cmd).await?;
        Ok(resp)
    }
}



impl ChatService {
    /// Retrieve group messages and return them parsed as Vec<ChatMessage>.
    pub async fn get_group_messages(&mut self, host: &str, session_token: &str, group_id: &str) -> anyhow::Result<Vec<crate::client::models::app_state::ChatMessage>> {
        let cmd = format!("/get_group_messages {} {}", session_token, group_id);
        let resp = self.send_multiline_command(host, cmd).await?;
        let msgs = message_parser::parse_group_messages(&resp).map_err(|e| anyhow::anyhow!(e))?;
        Ok(msgs)
    }
}