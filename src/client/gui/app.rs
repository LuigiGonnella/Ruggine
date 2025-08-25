use iced::{Application, Command, Element, Theme};
use crate::client::models::app_state::{AppState, ChatAppState};
use crate::client::models::messages::Message;
use crate::client::services::chat_service::ChatService;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::client::utils::session_store;

pub struct ChatApp {
    pub state: ChatAppState,
    pub chat_service: Arc<Mutex<ChatService>>,
}

impl Application for ChatApp {
    type Message = Message;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        // Create default app and attempt to auto-validate saved session token.
        let chat_service = Arc::new(Mutex::new(ChatService::new()));
        let app = ChatApp {
            state: ChatAppState::default(),
            chat_service: chat_service.clone(),
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
                // Use the app-level ChatService (persistent) to validate the saved session.
                let svc = chat_service.clone();
                let mut guard = svc.lock().await;
                match guard.send_command(&host, format!("/validate_session {}", token)).await {
                    Ok(response) => {
                        let cleaned = response.split("SESSION:").next().map(|s| s.trim().to_string()).unwrap_or_default();
                        if response.starts_with("OK:") {
                            // Extract username from response for auto-login display
                            let username = cleaned.trim_start_matches("OK:").trim();
                            return Message::AuthResult { 
                                success: true, 
                                message: format!("OK: {}", username), 
                                token: Some(token) 
                            };
                        } else {
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
    match message.clone() {
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
                let svc_outer = self.chat_service.clone();
                return Command::perform(
                    async move {
                        // Use the persistent ChatService stored in the app
                        let mut guard = svc_outer.lock().await;
                        let cmd = if is_login {
                            format!("/login {} {}", username, password)
                        } else {
                            format!("/register {} {}", username, password)
                        };
                        match guard.send_command(&host, cmd).await {
                            Ok(response) => {
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
            Msg::StartMessagePolling { with } => {
                if !self.state.group_polling_active {
                    println!("[APP] StartGroupMessagePolling requested for {}", group_id);
                    self.state.group_polling_active = true;
                    let svc = self.chat_service.clone();
                    let token = self.state.session_token.clone().unwrap_or_default();
                    let cfg = crate::server::config::ClientConfig::from_env();
                    let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                    let group_id_clone = group_id.clone();
                    return Command::perform(
                        async move {
                            println!("[APP] Fetching group messages for {}", group_id_clone);
                            let mut guard = svc.lock().await;
                            match guard.get_group_messages(&host, &token, &group_id_clone).await {
                                Ok(messages) => {
                                    println!("[APP] Fetched {} group messages for {}", messages.len(), group_id_clone);
                                    Msg::NewGroupMessagesReceived { group_id: group_id_clone.clone(), messages }
                                }
                                Err(e) => {
                                    println!("[APP] Group fetch failed for {}: {}", group_id_clone, e);
                                    Msg::NewGroupMessagesReceived { group_id: group_id_clone.clone(), messages: vec![] }
                                }
                            }
                        },
                        |msg| msg,
                    );
                }
                Command::none()
            }
            Msg::StopGroupMessagePolling => {
                self.state.group_polling_active = false;
                ()
            }
            Msg::NewGroupMessagesReceived { group_id, messages } => {
                if self.state.group_polling_active {
                    println!("[APP] NewGroupMessagesReceived for {}: {} messages", group_id, messages.len());
                    self.state.group_chats.insert(group_id.clone(), messages.to_vec());
                    // clear loading flag when messages arrive
                    self.state.loading_group_chats.remove(&group_id);
                    
                    // Continue polling
                    let svc = self.chat_service.clone();
                    let token = self.state.session_token.clone().unwrap_or_default();
                    let cfg = crate::server::config::ClientConfig::from_env();
                    let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                    let group_id_clone = group_id.clone();
                    
                    return Command::perform(
                        async move {
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                            let mut guard = svc.lock().await;
                            match guard.get_group_messages(&host, &token, &group_id_clone).await {
                                Ok(messages) => {
                                    drop(guard);
                                    Msg::NewGroupMessagesReceived { group_id: group_id_clone.clone(), messages }
                                }
                                Err(_) => {
                                    drop(guard);
                                    Msg::NewGroupMessagesReceived { group_id: group_id_clone.clone(), messages: vec![] }
                                }
                            }
                        },
                        |msg| msg,
                    );
                }
            }
            Msg::TriggerImmediateGroupRefresh { group_id } => {
                let cfg = crate::server::config::ClientConfig::from_env();
                let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                let token = self.state.session_token.clone().unwrap_or_default();
                let svc = self.chat_service.clone();
                let group_id_cloned = group_id.clone();
                return iced::Command::perform(
                    async move {
                        let mut guard = svc.lock().await;
                        guard.get_group_messages(&host, &token, &group_id_cloned).await.unwrap_or_default()
                    },
                    move |messages| Msg::NewGroupMessagesReceived { group_id: group_id.clone(), messages }
                );
            }
            Msg::StopMessagePolling => {
                self.state.polling_active = false;
                ()
            }
            Msg::NewMessagesReceived { with, messages } => {
                if self.state.polling_active {
                    println!("[APP] NewMessagesReceived for {}: {} messages", with, messages.len());
                    self.state.private_chats.insert(with.clone(), messages.to_vec());
                    // clear loading flag when messages arrive
                    self.state.loading_private_chats.remove(&with);
                    
                    // Continue polling
                    let svc = self.chat_service.clone();
                    let token = self.state.session_token.clone().unwrap_or_default();
                    let cfg = crate::server::config::ClientConfig::from_env();
                    let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                    let username = with.clone();
                    
                    return Command::perform(
                        async move {
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                            let mut guard = svc.lock().await;
                            match guard.get_private_messages(&host, &token, &username).await {
                                Ok(messages) => {
                                    drop(guard);
                                    Msg::NewMessagesReceived { with: username.clone(), messages }
                                }
                                Err(_) => {
                                    drop(guard);
                                    Msg::NewMessagesReceived { with: username.clone(), messages: vec![] }
                                }
                            }
                        },
                        |msg| msg,
                    );
                }
            }
            Msg::TriggerImmediateRefresh { with } => {
                let host = self.state.manual_host.clone();
                let token = self.state.session_token.clone().unwrap_or_default();
                let svc = self.chat_service.clone();
                    let with_cloned = with.clone();
                    return iced::Command::perform(
                        async move {
                            let mut guard = svc.lock().await;
                            guard.get_private_messages(&host, &token, &with_cloned).await.unwrap_or_default()
                        },
                        move |messages| Msg::NewMessagesReceived { with: with.clone(), messages }
                    );
            }
            _ => {}
        }
    self.state.update(message.clone(), &self.chat_service)
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
            AppState::UsersList(kind) => crate::client::gui::views::users_list::view(&self.state, kind),
            AppState::FriendRequests => crate::client::gui::views::friend_requests::view(&self.state),
            AppState::Chat => crate::client::gui::views::main_actions::view(&self.state),
            AppState::CreateGroup => crate::client::gui::views::create_group::view(&self.state),
            AppState::MyGroups => crate::client::gui::views::my_groups::view(&self.state),
            AppState::InviteToGroup { group_id, group_name } => crate::client::gui::views::invite_to_group::view(&self.state, group_id, group_name),
            AppState::MyGroupInvites => crate::client::gui::views::my_group_invites::view(&self.state),
            AppState::SendFriendRequest => crate::client::gui::views::send_friend_request::view(&self.state),
            AppState::ViewFriends => crate::client::gui::views::view_friends::view(&self.state),
        }
    }
}
