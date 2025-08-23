use iced::{Application, Command, Element, Theme};
use crate::client::models::app_state::{AppState, ChatAppState};
use crate::client::models::messages::Message;
use crate::client::services::chat_service::ChatService;
use crate::client::utils::session_store;

pub struct ChatApp {
    pub state: ChatAppState,
    pub chat_service: ChatService,
}

impl Application for ChatApp {
    type Message = Message;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        // Create default app and attempt to auto-validate saved session token.
        let app = ChatApp {
            state: ChatAppState::default(),
            chat_service: ChatService::default(),
        };

        // Perform async startup check: if a token is saved, try validate it against the default host.
        let cmd = Command::perform(
            async move {
                // Load token from secure store (do not log token contents)
                if let Some(token) = session_store::load_session_token() {
                    println!("[APP_START] Found saved session token (redacted)");
                    // try to connect to default host from env
                    let cfg = crate::server::config::ClientConfig::from_env();
                    let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                match tokio::net::TcpStream::connect(&host).await {
            Ok(stream) => {
                            use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
                            let (r, w) = stream.into_split();
                            let mut reader = BufReader::new(r);
                            let mut writer = BufWriter::new(w);
                            let cmd = format!("/validate_session {}\n", token);
                            writer.write_all(cmd.as_bytes()).await.ok();
                            writer.flush().await.ok();
                            let mut line = String::new();
                            let n = reader.read_line(&mut line).await.ok();
                            if n == Some(0) { return Message::None; }
                            let response = line.trim().to_string();
                            // Sanitize response: remove SESSION part before showing or passing to app state
                            let cleaned = response.split("SESSION:").next().map(|s| s.trim().to_string()).unwrap_or_default();
                            if response.starts_with("OK:") {
                                return Message::AuthResult { success: true, message: cleaned, token: Some(token) };
                            } else {
                                // invalid session -> act like no session saved
                                return Message::SessionMissing;
                            }
                        }
                    Err(_) => Message::SessionMissing,
                    }
        } else { Message::SessionMissing }
            },
            |m| m,
        );

        (app, cmd)
    }

    fn title(&self) -> String {
        "Ruggine Chat".to_string()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        use crate::client::models::messages::Message as Msg;
        match &message {
            Msg::SubmitLoginOrRegister => {
                let username = self.state.username.clone();
                let password = self.state.password.clone();
                // Resolve host selection: use ClientConfig from env for defaults
                let cfg = crate::server::config::ClientConfig::from_env();
                let host = match self.state.selected_host {
                    crate::client::gui::views::registration::HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                    crate::client::gui::views::registration::HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                    crate::client::gui::views::registration::HostType::Manual => self.state.manual_host.clone(),
                };
                let is_login = self.state.is_login;
                self.state.loading = true;
                self.state.error_message = None;
                use crate::client::gui::views::logger::{LogMessage, LogLevel};
                self.state.logger.push(LogMessage {
                    level: LogLevel::Info,
                    message: format!("Connessione a {}...", host),
                });
                // Esegui la connessione e invia il comando
                return Command::perform(
                    async move {
                        use tokio::net::TcpStream;
                        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
                        let stream = TcpStream::connect(&host).await;
                        match stream {
                            Ok(stream) => {
                                let (reader, writer) = stream.into_split();
                                let mut server_reader = BufReader::new(reader);
                                let mut server_writer = BufWriter::new(writer);
                                let cmd = if is_login {
                                    format!("/login {} {}", username, password)
                                } else {
                                    format!("/register {} {}", username, password)
                                };
                                server_writer.write_all(cmd.as_bytes()).await.ok();
                                server_writer.write_all(b"\n").await.ok();
                                server_writer.flush().await.ok();
                                let mut server_line = String::new();
                                let n = server_reader.read_line(&mut server_line).await.ok();
                                if n == Some(0) {
                                    return Msg::AuthResult { success: false, message: "Server disconnesso".to_string(), token: None };
                                }
                                let response = server_line.trim().to_string();
                                // extract token if present
                                let token = response.lines().find_map(|l| {
                                    if l.contains("SESSION:") {
                                        Some(l.split("SESSION:").nth(1).map(|s| s.trim().to_string()).unwrap_or_default())
                                    } else { None }
                                });
                                let cleaned = response.split("SESSION:").next().map(|s| s.trim().to_string()).unwrap_or_default();
                                if response.contains("OK: Registered") || response.contains("OK: Logged in") {
                                    Msg::AuthResult { success: true, message: cleaned, token }
                                } else {
                                    Msg::AuthResult { success: false, message: cleaned, token: None }
                                }
                            }
                            Err(e) => Msg::AuthResult { success: false, message: format!("Connessione fallita: {}", e), token: None },
                        }
                    },
                    |msg| msg,
                );
            }
            _ => {}
        }
        self.state.update(message, &mut self.chat_service)
    }

    fn view(&self) -> Element<Message> {
        match &self.state.app_state {
            AppState::CheckingSession => {
                // Small placeholder while we validate a persisted session token on startup.
                iced::widget::Text::new("Controllo sessione...").into()
            }
            AppState::Registration => crate::client::gui::views::registration::view(&self.state),
            AppState::MainActions => crate::client::gui::views::main_actions::view(&self.state),
            AppState::PrivateChat(username) => crate::client::gui::views::private_chat::view(&self.state, username),
            AppState::GroupChat(group_id, group_name) => crate::client::gui::views::group_chat::view(&self.state, group_id, group_name),
            AppState::FriendRequests => crate::client::gui::views::friend_requests::view(&self.state),
            AppState::Chat => crate::client::gui::views::main_actions::view(&self.state),
        }
    }
}
