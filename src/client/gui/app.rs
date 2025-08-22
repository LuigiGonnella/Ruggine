use iced::{Application, Command, Element, Theme};
use crate::client::models::app_state::{AppState, ChatAppState};
use crate::client::models::messages::Message;
use crate::client::services::chat_service::ChatService;

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
        (
            ChatApp {
                state: ChatAppState::default(),
                chat_service: ChatService::default(),
            },
            Command::none(),
        )
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
                let host = match self.state.selected_host {
                    crate::client::gui::views::registration::HostType::Localhost => "127.0.0.1:5000".to_string(),
                    crate::client::gui::views::registration::HostType::Remote => "remote.server.com:5000".to_string(),
                    crate::client::gui::views::registration::HostType::Manual => self.state.manual_host.clone(),
                };
                let is_login = self.state.is_login;
                self.state.loading = true;
                self.state.error_message = None;
                self.state.log_messages.push((format!("Connessione a {}...", host), iced::Color::from_rgb(0.0, 0.0, 1.0)));
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
                                if response.contains("OK: Registered") || response.contains("OK: Logged in") {
                                    let token = response.lines().find_map(|l| {
                                        if l.contains("SESSION:") {
                                            Some(l.split("SESSION:").nth(1).map(|s| s.trim().to_string()).unwrap_or_default())
                                        } else { None }
                                    });
                                    Msg::AuthResult { success: true, message: response, token }
                                } else {
                                    Msg::AuthResult { success: false, message: response, token: None }
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
            AppState::Registration => crate::client::gui::views::registration::view(&self.state),
            AppState::MainActions => crate::client::gui::views::main_actions::view(&self.state),
            AppState::PrivateChat(username) => crate::client::gui::views::private_chat::view(&self.state, username),
            AppState::GroupChat(group_id, group_name) => crate::client::gui::views::group_chat::view(&self.state, group_id, group_name),
            AppState::FriendRequests => crate::client::gui::views::friend_requests::view(&self.state),
            AppState::Chat => crate::client::gui::views::main_actions::view(&self.state),
        }
    }
}
