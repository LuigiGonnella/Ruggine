use iced::{Application, Command, Element, Theme};
use crate::client::models::app_state::{AppState, ChatAppState};
use crate::client::models::messages::Message;
use crate::client::services::chat_service::ChatService;
use crate::client::services::message_parser::format_timestamp;
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
                        if response.starts_with("OK:") {
                            // Extract username from response for auto-login display
                            let username = response.trim_start_matches("OK:").trim();
                            Message::AuthResult { 
                                success: true, 
                                message: username.to_string(), 
                                token: Some(token) 
                            }
                        } else {
                            Message::SessionMissing
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
            Msg::SessionMissing => {
                // Token non valido o assente, vai alla schermata di registrazione
                println!("[APP] Sessione non valida, vai alla registrazione");
                self.state.app_state = AppState::Registration;
                self.state.loading = false;
                return Command::none();
            }
            Msg::AuthResult { success, message, token } => {
                self.state.loading = false;
                
                if success {
                    // Login/registrazione riuscita
                    if let Some(token) = token {
                        self.state.session_token = Some(token.clone());
                        
                        // Estrai l'username dal messaggio - gestisci diversi formati
                        let message_content = message.trim_start_matches("OK:").trim();
                        let username = if message_content.starts_with("Logged in as ") {
                            // Format: "Logged in as luigi"
                            message_content.strip_prefix("Logged in as ").unwrap_or(message_content)
                        } else if message_content.starts_with("Registered as ") {
                            // Format: "Registered as luigi"  
                            message_content.strip_prefix("Registered as ").unwrap_or(message_content)
                        } else {
                            // Fallback: assume the whole content is the username
                            message_content
                        };
                        
                        println!("🔴 [DEBUG] AuthResult in app.rs - message: '{}', extracted username: '{}'", message, username);
                        self.state.username = username.to_string();
                        
                        // Salva il token in modo sicuro
                        crate::client::utils::session_store::save_session_token(&token);
                        
                        // Imposta l'utente corrente nel ChatService
                        let svc = self.chat_service.clone();
                        let username_clone = username.to_string();
                        let token_clone = token.clone();
                        
                        // Avvia connessione WebSocket e inizia il loop di controllo messaggi
                        return Command::perform(
                            async move {
                                let mut guard = svc.lock().await;
                                guard.set_current_user(username_clone);
                                
                                // Connetti il WebSocket
                                let cfg = crate::server::config::ClientConfig::from_env();
                                let ws_port = cfg.default_port + 1; // WebSocket su porta +1
                                println!("[APP] Tentativo connessione WebSocket a {}:{}", cfg.default_host, ws_port);
                                match guard.connect_websocket(&cfg.default_host, ws_port, &token_clone).await {
                                    Ok(()) => {
                                        println!("[APP] WebSocket connesso, avviando controllo messaggi");
                                        Msg::WebSocketConnected
                                    }
                                    Err(e) => {
                                        println!("[APP] Errore connessione WebSocket: {}", e);
                                        Msg::WebSocketError { error: format!("WebSocket connection failed: {}", e) }
                                    }
                                }
                            },
                            |msg| msg,
                        );
                    }
                } else {
                    // Login/registrazione fallita
                    self.state.error_message = Some(message);
                }
                
                return Command::none();
            }
            Msg::WebSocketConnected => {
                // WebSocket connesso, passa alla schermata principale
                println!("[APP] WebSocket connesso, passando a MainActions");
                self.state.app_state = AppState::MainActions;
                
                // Aggiungi messaggio di successo e pulisci il logger
                use crate::client::gui::views::logger::{LogMessage, LogLevel};
                self.state.logger.push(LogMessage {
                    level: LogLevel::Success,
                    message: format!("Login effettuato con successo come {}", self.state.username),
                });
                
                // Pulisci il logger dopo un breve delay per mostrare il messaggio di successo
                let cleanup_delay = Command::perform(
                    async move {
                        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                        Msg::ClearLog
                    },
                    |msg| msg,
                );
                
                // Avvia il loop di controllo messaggi
                let websocket_loop = Command::perform(
                    async move {
                        println!("[APP] Starting WebSocket message loop...");
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        Msg::CheckWebSocketMessages
                    },
                    |msg| msg,
                );
                
                return Command::batch(vec![cleanup_delay, websocket_loop]);
            }
            Msg::WebSocketError { error } => {
                println!("[APP] Errore WebSocket: {}", error);
                // Potresti aggiungere gestione errori qui (retry, notifica utente, etc.)
                return Command::none();
            }
            Msg::StartMessagePolling { with } => {
                // Load initial messages for the private chat
                let svc = self.chat_service.clone();
                let token = self.state.session_token.clone().unwrap_or_default();
                let cfg = crate::server::config::ClientConfig::from_env();
                let username = with.clone();
                
                return Command::perform(
                    async move {
                        match svc.lock().await.get_private_messages(&cfg.default_host, &token, &username).await {
                            Ok(messages) => Msg::NewMessagesReceived { with: username, messages },
                            Err(e) => {
                                println!("[APP] Error loading initial messages for {}: {}", username, e);
                                Msg::NewMessagesReceived { with: username, messages: vec![] }
                            }
                        }
                    },
                    |msg| msg,
                );
            }
            Msg::StartGroupMessagePolling { group_id } => {
                // Group messages now use WebSocket real-time updates only (no polling)
                self.state.group_polling_active = false;
                return Command::<Message>::none();
            }
            Msg::StopGroupMessagePolling => {
                // Stop group polling and return to main actions view
                self.state.group_polling_active = false;
                self.state.app_state = AppState::MainActions;
                return Command::<Message>::none();
            }
            Msg::NewGroupMessagesReceived { group_id, messages } => {
                // Update group chat messages from WebSocket (no more polling)
                self.state.group_chats.insert(group_id.clone(), messages.to_vec());
                // clear loading flag when messages arrive
                self.state.loading_group_chats.remove(&group_id);
                return Command::<Message>::none();
            }
            Msg::TriggerImmediateGroupRefresh { group_id } => {
                // Group messages now use WebSocket real-time updates only (no manual refresh needed)
                return Command::<Message>::none();
            }
            Msg::StopMessagePolling => {
                // Stop polling and return to main actions view
                self.state.polling_active = false;
                self.state.app_state = AppState::MainActions;
                return Command::<Message>::none();
            }
            Msg::NewMessagesReceived { with, messages } => {
                println!("[APP] NewMessagesReceived for {}: {} messages", with, messages.len());
                if self.state.polling_active {
                    self.state.private_chats.insert(with.clone(), messages.to_vec());
                    // clear loading flag when messages arrive
                    self.state.loading_private_chats.remove(&with);
                    
                    println!("[APP] Updated private_chats cache for {}, total cached: {}", with, messages.len());
                    
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
                } else {
                    return Command::<Message>::none();
                }
            }
            Msg::TriggerImmediateRefresh { with } => {
                let cfg = crate::server::config::ClientConfig::from_env();
                let host = format!("{}:{}", cfg.default_host, cfg.default_port);
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
            Msg::CheckWebSocketMessages => {
                // Controlla se ci sono messaggi WebSocket in arrivo
                println!("[APP] Checking for WebSocket messages...");
                let svc = self.chat_service.clone();
                return Command::perform(
                    async move {
                        let mut guard = svc.lock().await;
                        if let Some(ws_message) = guard.try_receive_websocket_message().await {
                            println!("[APP] Found WebSocket message, forwarding to handler");
                            Msg::WebSocketMessageReceived(ws_message)
                        } else {
                            // Continua a controllare dopo un breve delay
                            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                            Msg::CheckWebSocketMessages
                        }
                    },
                    |msg| msg,
                );
            }
            Msg::WebSocketMessageReceived(ws_msg) => {
                // Forward the WebSocket message to app state for proper processing and UI refresh
                let state_update = self.state.update(Message::WebSocketMessageReceived(ws_msg), &self.chat_service);
                
                // Immediately restart the WebSocket message checking loop
                let restart_loop = Command::perform(
                    async move { Msg::CheckWebSocketMessages },
                    |msg| msg,
                );
                
                return Command::batch([state_update, restart_loop]);
            }
            _ => {}
        }
        
        // Handle friend request sending
        if let Msg::SendFriendRequestToUser { username, message } = &message {
            let cfg = crate::server::config::ClientConfig::from_env();
            let host = format!("{}:{}", cfg.default_host, cfg.default_port);
            let token = self.state.session_token.clone().unwrap_or_default();
            let svc = self.chat_service.clone();
            let username_clone = username.clone();
            let message_clone = message.clone();
            
            return Command::perform(
                async move {
                    let mut guard = svc.lock().await;
                    let cmd = format!("/send_friend_request {} {} {}", token, username_clone, message_clone);
                    match guard.send_command(&host, cmd).await {
                        Ok(response) => {
                            if response.starts_with("OK:") {
                                Msg::FriendRequestResult { success: true, message: "Friend request sent successfully!".to_string() }
                            } else {
                                Msg::FriendRequestResult { success: false, message: response }
                            }
                        }
                        Err(e) => Msg::FriendRequestResult { success: false, message: format!("Error: {}", e) }
                    }
                },
                |msg| msg,
            );
        }
        
        self.state.update(message, &self.chat_service)
    }

    fn view(&self) -> Element<Message> {
        match &self.state.app_state {
            AppState::CheckingSession => iced::widget::Text::new("Controllo sessione...").into(),
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
