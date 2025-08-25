use std::collections::HashMap;
use crate::client::gui::views::registration::HostType;
use crate::client::gui::views::logger::LogMessage;
use crate::client::models::messages::Message;
use crate::client::services::chat_service::ChatService;
use std::sync::Arc;
use tokio::sync::Mutex;
use iced::Command;
use iced::widget::scrollable;

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    CheckingSession,
    Registration,
    MainActions,
    PrivateChat(String),
    GroupChat(String, String),
    UsersList(String),
    FriendRequests,
    Chat,
}

impl Default for AppState {
    fn default() -> Self {
        AppState::CheckingSession
    }
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub sender: String,
    pub content: String,
    pub timestamp: i64,
    pub formatted_time: String,
    pub sent_at: i64,
}

#[derive(Debug, Clone, Default)]
pub struct ChatAppState {
    pub app_state: AppState,
    pub username: String,
    pub password: String,
    pub selected_host: HostType,
    pub manual_host: String,
    pub is_login: bool,
    pub loading: bool,
    pub error_message: Option<String>,
    pub session_token: Option<String>,
    pub show_password: bool,
    pub logger: Vec<LogMessage>,
    pub users_search_query: String,
    pub users_search_results: Vec<String>,
    pub current_message_input: String,
    pub private_chats: HashMap<String, Vec<ChatMessage>>,
    pub loading_private_chats: std::collections::HashSet<String>,
    pub polling_active: bool,
}

impl ChatAppState {
    pub fn update(&mut self, message: Message, chat_service: &Arc<Mutex<ChatService>>) -> Command<Message> {
        use crate::client::gui::views::logger::{LogMessage, LogLevel};
        use crate::client::utils::session_store;
        use crate::client::services::{users_service::UsersService, friend_service::FriendService};
        
        match message {
            Message::ManualHostChanged(host) => {
                self.manual_host = host;
            }
            Message::UsernameChanged(username) => {
                self.username = username;
            }
            Message::PasswordChanged(password) => {
                self.password = password;
            }
            Message::HostSelected(host_type) => {
                self.selected_host = host_type;
            }
            Message::ToggleLoginRegister => {
                self.is_login = !self.is_login;
                self.error_message = None;
            }
            Message::ToggleShowPassword => {
                self.show_password = !self.show_password;
            }
            Message::AuthResult { success, message, token } => {
                self.loading = false;
                if success {
                    if let Some(t) = token {
                        self.session_token = Some(t.clone());
                        // Save token securely
                        if let Err(e) = session_store::save_session_token(&t) {
                            println!("[SESSION_STORE] Failed to save token: {}", e);
                        } else {
                            println!("[SESSION_STORE] Token saved successfully");
                        }
                        
                        // Extract username from success message for auto-login cases
                        if message.starts_with("OK:") {
                            let username_part = message.trim_start_matches("OK:").trim();
                            if !username_part.is_empty() && self.username.is_empty() {
                                self.username = username_part.to_string();
                            }
                        }
                    }
                    self.app_state = AppState::MainActions;
                    // Clear any previous error messages and logger for clean transition
                    self.error_message = None;
                    self.logger.clear();
                    self.logger.push(LogMessage {
                        level: LogLevel::Success,
                        message: if self.is_login || message.contains("validate") { 
                            "Login successful".to_string() 
                        } else { 
                            "Registration successful".to_string() 
                        },
                    });
                } else {
                    self.error_message = Some(message.clone());
                    self.logger.clear(); // Clear previous messages
                    self.logger.push(LogMessage {
                        level: LogLevel::Error,
                        message: message.clone(),
                    });
                }
            }
            Message::SessionMissing => {
                self.app_state = AppState::Registration;
                self.logger.clear(); // Clear any previous messages
            }
            Message::Logout => {
                // Clear session token from secure storage
                let _ = session_store::clear_session_token();
                
                // Send logout command if we have a token
                if let Some(token) = &self.session_token {
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    let cfg = crate::server::config::ClientConfig::from_env();
                    let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                    
                    // Send logout command asynchronously but don't wait for response
                    tokio::spawn(async move {
                        let mut guard = svc.lock().await;
                        let _ = guard.send_command(&host, format!("/logout {}", token_clone)).await;
                    });
                }
                
                // Reset state
                self.session_token = None;
                self.username.clear();
                self.password.clear();
                self.app_state = AppState::Registration;
                // Clear logger completely on logout for clean slate
                self.logger.clear();
            }
            Message::ClearLog => {
                self.logger.clear();
            }
            Message::LogInfo(msg) => {
                self.logger.push(LogMessage {
                    level: LogLevel::Info,
                    message: msg,
                });
            }
            Message::LogSuccess(msg) => {
                self.logger.push(LogMessage {
                    level: LogLevel::Success,
                    message: msg,
                });
            }
            Message::LogError(msg) => {
                self.logger.push(LogMessage {
                    level: LogLevel::Error,
                    message: msg,
                });
            }
            Message::OpenMainActions => {
                self.app_state = AppState::MainActions;
            }
            Message::OpenPrivateChat(username) => {
                self.app_state = AppState::PrivateChat(username.clone());
                self.current_message_input.clear();
                // Mark this private chat as loading so the UI shows a loader
                self.loading_private_chats.insert(username.clone());

                // Start message polling for real-time updates
                return Command::perform(
                    async move { Message::StartMessagePolling { with: username } },
                    |msg| msg,
                );
            }
            Message::OpenGroupChat(group_id, group_name) => {
                self.app_state = AppState::GroupChat(group_id, group_name);
            }
            Message::OpenUsersList { kind } => {
                self.app_state = AppState::UsersList(kind.clone());
                self.users_search_query.clear();
                self.users_search_results.clear();
                
                // Auto-load users based on kind
                let svc = chat_service.clone();
                let cfg = crate::server::config::ClientConfig::from_env();
                let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                
                return Command::perform(
                    async move {
                        let result = if kind == "Online" {
                            UsersService::list_online(&svc, &host).await
                        } else {
                            UsersService::list_all(&svc, &host).await
                        };
                        
                        match result {
                            Ok(users) => Message::UsersListLoaded { kind, list: users },
                            Err(_) => Message::UsersListLoaded { kind, list: vec![] },
                        }
                    },
                    |msg| msg,
                );
            }
            Message::OpenFriendRequests => {
                self.app_state = AppState::FriendRequests;
            }
            Message::UsersSearchQueryChanged(query) => {
                self.users_search_query = query;
            }
            Message::UsersSearch => {
                // Trigger search based on current query
                if !self.users_search_query.is_empty() {
                    let svc = chat_service.clone();
                    let cfg = crate::server::config::ClientConfig::from_env();
                    let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                    let query = self.users_search_query.clone();
                    // Clone current username so the async block does not borrow &self
                    let current_username = self.username.clone();

                    return Command::perform(
                        async move {
                            // For now, just return all users and filter client-side
                            match UsersService::list_all(&svc, &host).await {
                                Ok(users) => {
                                    let filtered: Vec<String> = users.into_iter()
                                        .filter(|u| u.to_lowercase().contains(&query.to_lowercase()))
                                        .filter(|u| u != &current_username) // Remove current user from search results
                                        .collect();
                                    Message::UsersListLoaded { kind: "Search".to_string(), list: filtered }
                                }
                                Err(_) => Message::UsersListLoaded { kind: "Search".to_string(), list: vec![] },
                            }
                        },
                        |msg| msg,
                    );
                }
            }
            Message::UsersListLoaded { kind: _, list } => {
                // Filter out current user from all user lists
                self.users_search_results = list.into_iter()
                    .filter(|u| u != &self.username)
                    .collect();
            }
            Message::ListOnlineUsers => {
                return Command::perform(
                    async { Message::OpenUsersList { kind: "Online".to_string() } },
                    |msg| msg,
                );
            }
            Message::ListAllUsers => {
                return Command::perform(
                    async { Message::OpenUsersList { kind: "All".to_string() } },
                    |msg| msg,
                );
            }
            Message::MessageInputChanged(input) => {
                self.current_message_input = input;
            }
            Message::SendPrivateMessage { to } => {
                if !self.current_message_input.trim().is_empty() {
                    if let Some(token) = &self.session_token {
                        let svc = chat_service.clone();
                        let token_clone = token.clone();
                        let to_clone = to.clone();
                        let message = self.current_message_input.trim().to_string();
                        let cfg = crate::server::config::ClientConfig::from_env();
                        let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                        
                        // Clear input immediately for better UX
                        // If we don't have the chat history cached yet, mark it as loading
                        if !self.private_chats.contains_key(&to) {
                            self.loading_private_chats.insert(to.clone());
                        }

                        self.current_message_input.clear();
                        
                        return Command::batch([
                            Command::perform(
                                async move {
                                    let mut guard = svc.lock().await;
                                    let _ = guard.send_private_message(&host, &token_clone, &to_clone, &message).await;
                                    Message::TriggerImmediateRefresh { with: to_clone }
                                },
                                |msg| msg,
                            ),
                            // Auto-scroll to bottom after sending
                            scrollable::snap_to(
                                scrollable::Id::new("messages_scroll"),
                                scrollable::RelativeOffset::END
                            )
                        ]);
                    }
                }
            }
            Message::LoadPrivateMessages { with } => {
                if let Some(token) = &self.session_token {
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    let with_clone = with.clone();
                    let cfg = crate::server::config::ClientConfig::from_env();
                    let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                    
                    return Command::perform(
                        async move {
                            let mut guard = svc.lock().await;
                            match guard.get_private_messages(&host, &token_clone, &with_clone).await {
                                Ok(messages) => Message::PrivateMessagesLoaded { with: with_clone, messages },
                                Err(_) => Message::PrivateMessagesLoaded { with: with_clone, messages: vec![] },
                            }
                        },
                        |msg| msg,
                    );
                }
            }
            Message::PrivateMessagesLoaded { with, messages } => {
                self.private_chats.insert(with.clone(), messages);
                self.loading_private_chats.remove(&with);
                
                // Auto-scroll to bottom when messages are loaded
                return scrollable::snap_to(
                    scrollable::Id::new("messages_scroll"),
                    scrollable::RelativeOffset::END
                );
            }
            // Placeholder implementations for other messages
            _ => {
                // Handle other messages as needed
            }
        }
        
        Command::none()
    }
}