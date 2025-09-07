use crate::server::{database::Database, auth, users, groups, messages, presence::PresenceRegistry};
use sqlx::Row;
use crate::server::config::ServerConfig;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use std::fs::File;
use std::io::BufReader as StdBufReader;

// Optional TLS
use tokio_rustls::TlsAcceptor;
use rustls::{ServerConfig as RustlsConfig};
use rustls_pemfile::{certs, rsa_private_keys, pkcs8_private_keys};

pub struct Server {
    pub db: Arc<Database>,
    pub config: ServerConfig,
    pub presence: PresenceRegistry,
}

impl Server {
    /// Configure TLS acceptor from environment variables
    fn setup_tls_acceptor(&self) -> anyhow::Result<Option<TlsAcceptor>> {
        if !self.config.enable_encryption {
            println!("[TLS] TLS disabled in configuration");
            return Ok(None);
        }

        let cert_path = std::env::var("TLS_CERT_PATH")
            .map_err(|_| anyhow::anyhow!("TLS_CERT_PATH environment variable not set"))?;
        let key_path = std::env::var("TLS_KEY_PATH")
            .map_err(|_| anyhow::anyhow!("TLS_KEY_PATH environment variable not set"))?;

        println!("[TLS] Loading certificate from: {}", cert_path);
        println!("[TLS] Loading private key from: {}", key_path);

        let cert_file = File::open(&cert_path)
            .map_err(|e| anyhow::anyhow!("Failed to open certificate file '{}': {}", cert_path, e))?;
        let mut cert_reader = StdBufReader::new(cert_file);
        let cert_chain = certs(&mut cert_reader)?
            .into_iter()
            .map(|v| rustls::Certificate(v))
            .collect::<Vec<_>>();

        if cert_chain.is_empty() {
            return Err(anyhow::anyhow!("No certificates found in {}", cert_path));
        }
        println!("[TLS] Loaded {} certificate(s)", cert_chain.len());

        let key_file = File::open(&key_path)
            .map_err(|e| anyhow::anyhow!("Failed to open private key file '{}': {}", key_path, e))?;
        let mut key_reader = StdBufReader::new(key_file);
        
        // Try PKCS8 first, then RSA
        let mut keys = pkcs8_private_keys(&mut key_reader)?;
        if keys.is_empty() {
            let key_file = File::open(&key_path)?;
            let mut key_reader = StdBufReader::new(key_file);
            keys = rsa_private_keys(&mut key_reader)?;
        }

        if keys.is_empty() {
            return Err(anyhow::anyhow!("No private keys found in {}", key_path));
        }
        println!("[TLS] Loaded private key");

        let priv_key = rustls::PrivateKey(keys.remove(0));
        let rustls_cfg = RustlsConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert_chain, priv_key)
            .map_err(|e| anyhow::anyhow!("TLS configuration error: {}", e))?;

        println!("[TLS] TLS configuration successful");
        Ok(Some(TlsAcceptor::from(std::sync::Arc::new(rustls_cfg))))
    }

    pub async fn run(&self, addr: &str) -> anyhow::Result<()> {
        let listener = TcpListener::bind(addr).await?;
        println!("[SERVER] Listening on {}", addr);

        // Setup TLS acceptor if enabled
        let tls_acceptor = match self.setup_tls_acceptor() {
            Ok(acceptor) => {
                if acceptor.is_some() {
                    println!("[TLS] TLS enabled and configured successfully");
                } else {
                    println!("[TLS] TLS disabled");
                }
                acceptor
            }
            Err(e) => {
                println!("[TLS] TLS configuration failed: {}", e);
                println!("[TLS] Falling back to plain TCP");
                None
            }
        };

        loop {
            let (stream, peer) = listener.accept().await?;
            println!("[SERVER] New connection from {}", peer);
            let db = self.db.clone();
            let config = self.config.clone();
            let acceptor = tls_acceptor.clone();
            let presence = self.presence.clone();
            tokio::spawn(async move {
                // If TLS is configured, try to accept TLS, otherwise use plain TCP
                if let Some(acceptor) = acceptor {
                    match acceptor.accept(stream).await {
                        Ok(tls_stream) => {
                                    if let Err(e) = handle_tls_client(db, config, tls_stream, peer, presence.clone()).await {
                                        println!("[SERVER] Client error (tls {}) : {}", peer, e);
                                    }
                        }
                        Err(e) => println!("[SERVER] TLS accept failed: {}", e),
                    }
                } else if let Err(e) = handle_client(db, config, stream, peer, presence.clone()).await {
                    println!("[SERVER] Client error ({}): {}", peer, e);
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
                let token = args[0];
                // attempt to resolve user_id first so we can kick presence after logout
                if let Some(uid) = auth::validate_session(self.db.clone(), token).await {
                    println!("[AUTH] Handling /logout for user {} (token masked)", uid);
                    let res = auth::logout(self.db.clone(), token).await;
                    // After logout, query DB to report current sessions count and is_online state for debugging
                    let sess_cnt = sqlx::query("SELECT COUNT(1) as c FROM sessions WHERE user_id = ?")
                        .bind(&uid)
                        .fetch_one(&self.db.pool)
                        .await
                        .ok()
                        .and_then(|r| r.try_get::<i64, _>("c").ok())
                        .unwrap_or(-1);
                    let is_online = sqlx::query("SELECT is_online FROM users WHERE id = ?")
                        .bind(&uid)
                        .fetch_optional(&self.db.pool)
                        .await
                        .ok()
                        .and_then(|opt| opt.map(|r| r.get::<i64, _>("is_online")))
                        .unwrap_or(-1);
                    println!("[AUTH][DB CHECK] after logout: sessions_count={} users.is_online={} for user {}", sess_cnt, is_online, uid);
                    let kicked = self.presence.kick_all(&uid).await;
                    println!("[AUTH] Logout triggered kick for user {} (kicked={})", uid, kicked);
                    res
                } else {
                    // session not valid/expired, still call logout for consistent response
                    println!("[AUTH] /logout called with invalid/expired token (raw token masked)");
                    let res = auth::logout(self.db.clone(), token).await;
                    // Can't resolve uid to run presence.kick_all; return result but also attempt to log token outcome
                    println!("[AUTH] /logout completed for unknown token, result={}", res);
                    res
                }
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
            "/online_users" => {
                users::list_online(self.db.clone()).await
            }
            "/all_users" => {
                let exclude = None;
                users::list_all(self.db.clone(), exclude).await
            }
            "/create_group" if args.len() >= 2 => {
                let session_token = args[0];
                let group_name = args[1];
                let participants = if args.len() > 2 { Some(args[2]) } else { None };
                if let Some(uid) = auth::validate_session(self.db.clone(), session_token).await {
                    groups::create_group_with_participants(self.db.clone(), &uid, group_name, participants).await
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
                let username = args[1];
                let group_id = args[2];
                if let Some(uid) = auth::validate_session(self.db.clone(), session_token).await {
                    groups::invite_user_to_group(self.db.clone(), &uid, username, group_id).await
                } else {
                    "ERR: Invalid or expired session".to_string()
                }
            }
            "/accept_group_invite" if args.len() >= 2 => {
                let session_token = args[0];
                let invite_id = args[1];
                if let Some(uid) = auth::validate_session(self.db.clone(), session_token).await {
                    groups::accept_invite(self.db.clone(), &uid, invite_id).await
                } else {
                    "ERR: Invalid or expired session".to_string()
                }
            }
            "/reject_group_invite" if args.len() >= 2 => {
                let session_token = args[0];
                let invite_id = args[1];
                if let Some(uid) = auth::validate_session(self.db.clone(), session_token).await {
                    groups::reject_invite(self.db.clone(), &uid, invite_id).await
                } else {
                    "ERR: Invalid or expired session".to_string()
                }
            }
            "/my_group_invites" if args.len() == 1 => {
                let session_token = args[0];
                if let Some(uid) = auth::validate_session(self.db.clone(), session_token).await {
                    groups::my_invites(self.db.clone(), &uid).await
                } else {
                    "ERR: Invalid or expired session".to_string()
                }
            }
            "/group_members" if args.len() == 2 => {
                let session_token = args[0];
                let group_id = args[1];
                if let Some(_uid) = auth::validate_session(self.db.clone(), session_token).await {
                    groups::get_group_members(self.db.clone(), group_id).await
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
            "/send_group_message" if args.len() >= 3 => {
                let session_token = args[0];
                let group_name = args[1];
                let message = &args[2..].join(" ");
                messages::send_group_message(self.db.clone(), session_token, group_name, message, &self.config).await
            }
            "/send_private_message"  if args.len() >= 3 => {
                let session_token = args[0];
                let to_username = args[1];
                let message = &args[2..].join(" ");
                messages::send_private_message(self.db.clone(), session_token, to_username, message, &self.config).await
            }
            "/get_group_messages" if args.len() == 2 => {
                let session_token = args[0];
                let group_name = args[1];
                messages::get_group_messages(self.db.clone(), session_token, group_name, &self.config).await
            }
            "/get_private_messages" if args.len() == 2 => {
                let session_token = args[0];
                let other_username = args[1];
                messages::get_private_messages(self.db.clone(), session_token, other_username, &self.config).await
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

async fn handle_client(db: Arc<Database>, config: ServerConfig, stream: TcpStream, peer: std::net::SocketAddr, presence: PresenceRegistry) -> anyhow::Result<()> {
    let (reader, writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);
    let mut line = String::new();
    let mut kick_rx: Option<tokio::sync::oneshot::Receiver<()>> = None;
    let mut registered_user: Option<String> = None;
    let mut registered_token: Option<String> = None;
    loop {
        line.clear();
        if let Some(rx) = &mut kick_rx {
            tokio::select! {
                biased;
                _ = rx => {
                    if let Some(uid) = &registered_user {
                        println!("[AUTH] User {} kicked out due to login from another device", uid);
                    } else {
                        println!("[SERVER] Client was kicked out");
                    }
                    break;
                }
                res = reader.read_line(&mut line) => {
                    let n = res?;
                    if n == 0 {
                        println!("[SERVER] Client disconnected: {}", peer);
                        break;
                    }
                }
            }
        } else {
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                println!("[SERVER] Client disconnected: {}", peer);
                break;
            }
        }
    let trimmed = line.trim();
    // Raw incoming line logger for diagnostics
    println!("[CONN:RAW] [{}] Raw line received: '{}'", peer, trimmed);
        if trimmed.is_empty() { continue; }
        let mut parts = trimmed.split_whitespace();
        let cmd = parts.next().unwrap_or("");
        let args: Vec<&str> = parts.collect();
        println!("[CONN] [{}] Cmd='{}' Args={:?}", peer, cmd, args);
        let server = Server { db: db.clone(), config: config.clone(), presence: presence.clone() };
        let response = server.handle_command(cmd, &args).await;
        println!("[CONN] [{}] Response: {}", peer, response);
        // If the client just validated an existing session, register presence so
        // we treat this connection as an active one (preserve session row for auto-login
        // but reflect presence in is_online).
        if cmd == "/validate_session" && args.len() == 1 && response.starts_with("OK:") {
            let token = args[0];
            println!("[CONN] [{}] /validate_session returned OK for token {} — registering presence", peer, token);
            if let Some(uid) = auth::validate_session(db.clone(), token).await {
                // Do not kick existing sessions on validate; just register this connection
                let rx = presence.register(&uid).await;
                println!("[CONN] [{}] Registered presence receiver for user {} (via validate_session)", peer, uid);
                // set is_online = 1 when a connection registers
                let _ = sqlx::query("UPDATE users SET is_online = 1 WHERE id = ?")
                    .bind(&uid)
                    .execute(&db.pool)
                    .await;
                println!("[DB] Set is_online=1 for user {} due to validate_session", uid);
                kick_rx = Some(rx);
                registered_user = Some(uid.clone());
                registered_token = Some(token.to_string());
            } else {
                println!("[CONN] [{}] validate_session token {} became invalid during registration", peer, token);
            }
        }
        if response.contains("SESSION:") {
            if let Some(line) = response.lines().find(|l| l.contains("SESSION:")) {
                if let Some(tok) = line.split("SESSION:").nth(1) {
                    let token = tok.trim();
                    println!("[CONN] [{}] Detected SESSION token: {}", peer, token);
                    if let Some(uid) = auth::validate_session(db.clone(), token).await {
                        println!("[CONN] [{}] Token maps to user_id={}", peer, uid);
                        // kick previous sessions for this user and record event
                        let kicked = presence.kick_all(&uid).await;
                        if kicked > 0 {
                            println!("[AUTH] User {} kicked out due to login from another device (kicked={})", uid, kicked);
                            let now = chrono::Utc::now().timestamp();
                            let res = sqlx::query("INSERT INTO session_events (user_id, event_type, created_at) VALUES (?, ?, ?)")
                                .bind(&uid)
                                .bind("kicked_out")
                                .bind(now)
                                .execute(&db.pool)
                                .await;
                            println!("[DB] Inserted kicked_out event for {} result={:?}", uid, res);
                        } else {
                            println!("[AUTH] No previous sessions to kick for {}", uid);
                        }
                        let rx = presence.register(&uid).await;
                        println!("[CONN] [{}] Registered presence receiver for user {}", peer, uid);
                        // set is_online = 1 when a connection registers
                        let _ = sqlx::query("UPDATE users SET is_online = 1 WHERE id = ?")
                            .bind(&uid)
                            .execute(&db.pool)
                            .await;
                        println!("[DB] Set is_online=1 for user {} due to active connection", uid);
                        kick_rx = Some(rx);
                        registered_user = Some(uid.clone());
                        registered_token = Some(token.to_string());
                    }
                }
            }
        }
        writer.write_all(response.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }
    if let Some(uid) = registered_user {
        println!("[CONN] [{}] Connection for user {} ending; cleaning up", peer, uid);
        presence.unregister_one(&uid).await;
        // If no more active connections, set is_online = 0 (preserve session row for auto-login)
        let remaining = presence.count(&uid).await;
        if remaining == 0 {
            let _ = sqlx::query("UPDATE users SET is_online = 0 WHERE id = ?")
                .bind(&uid)
                .execute(&db.pool)
                .await;
            println!("[DB] Set is_online=0 for user {} because no active connections remain", uid);
        } else {
            println!("[CONN] [{}] {} active connections remain for user {}, leaving is_online=1", peer, remaining, uid);
        }
        if let Some(tok) = registered_token {
            println!("[CONN] [{}] Preserving session token {} for user {} to allow auto-login on reconnect", peer, tok, uid);
        } else {
            println!("[CONN] [{}] No session token associated with this connection", peer);
        }
        let now = chrono::Utc::now().timestamp();
        let res = sqlx::query("INSERT INTO session_events (user_id, event_type, created_at) VALUES (?, ?, ?)")
            .bind(&uid)
            .bind("quit")
            .bind(now)
            .execute(&db.pool)
            .await;
        println!("[DB] Inserted quit event for {} result={:?}", uid, res);
    }
    Ok(())
}

// TLS stream handling: keep the same protocol logic but using the TLS stream types
async fn handle_tls_client<S>(db: Arc<Database>, config: ServerConfig, stream: S, peer: std::net::SocketAddr, presence: PresenceRegistry) -> anyhow::Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let (reader, writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);
    let mut line = String::new();
    let mut kick_rx: Option<tokio::sync::oneshot::Receiver<()>> = None;
    let mut registered_user: Option<String> = None;
    let mut registered_token: Option<String> = None;
    loop {
        line.clear();
        if let Some(rx) = &mut kick_rx {
            tokio::select! {
                biased;
                _ = rx => {
                    if let Some(uid) = &registered_user {
                        println!("[AUTH] User {} kicked out due to login from another device", uid);
                    } else {
                        println!("[SERVER] Client was kicked out");
                    }
                    break;
                }
                res = reader.read_line(&mut line) => {
                    let n = res?;
                    if n == 0 {
                        println!("[SERVER] Client disconnected: {}", peer);
                        break;
                    }
                }
            }
        } else {
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                println!("[SERVER] Client disconnected: {}", peer);
                break;
            }
        }
    let trimmed = line.trim();
    // Raw incoming line logger for diagnostics (TLS)
    println!("[CONN:RAW] [{}] TLS Raw line received: '{}'", peer, trimmed);
    if trimmed.is_empty() { continue; }
        let mut parts = trimmed.split_whitespace();
        let cmd = parts.next().unwrap_or("");
        let args: Vec<&str> = parts.collect();
        let server = Server { db: db.clone(), config: config.clone(), presence: presence.clone() };
        let response = server.handle_command(cmd, &args).await;
        // If the client just validated an existing session, register presence so
        // we treat this TLS connection as an active one (preserve session row for auto-login
        // but reflect presence in is_online).
        if cmd == "/validate_session" && args.len() == 1 && response.starts_with("OK:") {
            let token = args[0];
            println!("[CONN] [{}] TLS /validate_session returned OK for token {} — registering presence", peer, token);
            if let Some(uid) = auth::validate_session(db.clone(), token).await {
                let rx = presence.register(&uid).await;
                println!("[CONN] [{}] TLS Registered presence receiver for user {} (via validate_session)", peer, uid);
                let _ = sqlx::query("UPDATE users SET is_online = 1 WHERE id = ?")
                    .bind(&uid)
                    .execute(&db.pool)
                    .await;
                println!("[DB] TLS Set is_online=1 for user {} due to validate_session", uid);
                kick_rx = Some(rx);
                registered_user = Some(uid.clone());
                registered_token = Some(token.to_string());
            } else {
                println!("[CONN] [{}] TLS validate_session token {} became invalid during registration", peer, token);
            }
        }
        if response.contains("SESSION:") {
            if let Some(line) = response.lines().find(|l| l.contains("SESSION:")) {
                if let Some(tok) = line.split("SESSION:").nth(1) {
                    let token = tok.trim();
                    if let Some(uid) = auth::validate_session(db.clone(), token).await {
                        let kicked = presence.kick_all(&uid).await;
                        if kicked > 0 {
                            println!("[AUTH] User {} kicked out due to login from another device", uid);
                            let now = chrono::Utc::now().timestamp();
                            let _ = sqlx::query("INSERT INTO session_events (user_id, event_type, created_at) VALUES (?, ?, ?)")
                                .bind(&uid)
                                .bind("kicked_out")
                                .bind(now)
                                .execute(&db.pool)
                                .await;
                        }
                        let rx = presence.register(&uid).await;
                        kick_rx = Some(rx);
                        registered_user = Some(uid.clone());
                        registered_token = Some(token.to_string());
                    }
                }
            }
        }
        writer.write_all(response.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }
    if let Some(uid) = registered_user {
        println!("[CONN] [{}] TLS connection for user {} ending; cleaning up", peer, uid);
        presence.unregister_one(&uid).await;
        // If no more active connections, set is_online = 0 (preserve session row for auto-login)
        let remaining = presence.count(&uid).await;
        if remaining == 0 {
            let _ = sqlx::query("UPDATE users SET is_online = 0 WHERE id = ?")
                .bind(&uid)
                .execute(&db.pool)
                .await;
            println!("[DB] TLS Set is_online=0 for user {} because no active connections remain", uid);
        } else {
            println!("[CONN] [{}] TLS {} active connections remain for user {}, leaving is_online=1", peer, remaining, uid);
        }
        if let Some(tok) = registered_token {
            println!("[CONN] [{}] TLS preserving session token {} for user {} to allow auto-login on reconnect", peer, tok, uid);
        } else {
            println!("[CONN] [{}] TLS No session token associated with this connection", peer);
        }
        let now = chrono::Utc::now().timestamp();
        let res = sqlx::query("INSERT INTO session_events (user_id, event_type, created_at) VALUES (?, ?, ?)")
            .bind(&uid)
            .bind("quit")
            .bind(now)
            .execute(&db.pool)
            .await;
        println!("[DB] TLS Inserted quit event for {} result={:?}", uid, res);
    }
    Ok(())
}
