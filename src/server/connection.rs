use crate::server::{database::Database, auth, users, groups, messages};
use sqlx::Row;
use crate::server::config::ServerConfig;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use std::fs::File;
use std::io::BufReader as StdBufReader;

// Optional TLS
use tokio_rustls::TlsAcceptor;
use rustls::{Certificate, PrivateKey, ServerConfig as RustlsConfig};
use rustls_pemfile::{certs, rsa_private_keys, pkcs8_private_keys};

pub struct Server {
    pub db: Arc<Database>,
    pub config: ServerConfig,
}

impl Server {
    pub async fn run(&self, addr: &str) -> anyhow::Result<()> {
        let listener = TcpListener::bind(addr).await?;
        println!("[SERVER] Listening on {}", addr);

        // Setup TLS acceptor if enabled and cert files are present
        let tls_acceptor = if self.config.enable_encryption {
            // Expect env variables TLS_CERT_PATH and TLS_KEY_PATH
            if let (Ok(cert_path), Ok(key_path)) = (std::env::var("TLS_CERT_PATH"), std::env::var("TLS_KEY_PATH")) {
                let cert_file = File::open(&cert_path)?;
                let mut cert_reader = StdBufReader::new(cert_file);
                let cert_chain = certs(&mut cert_reader)?.into_iter().map(Certificate).collect::<Vec<_>>();

                let key_file = File::open(&key_path)?;
                let mut key_reader = StdBufReader::new(key_file);
                // Support pkcs8 or rsa keys
                let mut keys = pkcs8_private_keys(&mut key_reader)?;
                if keys.is_empty() {
                    // retry rsa
                    let key_file = File::open(&key_path)?;
                    let mut key_reader = StdBufReader::new(key_file);
                    keys = rsa_private_keys(&mut key_reader)?;
                }
                if keys.is_empty() {
                    println!("[SERVER] TLS enabled but no private keys found in {}", key_path);
                    None
                } else {
                    let priv_key = PrivateKey(keys.remove(0));
                    let mut rustls_cfg = RustlsConfig::builder()
                        .with_safe_defaults()
                        .with_no_client_auth()
                        .with_single_cert(cert_chain, priv_key)
                        .map_err(|e| anyhow::anyhow!(e))?;
                    rustls_cfg.alpn_protocols = vec![b"ruggine".to_vec()];
                    Some(TlsAcceptor::from(std::sync::Arc::new(rustls_cfg)))
                }
            } else { None }
        } else { None };

        loop {
            let (stream, peer) = listener.accept().await?;
            println!("[SERVER] New connection from {}", peer);
            let db = self.db.clone();
            let config = self.config.clone();
            let acceptor = tls_acceptor.clone();
            tokio::spawn(async move {
                // If TLS is configured, try to accept TLS, otherwise use plain TCP
                if let Some(acceptor) = acceptor {
                    match acceptor.accept(stream).await {
                        Ok(tls_stream) => {
                            if let Err(e) = handle_tls_client(db, config, tls_stream).await {
                                println!("[SERVER] Client error (tls): {}", e);
                            }
                        }
                        Err(e) => println!("[SERVER] TLS accept failed: {}", e),
                    }
                } else {
                    if let Err(e) = handle_client(db, config, stream).await {
                        println!("[SERVER] Client error: {}", e);
                    }
                }
            });
        }
    }

    pub async fn handle_command(&self, cmd: &str, args: &[&str]) -> String {
        println!("[SERVER] Received command: {} {:?}", cmd, args);
        match cmd {
            // FRIENDSHIP SYSTEM
            "/send_friend_request" if args.len() >= 2 => {
                let session_token = args[0];
                let to_username = args[1];
                let message = if args.len() > 2 { args[2..].join(" ") } else { "".to_string() };
                if let Some(uid) = auth::validate_session(self.db.clone(), session_token).await {
                    users::send_friend_request(self.db.clone(), &uid, to_username, &message).await
                } else {
                    "ERR: Invalid or expired session".to_string()
                }
            }
            "/accept_friend_request" if args.len() == 2 => {
                let session_token = args[0];
                let from_username = args[1];
                if let Some(uid) = auth::validate_session(self.db.clone(), session_token).await {
                    users::accept_friend_request(self.db.clone(), &uid, from_username).await
                } else {
                    "ERR: Invalid or expired session".to_string()
                }
            }
            "/reject_friend_request" if args.len() == 2 => {
                let session_token = args[0];
                let from_username = args[1];
                if let Some(uid) = auth::validate_session(self.db.clone(), session_token).await {
                    users::reject_friend_request(self.db.clone(), &uid, from_username).await
                } else {
                    "ERR: Invalid or expired session".to_string()
                }
            }
            "/list_friends" if args.len() == 1 => {
                let session_token = args[0];
                if let Some(uid) = auth::validate_session(self.db.clone(), session_token).await {
                    users::list_friends(self.db.clone(), &uid).await
                } else {
                    "ERR: Invalid or expired session".to_string()
                }
            }
            "/received_friend_requests" if args.len() == 1 => {
                let session_token = args[0];
                if let Some(uid) = auth::validate_session(self.db.clone(), session_token).await {
                    users::received_friend_requests(self.db.clone(), &uid).await
                } else {
                    "ERR: Invalid or expired session".to_string()
                }
            }
            "/sent_friend_requests" if args.len() == 1 => {
                let session_token = args[0];
                if let Some(uid) = auth::validate_session(self.db.clone(), session_token).await {
                    users::sent_friend_requests(self.db.clone(), &uid).await
                } else {
                    "ERR: Invalid or expired session".to_string()
                }
            }
            // SYSTEM
            "/help" => {
                users::help().await
            }
            "/quit" => {
                "OK: Disconnected".to_string()
            }
            "/logout" if args.len() == 1 => {
                // args[0] = session_token
                auth::logout(self.db.clone(), args[0]).await
            }
            "/validate_session" if args.len() == 1 => {
                let session_token = args[0];
                if let Some(uid) = auth::validate_session(self.db.clone(), session_token).await {
                    // Recupera username
                    let row = sqlx::query("SELECT username FROM users WHERE id = ?")
                        .bind(&uid)
                        .fetch_optional(&self.db.pool)
                        .await;
                    if let Ok(Some(r)) = row {
                        let username: String = r.get("username");
                        format!("OK: {}", username)
                    } else {
                        "ERR: User not found".to_string()
                    }
                } else {
                    "ERR: Invalid or expired session".to_string()
                }
            }
            "/register" if args.len() == 2 => {
                auth::register(self.db.clone(), args[0], args[1], &self.config).await
            }
            "/login" if args.len() == 2 => {
                auth::login(self.db.clone(), args[0], args[1], &self.config).await
            }
            "/users" => {
                users::list_online(self.db.clone()).await
            }
            "/all_users" => {
                let exclude = None;
                users::list_all(self.db.clone(), exclude).await
            }
            "/create_group" if args.len() == 2 => {
                let session_token = args[0];
                if let Some(uid) = auth::validate_session(self.db.clone(), session_token).await {
                    groups::create_group(self.db.clone(), &uid, args[1]).await
                } else {
                    "ERR: Invalid or expired session".to_string()
                }
            }
            "/my_groups" if args.len() == 1 => {
                let session_token = args[0];
                if let Some(uid) = auth::validate_session(self.db.clone(), session_token).await {
                    groups::my_groups(self.db.clone(), &uid).await
                } else {
                    "ERR: Invalid or expired session".to_string()
                }
            }
            "/invite" if args.len() == 3 => {
                let session_token = args[0];
                if let Some(uid) = auth::validate_session(self.db.clone(), session_token).await {
                    groups::invite(self.db.clone(), &uid, args[1], args[2]).await
                } else {
                    "ERR: Invalid or expired session".to_string()
                }
            }
            "/accept_invite" if args.len() == 2 => {
                let session_token = args[0];
                if let Some(uid) = auth::validate_session(self.db.clone(), session_token).await {
                    groups::accept_invite(self.db.clone(), &uid, args[1]).await
                } else {
                    "ERR: Invalid or expired session".to_string()
                }
            }
            "/reject_invite" if args.len() == 2 => {
                let session_token = args[0];
                if let Some(uid) = auth::validate_session(self.db.clone(), session_token).await {
                    groups::reject_invite(self.db.clone(), &uid, args[1]).await
                } else {
                    "ERR: Invalid or expired session".to_string()
                }
            }
            "/my_invites" if args.len() == 1 => {
                let session_token = args[0];
                if let Some(uid) = auth::validate_session(self.db.clone(), session_token).await {
                    groups::my_invites(self.db.clone(), &uid).await
                } else {
                    "ERR: Invalid or expired session".to_string()
                }
            }
            "/join_group" if args.len() == 2 => {
                let session_token = args[0];
                if let Some(uid) = auth::validate_session(self.db.clone(), session_token).await {
                    groups::join_group(self.db.clone(), &uid, args[1]).await
                } else {
                    "ERR: Invalid or expired session".to_string()
                }
            }
            "/leave_group" if args.len() == 2 => {
                let session_token = args[0];
                if let Some(uid) = auth::validate_session(self.db.clone(), session_token).await {
                    groups::leave_group(self.db.clone(), &uid, args[1]).await
                } else {
                    "ERR: Invalid or expired session".to_string()
                }
            }
            // MESSAGGI
            "/send" if args.len() >= 3 => {
                let session_token = args[0];
                let group_name = args[1];
                let message = &args[2..].join(" ");
                messages::send_group_message(self.db.clone(), session_token, group_name, message, &self.config).await
            }
            "/send_private" | "/private" if args.len() >= 3 => {
                let session_token = args[0];
                let to_username = args[1];
                let message = &args[2..].join(" ");
                messages::send_private_message(self.db.clone(), session_token, to_username, message, &self.config).await
            }
            "/get_group_messages" if args.len() == 2 => {
                let session_token = args[0];
                let group_name = args[1];
                messages::get_group_messages(self.db.clone(), session_token, group_name).await
            }
            "/get_private_messages" if args.len() == 2 => {
                let session_token = args[0];
                let other_username = args[1];
                messages::get_private_messages(self.db.clone(), session_token, other_username).await
            }
            "/delete_group_messages" if args.len() == 2 => {
                let session_token = args[0];
                let group_id = args[1];
                messages::delete_group_messages(self.db.clone(), session_token, group_id).await
            }
            "/delete_private_messages" if args.len() == 2 => {
                let session_token = args[0];
                let other_username = args[1];
                messages::delete_private_messages(self.db.clone(), session_token, other_username).await
            }
            _ => "ERR: Unknown or invalid command".to_string(),
        }
    }
}

async fn handle_client(db: Arc<Database>, config: ServerConfig, stream: TcpStream) -> anyhow::Result<()> {
    let (reader, writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            println!("[SERVER] Client disconnected");
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        let mut parts = trimmed.split_whitespace();
        let cmd = parts.next().unwrap_or("");
        let args: Vec<&str> = parts.collect();
        let server = Server { db: db.clone(), config: config.clone() };
        let response = server.handle_command(cmd, &args).await;
        writer.write_all(response.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }
    Ok(())
}

// TLS stream handling: keep the same protocol logic but using the TLS stream types
async fn handle_tls_client<S>(db: Arc<Database>, config: ServerConfig, stream: S) -> anyhow::Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let (reader, writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            println!("[SERVER] Client disconnected");
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        let mut parts = trimmed.split_whitespace();
        let cmd = parts.next().unwrap_or("");
        let args: Vec<&str> = parts.collect();
        let server = Server { db: db.clone(), config: config.clone() };
        let response = server.handle_command(cmd, &args).await;
        writer.write_all(response.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }
    Ok(())
}
