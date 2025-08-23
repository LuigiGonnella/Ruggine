use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter, stdin};

use crate::server::config::ClientConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // load .env variables so KEYRING_FALLBACK can be set there for development
    let _ = dotenvy::dotenv();
    let client_config = ClientConfig::from_env();
    let addr = std::env::args().nth(1).unwrap_or_else(|| format!("{}:{}", client_config.default_host, client_config.default_port));
    println!("[CLIENT] Benvenuto! Digita i comandi (es: /register user pass, /login user pass):");
    let stream = TcpStream::connect(&addr).await?;
    let (reader, writer) = stream.into_split();
    let mut server_reader = BufReader::new(reader);
    let mut server_writer = BufWriter::new(writer);
    let mut input = BufReader::new(stdin());
    let mut input_line = String::new();
    let mut server_line = String::new();
    let mut session_token: Option<String> = None;
    loop {
        input_line.clear();
        print!("> ");
        use std::io::Write;
        std::io::stdout().flush().unwrap();
        let n = input.read_line(&mut input_line).await?;
        if n == 0 { break; }
        let cmd = input_line.trim();
        if cmd.is_empty() { continue; }
        let mut parts = cmd.split_whitespace();
        let command = parts.next().unwrap_or("");
        let args: Vec<&str> = parts.collect();
        // Comandi che NON richiedono session_token
    let public_cmds = ["/register", "/login", "/users", "/all_users", "/logout", "/help", "/quit"];
        let friend_cmds = [
            "/send_friend_request", "/accept_friend_request", "/reject_friend_request",
            "/list_friends", "/received_friend_requests", "/sent_friend_requests"
        ];
        // Comandi di messaggistica che richiedono token ma hanno parsing speciale
        let msg_cmds = ["/send", "/send_private", "/private", "/get_group_messages", "/get_private_messages", "/delete_group_messages", "/delete_private_messages"];
        let mut to_send = String::new();
        // Limite lunghezza messaggio
        if msg_cmds.contains(&command) && args.len() >= 2 {
            let message = &args[1..].join(" ");
            if message.len() > 2048 {
                println!("[CLIENT] Messaggio troppo lungo (max 2048 caratteri)");
                continue;
            }
        }
        if public_cmds.contains(&command) {
            // /logout richiede il token
            if command == "/logout" {
                if let Some(token) = &session_token {
                    to_send = format!("/logout {}", token);
                } else {
                    println!("[CLIENT] Devi prima effettuare il login!");
                    continue;
                }
            } else if command == "/quit" {
                to_send = "/quit".to_string();
            } else {
                to_send = cmd.to_string();
            }
        } else if friend_cmds.contains(&command) {
            if let Some(token) = &session_token {
                match command {
                    "/send_friend_request" if args.len() >= 1 => {
                        let to_username = args[0];
                        let message = if args.len() > 1 { args[1..].join(" ") } else { "".to_string() };
                        to_send = format!("/send_friend_request {} {} {}", token, to_username, message);
                    }
                    "/accept_friend_request" | "/reject_friend_request" if args.len() == 1 => {
                        to_send = format!("{} {} {}", command, token, args[0]);
                    }
                    "/list_friends" | "/received_friend_requests" | "/sent_friend_requests" => {
                        to_send = format!("{} {}", command, token);
                    }
                    _ => {
                        println!("[CLIENT] Sintassi comando non valida.");
                        continue;
                    }
                }
            } else {
                println!("[CLIENT] Devi prima effettuare il login!");
                continue;
            }
    } else if msg_cmds.contains(&command) {
            if let Some(token) = &session_token {
                // Ricostruisci la sintassi server
                match command {
                    "/send" if args.len() >= 2 => {
                        let group = args[0];
                        let message = &args[1..].join(" ");
                        to_send = format!("/send {} {} {}", token, group, message);
                    }
                    "/send_private" | "/private" if args.len() >= 2 => {
                        let user = args[0];
                        let message = &args[1..].join(" ");
                        to_send = format!("{} {} {} {}", command, token, user, message);
                    }
                    "/get_group_messages" if args.len() == 1 => {
                        to_send = format!("/get_group_messages {} {}", token, args[0]);
                    }
                    "/get_private_messages" if args.len() == 1 => {
                        to_send = format!("/get_private_messages {} {}", token, args[0]);
                    }
                    "/delete_group_messages" if args.len() == 1 => {
                        to_send = format!("/delete_group_messages {} {}", token, args[0]);
                    }
                    "/delete_private_messages" if args.len() == 1 => {
                        to_send = format!("/delete_private_messages {} {}", token, args[0]);
                    }
                    _ => {
                        println!("[CLIENT] Sintassi comando non valida.");
                        continue;
                    }
                }
            } else {
                println!("[CLIENT] Devi prima effettuare il login!");
                continue;
            }
        } else {
            // Altri comandi autenticati generici
            if let Some(token) = &session_token {
                to_send = format!("{} {}{}", command, token, if args.is_empty() { "".to_string() } else { format!(" {}", args.join(" ")) });
            } else {
                println!("[CLIENT] Devi prima effettuare il login!");
                continue;
            }
        }
        server_writer.write_all(to_send.as_bytes()).await?;
        server_writer.write_all(b"\n").await?;
        server_writer.flush().await?;
        server_line.clear();
        let n = server_reader.read_line(&mut server_line).await?;
        if n == 0 {
            println!("[CLIENT] Server disconnesso");
            break;
        }
        let raw_response = server_line.trim().to_string();
        // Do not print raw server lines that may contain session tokens. Show sanitized messages instead.
        let cleaned = raw_response.split("SESSION:").next().map(|s| s.trim()).unwrap_or("");
        if cleaned.starts_with("OK:") {
            // display only the human-friendly part after OK:
            println!("[SERVER] {}", cleaned.trim_start_matches("OK:").trim());
        } else if cleaned.starts_with("ERR:") {
            println!("[SERVER][ERROR] {}", cleaned.trim_start_matches("ERR:").trim());
        } else {
            println!("[SERVER] {}", cleaned);
        }
        // Estrai session_token dopo login
        if command == "/login" && raw_response.contains("SESSION:") {
            if let Some(line) = raw_response.lines().find(|l| l.contains("SESSION:")) {
                if let Some(token) = line.split("SESSION:").nth(1) {
                    session_token = Some(token.trim().to_string());
                    println!("[CLIENT] Login effettuato! Sessione attiva.");
                }
            }
        }
        // Cancella session_token dopo logout
        if command == "/logout" && raw_response.starts_with("OK: Logout") {
            session_token = None;
            println!("[CLIENT] Logout effettuato. Sessione terminata.");
        }
        // Chiudi app dopo /quit
        if command == "/quit" {
            println!("[CLIENT] Disconnessione e uscita.");
            break;
        }
    }
    Ok(())
}
