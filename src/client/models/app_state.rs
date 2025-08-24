#[derive(Debug, Clone)]
pub enum AppState {
    CheckingSession,
    Registration,
    MainActions,
    Chat,
    PrivateChat(String),
    GroupChat(String, String),
    UsersList(String),
    FriendRequests,
}

impl Default for AppState {
    fn default() -> Self {
    AppState::CheckingSession
    }
}


use crate::client::gui::views::registration::HostType;
use crate::client::gui::views::logger::LogMessage;
use tokio::sync::mpsc;
use crate::server::config::ClientConfig;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub sender: String,
    pub content: String,
    pub timestamp: i64, // Unix timestamp
    pub formatted_time: String, // Human readable time
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
    pub error_message: Option<String>,
    pub loading: bool,
    pub logger: Vec<LogMessage>,
    pub welcome_message: Option<String>,
    pub session_token: Option<String>,
    pub show_password: bool, // aggiunto per toggle password
    // Users list/search UI state
    pub users_list_cache: Vec<String>,
    pub users_search_results: Vec<String>,
    pub users_search_query: String,
    // Private chat state
    pub private_chats: HashMap<String, Vec<ChatMessage>>,
    pub message_receiver: Option<mpsc::UnboundedReceiver<(String, Vec<ChatMessage>)>>,
    pub current_message_input: String,
}

impl ChatAppState {
    pub fn update(&mut self, message: crate::client::models::messages::Message, chat_service: &std::sync::Arc<tokio::sync::Mutex<crate::client::services::chat_service::ChatService>>) -> iced::Command<crate::client::models::messages::Message> {
    use crate::client::models::messages::Message as Msg;
    use crate::client::utils::session_store;
    match message {
            Msg::UsernameChanged(val) => self.username = val,
            Msg::PasswordChanged(val) => self.password = val,
            Msg::HostSelected(host) => self.selected_host = host,
            Msg::ManualHostChanged(val) => self.manual_host = val,
            Msg::ToggleLoginRegister => self.is_login = !self.is_login,
            Msg::ToggleShowPassword => self.show_password = !self.show_password,
            Msg::SubmitLoginOrRegister => {
                self.loading = true;
                self.error_message = None;
                let username = self.username.clone();
                let password = self.password.clone();
                let is_login = self.is_login;
                let selected_host = self.selected_host.clone();
                let manual_host = self.manual_host.clone();
                let svc = chat_service.clone();
                // determine host
                let cfg = ClientConfig::from_env();
                let host = match selected_host {
                    HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                    HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                    HostType::Manual => manual_host,
                };
                return iced::Command::perform(async move {
                    let mut guard = svc.lock().await;
                    let cmd = if is_login {
                        format!("/login {} {}", username, password)
                    } else {
                        format!("/register {} {}", username, password)
                    };
                    match guard.send_command(&host, cmd).await {
                        Ok(resp) => {
                            // parse SESSION token if present
                            let token = resp.lines().find_map(|l| l.split("SESSION:").nth(1).map(|s| s.trim().to_string()));
                            let success = resp.starts_with("OK:");
                            crate::client::models::messages::Message::AuthResult { success, message: resp, token }
                        }
                        Err(e) => crate::client::models::messages::Message::AuthResult { success: false, message: format!("ERR: {}", e), token: None },
                    }
                }, |m| m);
            }
            Msg::AuthResult { success, message, token } => {
                self.loading = false;
                if success {
                    // store token in memory and persist securely (keyring)
                    if let Some(ref t) = token {
                        self.session_token = Some(t.clone());
                        let _ = session_store::save_session_token(t);
                    }
                    // If the username is not set (startup from saved token), try to extract it from server message
                    if self.username.trim().is_empty() {
                        if message.starts_with("OK:") {
                            if let Some(name_part) = message.splitn(2, ':').nth(1) {
                                self.username = name_part.trim().to_string();
                            }
                        }
                    }
                    // Push a single success log (replace previous) and switch to main state
                    let action = if self.is_login { "Login" } else { "Registrazione" };
                    self.logger.clear();
                    self.logger.push(crate::client::gui::views::logger::LogMessage {
                        level: crate::client::gui::views::logger::LogLevel::Success,
                        message: format!("Benvenuto, {}.", self.username),
                    });
                    self.app_state = AppState::MainActions;
                    // Schedule auto-clear of the log after 5 seconds
                    return iced::Command::perform(async { tokio::time::sleep(std::time::Duration::from_secs(5)).await; () }, |_| Msg::ClearLog);
                } else {
                    // sanitize server message: remove SESSION part and OK:/ERR prefixes
                    let raw = message.split("SESSION:").next().map(|s| s.trim()).unwrap_or("").to_string();
                    let cleaned = raw.trim_start_matches("OK:").trim_start_matches("ERR:").trim().to_string();
                    // Map common server messages into short, user-friendly Italian messages
                    let user_friendly = if cleaned.contains("User not found") || cleaned.contains("user not found") {
                        "Utente non trovato. Verifica username e riprova.".to_string()
                    } else if cleaned.contains("UNIQUE constraint failed") || cleaned.contains("UNIQUE constraint") {
                        // If we were attempting registration, show explicit registration-failed message
                        if !self.is_login {
                            // include attempted username when available
                            if !self.username.trim().is_empty() {
                                format!("Registrazione fallita: l'username '{}' è già in uso.", self.username.trim())
                            } else {
                                "Registrazione fallita: username già in uso. Scegli un altro username.".to_string()
                            }
                        } else {
                            "Username già in uso. Scegli un altro username.".to_string()
                        }
                    } else if cleaned.contains("incorrect password") || cleaned.contains("Invalid credentials") {
                        "Password errata. Riprova.".to_string()
                    } else if cleaned.contains("connection refused") || cleaned.contains("disconnected") {
                        "Impossibile connettersi al server. Controlla rete/host.".to_string()
                    } else {
                        // fallback: show a short summary
                        "Autenticazione fallita. Controlla i dati e riprova.".to_string()
                    };
                    // Do not set bottom inline error_message; rely on top logger only
                    self.error_message = None;
                    self.logger.clear();
                    self.logger.push(crate::client::gui::views::logger::LogMessage {
                        level: crate::client::gui::views::logger::LogLevel::Error,
                        message: format!("{}", user_friendly),
                    });
                    // Auto-clear after 5 seconds
                    return iced::Command::perform(async { tokio::time::sleep(std::time::Duration::from_secs(5)).await; () }, |_| Msg::ClearLog);
                }
            }
            Msg::Logout => {
                // Attempt to send a /logout command to the server (if we have a token).
                // Capture token before clearing local state.
                let token_to_send = if let Some(t) = self.session_token.clone() {
                    Some(t)
                } else {
                    session_store::load_session_token()
                };

                // Immediately clear persisted/local session and switch UI to registration.
                let _ = session_store::clear_session_token();
                self.session_token = None;
                self.app_state = AppState::Registration;
                self.welcome_message = None;
                self.logger.clear();
                self.logger.push(crate::client::gui::views::logger::LogMessage {
                    level: crate::client::gui::views::logger::LogLevel::Info,
                    message: "Logout effettuato con successo.".to_string(),
                });

                // If we have a token, asynchronously send /logout <token> using the shared ChatService.
                if let Some(token) = token_to_send {
                    // Resolve host selection similar to other flows
                    let cfg = ClientConfig::from_env();
                    let host = match self.selected_host {
                        HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                        HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                        HostType::Manual => self.manual_host.clone(),
                    };
                    let svc = chat_service.clone();
                    return iced::Command::perform(async move {
                        let mut guard = svc.lock().await;
                        match guard.send_command(&host, format!("/logout {}", token)).await {
                            Ok(resp) => Msg::LogInfo(format!("Logout response: {}", resp)),
                            Err(e) => Msg::LogError(format!("Logout failed: {}", e)),
                        }
                    }, |m| m);
                }

                // Fallback: keep the small clear-log timer
                return iced::Command::perform(async { tokio::time::sleep(std::time::Duration::from_secs(3)).await; () }, |_| Msg::ClearLog);
            }
            Msg::LogInfo(msg) => self.logger.push(crate::client::gui::views::logger::LogMessage {
                level: crate::client::gui::views::logger::LogLevel::Info,
                message: msg,
            }),
            Msg::LogSuccess(msg) => self.logger.push(crate::client::gui::views::logger::LogMessage {
                level: crate::client::gui::views::logger::LogLevel::Success,
                message: msg,
            }),
            Msg::LogError(msg) => self.logger.push(crate::client::gui::views::logger::LogMessage {
                level: crate::client::gui::views::logger::LogLevel::Error,
                message: msg,
            }),
            Msg::ClearLog => {
                self.logger.clear();
            }
            Msg::SessionMissing => {
                // No valid session found at startup — show registration/login
                self.loading = false;
                self.app_state = AppState::Registration;
            }
            Msg::OpenFriendRequests => {
                self.app_state = AppState::FriendRequests;
            }
            Msg::OpenMainActions => {
                // Stop polling when leaving private chat
                if matches!(self.app_state, AppState::PrivateChat(_)) {
                    self.polling_active = false;
                }
                self.app_state = AppState::MainActions;
            }
            Msg::OpenUsersList { kind } => {
                // when opening, fetch list from server (online or all) and store cache
                self.logger.push(crate::client::gui::views::logger::LogMessage { level: crate::client::gui::views::logger::LogLevel::Info, message: format!("Loading {} users...", kind) });
                if let Some(token) = self.session_token.clone() {
                    let cfg = ClientConfig::from_env();
                    let host = match self.selected_host {
                        HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                        HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                        HostType::Manual => self.manual_host.clone(),
                    };
                    let svc = chat_service.clone();
                    let kind_clone = kind.clone();
                    return iced::Command::perform(async move {
                        if kind_clone == "Online" {
                            match crate::client::services::users_service::UsersService::list_online(&svc, &host).await {
                                Ok(list) => Msg::UsersListLoaded { kind: kind_clone.clone(), list },
                                Err(e) => Msg::LogError(format!("Load users failed: {}", e)),
                            }
                        } else {
                            match crate::client::services::users_service::UsersService::list_all(&svc, &host).await {
                                Ok(list) => Msg::UsersListLoaded { kind: kind_clone.clone(), list },
                                Err(e) => Msg::LogError(format!("Load users failed: {}", e)),
                            }
                        }
                    }, |m| m);
                } else {
                    // no token: still open view but empty cache
                    self.app_state = AppState::UsersList(kind);
                }
            }
            Msg::UsersListLoaded { kind, list } => {
                // Exclude current user (case-insensitive) from cache and results
                let me = self.username.to_lowercase();
                let filtered: Vec<String> = list.into_iter().filter(|u| u.to_lowercase() != me).collect();
                self.users_list_cache = filtered.clone();
                let mut r = filtered;
                r.truncate(10);
                self.users_search_results = r;
                self.app_state = AppState::UsersList(kind);
            }
            Msg::UsersSearchQueryChanged(q) => {
                self.users_search_query = q;
            }
            Msg::UsersSearch => {
                // perform local filter on users_list_cache
                let q = self.users_search_query.to_lowercase();
                let me = self.username.to_lowercase();
                let mut results: Vec<String> = self.users_list_cache.iter()
                    .filter(|u| {
                        let lu = u.to_lowercase();
                        lu.contains(&q) && lu != me
                    })
                    .cloned()
                    .collect();
                results.truncate(10);
                self.users_search_results = results;
            }
            Msg::OpenPrivateChat(username) => {
                // Carica i messaggi per questa chat quando si apre
                let username_clone = username.clone();
                self.app_state = AppState::PrivateChat(username);
                
                // Se non abbiamo già i messaggi in cache, caricali
                if !self.private_chats.contains_key(&username_clone) {
                    if let Some(token) = self.session_token.clone() {
                        let cfg = ClientConfig::from_env();
                        let host = match self.selected_host {
                            HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                            HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                            HostType::Manual => self.manual_host.clone(),
                        };
                        let svc = chat_service.clone();
                        let token_clone = token.clone();
                        let user_clone = username_clone.clone();
                        return iced::Command::perform(async move {
                            match crate::client::services::friend_service::FriendService::get_private_messages(&svc, &host, &token_clone, &user_clone).await {
                                Ok(raw_messages) => {
                                    // Parsa i messaggi dal formato server
                                    let messages = parse_server_messages(&raw_messages, &user_clone);
                                    Msg::PrivateMessagesLoaded { with: user_clone, messages }
                                }
                                Err(e) => Msg::LogError(format!("Caricamento messaggi fallito: {}", e)),
                            }
                        }, |m| m);
                    }
                }
            }
            Msg::PrivateMessagesLoaded { with, messages } => {
                // Ordina i messaggi per timestamp prima di inserirli
                let mut sorted_messages = messages;
                sorted_messages.sort_by_key(|m| m.sent_at);
                self.private_chats.insert(with, sorted_messages);
            }
            Msg::MessageInputChanged(input) => {
                self.current_message_input = input;
            }
            Msg::SendPrivateMessage { to } => {
                if !self.current_message_input.trim().is_empty() {
                    if let Some(token) = self.session_token.clone() {
                        let cfg = ClientConfig::from_env();
                        let host = match self.selected_host {
                            HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                            HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                            HostType::Manual => self.manual_host.clone(),
                        };
                        let svc = chat_service.clone();
                        let token_clone = token.clone();
                        let to_clone = to.clone();
                        let message_content = self.current_message_input.clone();
                        let sender = self.username.clone();
                        
                        // Aggiungi immediatamente il messaggio alla cache locale (ottimistic update)
                        let now = chrono::Utc::now();
                        let chat_message = ChatMessage {
                            sender: sender.clone(),
                            content: message_content.clone(),
                            timestamp: now.format("%H:%M").to_string(),
                            sent_at: now.timestamp(),
                        };
                        
                        self.private_chats.entry(to_clone.clone()).or_insert_with(Vec::new).push(chat_message);
                        self.current_message_input.clear();
                        
                        return iced::Command::perform(async move {
                            match crate::client::services::friend_service::FriendService::send_private_message(&svc, &host, &token_clone, &to_clone, &message_content).await {
                                Ok(_) => Msg::LogInfo("Messaggio inviato".to_string()),
                                Err(e) => Msg::LogError(format!("Invio messaggio fallito: {}", e)),
                            }
                        }, |m| m);
                    }
                }
            }
            Msg::OpenGroupChat(group_id, group_name) => {
                self.app_state = AppState::GroupChat(group_id, group_name);
            }
            // Quick test network actions
            Msg::SendGroupMessageTest => {
                self.logger.push(crate::client::gui::views::logger::LogMessage { level: crate::client::gui::views::logger::LogLevel::Info, message: "Invio messaggio di gruppo (test)...".to_string() });
                                            // Trigger immediate refresh for the recipient
                                            Message::TriggerImmediateRefresh { with: to.clone() }
                // Perform async network call via shared ChatService
                if let Some(token) = self.session_token.clone() {
                    let cfg = ClientConfig::from_env();
                    let host = match self.selected_host {
                        HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                        HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                        HostType::Manual => self.manual_host.clone(),
                    };
                    let token_clone = token.clone();
                    let svc = chat_service.clone();
                    return iced::Command::perform(async move {
                        let res = crate::client::services::group_service::GroupService::send_group_message(&svc, &host, &token_clone, "GruppoDemo", "Test message from UI").await;
                        match res {
                            Ok(r) => Msg::LogInfo(format!("Group send: {}", r)),
                            Err(e) => Msg::LogError(format!("Group send failed: {}", e)),
                        }
                    }, |m| m);
                } else {
                    self.logger.push(crate::client::gui::views::logger::LogMessage { level: crate::client::gui::views::logger::LogLevel::Error, message: "No session token available".to_string() });
                }
            }
            Msg::GetGroupMessagesTest => {
                self.logger.push(crate::client::gui::views::logger::LogMessage { level: crate::client::gui::views::logger::LogLevel::Info, message: "Richiesta messaggi gruppo (test)...".to_string() });
                if let Some(token) = self.session_token.clone() {
                    let cfg = ClientConfig::from_env();
                    let host = match self.selected_host {
                        HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                        HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                        HostType::Manual => self.manual_host.clone(),
                    };
                    let token_clone = token.clone();
                    let svc = chat_service.clone();
                    return iced::Command::perform(async move {
                        match crate::client::services::group_service::GroupService::get_group_messages(&svc, &host, &token_clone, "GruppoDemo").await {
                            Ok(msgs) => Msg::LogInfo(format!("Got {} messages", msgs.len())),
                            Err(e) => Msg::LogError(format!("Get group failed: {}", e)),
                        }
                    }, |m| m);
                }
            }
            Msg::DeleteGroupMessagesTest => {
                self.logger.push(crate::client::gui::views::logger::LogMessage { level: crate::client::gui::views::logger::LogLevel::Info, message: "Cancellazione messaggi gruppo (test)...".to_string() });
                if let Some(token) = self.session_token.clone() {
                    let cfg = ClientConfig::from_env();
                    let host = match self.selected_host {
                        HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                        HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                        HostType::Manual => self.manual_host.clone(),
                    };
                    let token_clone = token.clone();
                    let svc = chat_service.clone();
                    return iced::Command::perform(async move {
                        match crate::client::services::group_service::GroupService::delete_group_messages(&svc, &host, &token_clone, "group-id-demo").await {
                            Ok(r) => Msg::LogInfo(format!("Delete group: {}", r)),
                            Err(e) => Msg::LogError(format!("Delete group failed: {}", e)),
                        }
                    }, |m| m);
                }
            }
            Msg::SendPrivateMessageTest => {
                self.logger.push(crate::client::gui::views::logger::LogMessage { level: crate::client::gui::views::logger::LogLevel::Info, message: "Invio messaggio privato (test)...".to_string() });
                if let Some(token) = self.session_token.clone() {
                    let cfg = ClientConfig::from_env();
                    let host = match self.selected_host {
                        HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                        HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                        HostType::Manual => self.manual_host.clone(),
                    };
                    let token_clone = token.clone();
                    let svc = chat_service.clone();
                    return iced::Command::perform(async move {
                        match crate::client::services::friend_service::FriendService::send_private_message(&svc, &host, &token_clone, "alice", "Hello from UI test").await {
                            Ok(r) => Msg::LogInfo(format!("Private send: {}", r)),
                            Err(e) => Msg::LogError(format!("Private send failed: {}", e)),
                        }
                    }, |m| m);
                }
            }
            // Friend system actions handlers
            Msg::SendFriendRequest { to, message } => {
                self.logger.push(crate::client::gui::views::logger::LogMessage { level: crate::client::gui::views::logger::LogLevel::Info, message: format!("Invio richiesta amicizia a {}...", to) });
                if let Some(token) = self.session_token.clone() {
                    let cfg = ClientConfig::from_env();
                    let host = match self.selected_host {
                        HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                        HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                        HostType::Manual => self.manual_host.clone(),
                    };
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    return iced::Command::perform(async move {
                        match crate::client::services::friend_service::FriendService::send_friend_request(&svc, &host, &token_clone, &to, &message).await {
                            Ok(r) => Msg::LogInfo(format!("Friend request: {}", r)),
                            Err(e) => Msg::LogError(format!("Friend request failed: {}", e)),
                        }
                    }, |m| m);
                }
            }
            Msg::AcceptFriendRequest { from } => {
                self.logger.push(crate::client::gui::views::logger::LogMessage { level: crate::client::gui::views::logger::LogLevel::Info, message: format!("Accetto richiesta amicizia da {}...", from) });
                if let Some(token) = self.session_token.clone() {
                    let cfg = ClientConfig::from_env();
                    let host = match self.selected_host {
                        HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                        HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                        HostType::Manual => self.manual_host.clone(),
                    };
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    return iced::Command::perform(async move {
                        match crate::client::services::friend_service::FriendService::accept_friend_request(&svc, &host, &token_clone, &from).await {
                            Ok(r) => Msg::LogInfo(format!("Accept request: {}", r)),
                            Err(e) => Msg::LogError(format!("Accept failed: {}", e)),
                        }
                    }, |m| m);
                }
            }
            Msg::RejectFriendRequest { from } => {
                self.logger.push(crate::client::gui::views::logger::LogMessage { level: crate::client::gui::views::logger::LogLevel::Info, message: format!("Rifiuto richiesta amicizia da {}...", from) });
                if let Some(token) = self.session_token.clone() {
                    let cfg = ClientConfig::from_env();
                    let host = match self.selected_host {
                        HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                        HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                        HostType::Manual => self.manual_host.clone(),
                    };
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    return iced::Command::perform(async move {
                        match crate::client::services::friend_service::FriendService::reject_friend_request(&svc, &host, &token_clone, &from).await {
                            Ok(r) => Msg::LogInfo(format!("Reject request: {}", r)),
                            Err(e) => Msg::LogError(format!("Reject failed: {}", e)),
                        }
                    }, |m| m);
                }
            }
            Msg::ListFriends => {
                self.logger.push(crate::client::gui::views::logger::LogMessage { level: crate::client::gui::views::logger::LogLevel::Info, message: "Richiesta lista amici...".to_string() });
                if let Some(token) = self.session_token.clone() {
                    let cfg = ClientConfig::from_env();
                    let host = match self.selected_host {
                        HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                        HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                        HostType::Manual => self.manual_host.clone(),
                    };
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    return iced::Command::perform(async move {
                        match crate::client::services::friend_service::FriendService::list_friends(&svc, &host, &token_clone).await {
                            Ok(list) => Msg::LogInfo(format!("Friends: {}", list.join(", "))),
                            Err(e) => Msg::LogError(format!("List friends failed: {}", e)),
                        }
                    }, |m| m);
                }
            }
            Msg::ListOnlineUsers => {
                self.logger.push(crate::client::gui::views::logger::LogMessage { level: crate::client::gui::views::logger::LogLevel::Info, message: "Richiesta utenti online...".to_string() });
                let cfg = ClientConfig::from_env();
                let host = match self.selected_host {
                    HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                    HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                    HostType::Manual => self.manual_host.clone(),
                };
                let svc = chat_service.clone();
                return iced::Command::perform(async move {
                    match crate::client::services::users_service::UsersService::list_online(&svc, &host).await {
                        Ok(list) => Msg::LogInfo(format!("Online: {}", list.join(", "))),
                        Err(e) => Msg::LogError(format!("List online failed: {}", e)),
                    }
                }, |m| m);
            }
            Msg::ListAllUsers => {
                self.logger.push(crate::client::gui::views::logger::LogMessage { level: crate::client::gui::views::logger::LogLevel::Info, message: "Richiesta lista utenti...".to_string() });
                let cfg = ClientConfig::from_env();
                let host = match self.selected_host {
                    HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                    HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                    HostType::Manual => self.manual_host.clone(),
                };
                let svc = chat_service.clone();
                return iced::Command::perform(async move {
                    match crate::client::services::users_service::UsersService::list_all(&svc, &host).await {
                        Ok(list) => Msg::LogInfo(format!("All users: {}", list.join(", "))),
                        Err(e) => Msg::LogError(format!("List all failed: {}", e)),
                    }
                }, |m| m);
            }
            Msg::CreateGroup { name } => {
                self.logger.push(crate::client::gui::views::logger::LogMessage { level: crate::client::gui::views::logger::LogLevel::Info, message: format!("Creazione gruppo {}...", name) });
                if let Some(token) = self.session_token.clone() {
                    let cfg = ClientConfig::from_env();
                    let host = match self.selected_host {
                        HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                        HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                        HostType::Manual => self.manual_host.clone(),
                    };
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    return iced::Command::perform(async move {
                        match crate::client::services::group_service::GroupService::create_group(&svc, &host, &token_clone, &name).await {
                            Ok(r) => Msg::LogInfo(format!("Create group: {}", r)),
                            Err(e) => Msg::LogError(format!("Create group failed: {}", e)),
                        }
                    }, |m| m);
                }
            }
            // Group invite and membership actions
            Msg::InviteToGroup { group_id, username } => {
                self.logger.push(crate::client::gui::views::logger::LogMessage { level: crate::client::gui::views::logger::LogLevel::Info, message: format!("Invito {} al gruppo {}...", username, group_id) });
                if let Some(token) = self.session_token.clone() {
                    let cfg = ClientConfig::from_env();
                    let host = match self.selected_host {
                        HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                        HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                        HostType::Manual => self.manual_host.clone(),
                    };
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    let group = group_id.clone();
                    let user = username.clone();
                    return iced::Command::perform(async move {
                        match crate::client::services::group_service::GroupService::invite(&svc, &host, &token_clone, &group, &user).await {
                            Ok(r) => Msg::LogInfo(format!("Invite: {}", r)),
                            Err(e) => Msg::LogError(format!("Invite failed: {}", e)),
                        }
                    }, |m| m);
                }
            }
            Msg::AcceptGroupInvite { invite_id } => {
                self.logger.push(crate::client::gui::views::logger::LogMessage { level: crate::client::gui::views::logger::LogLevel::Info, message: format!("Accetto invito gruppo {}...", invite_id) });
                if let Some(token) = self.session_token.clone() {
                    let cfg = ClientConfig::from_env();
                    let host = match self.selected_host {
                        HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                        HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                        HostType::Manual => self.manual_host.clone(),
                    };
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    let invite = invite_id.clone();
                    return iced::Command::perform(async move {
                        match crate::client::services::group_service::GroupService::accept_invite(&svc, &host, &token_clone, &invite).await {
                            Ok(r) => Msg::LogInfo(format!("Accept invite: {}", r)),
                            Err(e) => Msg::LogError(format!("Accept invite failed: {}", e)),
                        }
                    }, |m| m);
                }
            }
            Msg::RejectGroupInvite { invite_id } => {
                self.logger.push(crate::client::gui::views::logger::LogMessage { level: crate::client::gui::views::logger::LogLevel::Info, message: format!("Rifiuto invito gruppo {}...", invite_id) });
                if let Some(token) = self.session_token.clone() {
                    let cfg = ClientConfig::from_env();
                    let host = match self.selected_host {
                        HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                        HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                        HostType::Manual => self.manual_host.clone(),
                    };
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    let invite = invite_id.clone();
                    return iced::Command::perform(async move {
                        match crate::client::services::group_service::GroupService::reject_invite(&svc, &host, &token_clone, &invite).await {
                            Ok(r) => Msg::LogInfo(format!("Reject invite: {}", r)),
                            Err(e) => Msg::LogError(format!("Reject invite failed: {}", e)),
                        }
                    }, |m| m);
                }
            }
            Msg::MyGroupInvites => {
                self.logger.push(crate::client::gui::views::logger::LogMessage { level: crate::client::gui::views::logger::LogLevel::Info, message: "Richiesta miei inviti di gruppo...".to_string() });
                if let Some(token) = self.session_token.clone() {
                    let cfg = ClientConfig::from_env();
                    let host = match self.selected_host {
                        HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                        HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                        HostType::Manual => self.manual_host.clone(),
                    };
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    return iced::Command::perform(async move {
                        match crate::client::services::group_service::GroupService::my_invites(&svc, &host, &token_clone).await {
                            Ok(list) => Msg::LogInfo(format!("Invites: {}", list.join(" | "))),
                            Err(e) => Msg::LogError(format!("My invites failed: {}", e)),
                        }
                    }, |m| m);
                }
            }
            Msg::JoinGroup { group_id } => {
                self.logger.push(crate::client::gui::views::logger::LogMessage { level: crate::client::gui::views::logger::LogLevel::Info, message: format!("Join gruppo {}...", group_id) });
                if let Some(token) = self.session_token.clone() {
                    let cfg = ClientConfig::from_env();
                    let host = match self.selected_host {
                        HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                        HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                        HostType::Manual => self.manual_host.clone(),
                    };
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    let group = group_id.clone();
                    return iced::Command::perform(async move {
                        match crate::client::services::group_service::GroupService::join_group(&svc, &host, &token_clone, &group).await {
                            Ok(r) => Msg::LogInfo(format!("Join group: {}", r)),
                            Err(e) => Msg::LogError(format!("Join group failed: {}", e)),
                        }
                    }, |m| m);
                }
            }
            Msg::LeaveGroup { group_id } => {
                self.logger.push(crate::client::gui::views::logger::LogMessage { level: crate::client::gui::views::logger::LogLevel::Info, message: format!("Leave gruppo {}...", group_id) });
                if let Some(token) = self.session_token.clone() {
                    let cfg = ClientConfig::from_env();
                    let host = match self.selected_host {
                        HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                        HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                        HostType::Manual => self.manual_host.clone(),
                    };
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    let group = group_id.clone();
                    return iced::Command::perform(async move {
                        match crate::client::services::group_service::GroupService::leave_group(&svc, &host, &token_clone, &group).await {
                            Ok(r) => Msg::LogInfo(format!("Leave group: {}", r)),
                            Err(e) => Msg::LogError(format!("Leave group failed: {}", e)),
                        }
                    }, |m| m);
                }
            }
            Msg::MyGroups => {
                self.logger.push(crate::client::gui::views::logger::LogMessage { level: crate::client::gui::views::logger::LogLevel::Info, message: "Richiesta miei gruppi...".to_string() });
                if let Some(token) = self.session_token.clone() {
                    let cfg = ClientConfig::from_env();
                    let host = match self.selected_host {
                        HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                        HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                        HostType::Manual => self.manual_host.clone(),
                    };
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    return iced::Command::perform(async move {
                        match crate::client::services::group_service::GroupService::my_groups(&svc, &host, &token_clone).await {
                            Ok(list) => Msg::LogInfo(format!("My groups: {}", list.join(", "))),
                            Err(e) => Msg::LogError(format!("My groups failed: {}", e)),
                        }
                    }, |m| m);
                }
            }
            Msg::ReceivedFriendRequests => {
                self.logger.push(crate::client::gui::views::logger::LogMessage { level: crate::client::gui::views::logger::LogLevel::Info, message: "Richiesta richieste ricevute...".to_string() });
                if let Some(token) = self.session_token.clone() {
                    let cfg = ClientConfig::from_env();
                    let host = match self.selected_host {
                        HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                        HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                        HostType::Manual => self.manual_host.clone(),
                    };
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    return iced::Command::perform(async move {
                        match crate::client::services::friend_service::FriendService::received_friend_requests(&svc, &host, &token_clone).await {
                            Ok(list) => Msg::LogInfo(format!("Received requests: {}", list.join(" | "))),
                            Err(e) => Msg::LogError(format!("Received requests failed: {}", e)),
                        }
                    }, |m| m);
                }
            }
            Msg::SentFriendRequests => {
                self.logger.push(crate::client::gui::views::logger::LogMessage { level: crate::client::gui::views::logger::LogLevel::Info, message: "Richiesta richieste inviate...".to_string() });
                if let Some(token) = self.session_token.clone() {
                    let cfg = ClientConfig::from_env();
                    let host = match self.selected_host {
                        HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                        HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                        HostType::Manual => self.manual_host.clone(),
                    };
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    return iced::Command::perform(async move {
                        match crate::client::services::friend_service::FriendService::sent_friend_requests(&svc, &host, &token_clone).await {
                            Ok(list) => Msg::LogInfo(format!("Sent requests: {}", list.join(" | "))),
                            Err(e) => Msg::LogError(format!("Sent requests failed: {}", e)),
                        }
                    }, |m| m);
                }
            }
            Msg::GetPrivateMessagesTest => {
                self.logger.push(crate::client::gui::views::logger::LogMessage { level: crate::client::gui::views::logger::LogLevel::Info, message: "Richiesta messaggi privati (test)...".to_string() });
                if let Some(token) = self.session_token.clone() {
                    let cfg = ClientConfig::from_env();
                    let host = match self.selected_host {
                        HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                        HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                        HostType::Manual => self.manual_host.clone(),
                    };
                    let token_clone = token.clone();
                    let svc = chat_service.clone();
                    return iced::Command::perform(async move {
                        match crate::client::services::friend_service::FriendService::get_private_messages(&svc, &host, &token_clone, "alice").await {
                            Ok(msgs) => Msg::LogInfo(format!("Got {} private messages", msgs.len())),
                            Err(e) => Msg::LogError(format!("Get private failed: {}", e)),
                        }
                    }, |m| m);
                }
            }
            Msg::DeletePrivateMessagesTest => {
                self.logger.push(crate::client::gui::views::logger::LogMessage { level: crate::client::gui::views::logger::LogLevel::Info, message: "Cancellazione messaggi privati (test)...".to_string() });
                if let Some(token) = self.session_token.clone() {
                    let cfg = ClientConfig::from_env();
                    let host = match self.selected_host {
                        HostType::Localhost => format!("{}:{}", cfg.default_host, cfg.default_port),
                        HostType::Remote => format!("{}:{}", cfg.public_host, cfg.default_port),
                        HostType::Manual => self.manual_host.clone(),
                    };
                    let token_clone = token.clone();
                    let svc = chat_service.clone();
                    return iced::Command::perform(async move {
                        match crate::client::services::friend_service::FriendService::delete_private_messages(&svc, &host, &token_clone, "alice").await {
                            Ok(r) => Msg::LogInfo(format!("Delete private: {}", r)),
                            Err(e) => Msg::LogError(format!("Delete private failed: {}", e)),
                        }
                    }, |m| m);
                }
            }
            _ => {}
        }
        iced::Command::none()
    }
}

// Funzione helper per parsare i messaggi dal server
fn parse_server_messages(raw_messages: &[String], other_username: &str) -> Vec<ChatMessage> {
    raw_messages.iter().filter_map(|line| {
        // Formato atteso: "[timestamp] sender_id: message"
        if let Some(bracket_end) = line.find(']') {
            if let Some(colon_pos) = line[bracket_end..].find(':') {
                let timestamp_str = &line[1..bracket_end];
                let sender_part = &line[bracket_end + 2..bracket_end + colon_pos];
                let message_content = &line[bracket_end + colon_pos + 2..];
                
                // Converti timestamp Unix in formato leggibile
                if let Ok(timestamp) = timestamp_str.parse::<i64>() {
                    let datetime = chrono::DateTime::from_timestamp(timestamp, 0)?;
                    let formatted_time = datetime.format("%H:%M").to_string();
                    
                    // Determina se il messaggio è nostro o dell'altro utente
                    // Il server restituisce user_id, ma noi vogliamo username
                    let sender = if sender_part.len() > 10 { // Probabilmente un UUID
                        other_username.to_string() // Assumiamo sia dell'altro utente
                    } else {
                        sender_part.to_string()
                    };
                    
                    return Some(ChatMessage {
                        sender,
                        content: message_content.to_string(),
                        timestamp: formatted_time,
                        sent_at: timestamp,
                    });
                }
            }
        }
        None
}