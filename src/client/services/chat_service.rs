use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, Duration};
use std::sync::Arc;

#[derive(Default)]
pub struct ChatService {
    /// Sender used by the app to request the background task to send a command and
    /// wait for a single-line response.
    pub tx: Option<mpsc::UnboundedSender<(String, oneshot::Sender<String>)>>,
    /// Keep the background task handle so it stays alive for the lifetime of the service
    pub _bg: Option<tokio::task::JoinHandle<()>>,
}

impl ChatService {
    pub fn new() -> Self {
        Self { tx: None, _bg: None }
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

        let (tx, mut rx) = mpsc::unbounded_channel::<(String, oneshot::Sender<String>)>();

        // Spawn background task that processes outgoing requests sequentially.
        // The task will transparently reconnect and resend the current command
        // if the connection is closed by the server (for example after logout).
        let handle = tokio::spawn(async move {
            let mut server_line = String::new();
            // current reader/writer are in scope and may be replaced on reconnect
            loop {
                // Wait for the next outgoing command. If channel closed, exit cleanly.
                let (cmd, resp_tx) = match rx.recv().await {
                    Some(pair) => pair,
                    None => break,
                };

                // Log outgoing
                if cmd.starts_with("/logout") {
                    println!("[CLIENT:SVC] Sending logout command (redacted)");
                } else {
                    println!("[CLIENT:SVC] Sending command: {}", cmd);
                }

                // Attempt to send this command and receive a single-line response.
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

                    // Try to read a single-line response.
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
            tx.send((cmd, resp_tx)).map_err(|_| anyhow::anyhow!("send failed: background task ended"))?;
            let resp = resp_rx.await.map_err(|_| anyhow::anyhow!("response channel closed before response"))?;
            Ok(resp)
        } else {
            Err(anyhow::anyhow!("not connected"))
        }
    }

    // Placeholder methods for later
    pub fn send_private_message(&mut self, _to: &str, _msg: &str) {
        // TODO
    }
    pub fn get_private_messages(&self, _with: &str) -> Vec<String> {
        vec![]
    }
}
