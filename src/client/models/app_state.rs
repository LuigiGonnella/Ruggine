use std::collections::HashMap;
use crate::client::gui::views::registration::HostType;
use crate::client::gui::views::logger::LogMessage;
use crate::client::models::messages::Message;
use crate::client::services::chat_service::ChatService;
use std::sync::Arc;
use tokio::sync::Mutex;
use iced::Command;
use iced::widget::scrollable;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum AppState {
    #[default]
    CheckingSession,
    Registration,
    MainActions,
    PrivateChat(String),
    GroupChat(String, String),
    UsersList(String),
    FriendRequests,
    Chat,
    CreateGroup,
    MyGroups,
    InviteToGroup { group_id: String, group_name: String },
    MyGroupInvites,
    SendFriendRequest,
    ViewFriends,
}

// Helper function to extract username from friend request action messages
fn extract_username_from_friend_action(message: &str) -> Option<String> {
    // Try to extract username from success messages like "Friend request accepted from username"
    if message.contains("accepted from") {
        if let Some(username) = message.split("accepted from ").nth(1) {
            return Some(username.trim().to_string());
        }
    }
    if message.contains("rejected from") {
        if let Some(username) = message.split("rejected from ").nth(1) {
            return Some(username.trim().to_string());
        }
    }
    None
}
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub sender: String,
    pub content: String,
    pub timestamp: i64,
    pub formatted_time: String,
    pub sent_at: i64,
    /// True if this is a temporary local message awaiting server confirmation
    pub is_pending: bool,
}

#[derive(Debug, Clone)]
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
    /// Track the latest timestamp loaded via HTTP for each chat to avoid WebSocket duplicates
    pub last_http_timestamp: HashMap<String, i64>,
    pub polling_active: bool,
    /// Track if WebSocket message polling is active
    pub websocket_polling_active: bool,
    pub group_chats: HashMap<String, Vec<ChatMessage>>,
    pub loading_group_chats: std::collections::HashSet<String>,
    pub group_polling_active: bool,
    pub create_group_name: String,
    pub selected_participants: std::collections::HashSet<String>,
    pub my_groups: Vec<(String, String, usize)>, // (id, name, member_count)
    pub loading_groups: bool,
    pub my_group_invites: Vec<(i64, String, String)>, // (invite_id, group_name, invited_by)
    pub loading_invites: bool,
    pub friends_list: Vec<String>,
    pub friend_requests: Vec<(String, String)>, // (username, message)
}

impl Default for ChatAppState {
    fn default() -> Self {
        Self {
            app_state: AppState::default(),
            username: String::new(),
            password: String::new(),
            selected_host: HostType::default(),
            manual_host: String::new(),
            is_login: false,
            loading: false,
            error_message: None,
            session_token: None,
            show_password: false,
            logger: Vec::new(),
            users_search_query: String::new(),
            users_search_results: Vec::new(),
            current_message_input: String::new(),
            private_chats: HashMap::new(),
            loading_private_chats: std::collections::HashSet::new(),
            last_http_timestamp: HashMap::new(),
            polling_active: false,
            websocket_polling_active: false,
            group_chats: HashMap::new(),
            loading_group_chats: std::collections::HashSet::new(),
            group_polling_active: false,
            create_group_name: String::new(),
            selected_participants: std::collections::HashSet::new(),
            my_groups: Vec::new(),
            loading_groups: false,
            my_group_invites: Vec::new(),
            loading_invites: false,
            friends_list: Vec::new(),
            friend_requests: Vec::new(),
        }
    }
}

impl ChatAppState {
    pub fn update(&mut self, message: Message, chat_service: &Arc<Mutex<ChatService>>) -> Command<Message> {
        use crate::client::gui::views::logger::{LogMessage, LogLevel};
        use crate::client::utils::session_store;
        use crate::client::services::users_service::UsersService;
        
        match message {
            Message::NoOp => {
                // No operation - just return none
                return Command::none();
            }
            Message::ManualHostChanged(host) => {
                self.manual_host = host;
            }
            Message::UsernameChanged(username) => {
                println!("🔵 [DEBUG] UsernameChanged event triggered");
                println!("🔵 [DEBUG] Previous username: '{}'", self.username);
                println!("🔵 [DEBUG] New username: '{}'", username);
                self.username = username.clone();
                println!("🔵 [DEBUG] After update - username: '{}'", self.username);
            }
            Message::PasswordChanged(password) => {
                self.password = password;
            }
            Message::ToggleShowPassword => {
                self.show_password = !self.show_password;
            }
            Message::HostSelected(host_type) => {
                self.selected_host = host_type;
            }
            Message::ToggleLoginRegister => {
                self.is_login = !self.is_login;
                self.error_message = None;
            }
            Message::AuthResult { success, message, token } => {
                println!("🟢 [DEBUG] AuthResult received - success: {}, message: '{}', token present: {}", success, message, token.is_some());
                println!("🟢 [DEBUG] Current username before AuthResult: '{}'", self.username);
                self.loading = false;
                if success {
            
                                    
                    if let Some(t) = token {
                        self.session_token = Some(t.clone());
                        // Save token securely
                                if let Err(_e) = session_store::save_session_token(&t) {
                                    // Failed to save session token to secure store; ignore (non-fatal)
                                }
                        
                        // Extract username from success message for auto-login cases
                        if message.starts_with("OK:") {
                            println!("🟡 [DEBUG] Processing server response: '{}'", message);
                            let username_part = message.trim_start_matches("OK:").trim();
                            println!("🟡 [DEBUG] After removing 'OK:': '{}'", username_part);
                            
                            // Check for different response formats:
                            let actual_username = if username_part.contains("Logged in as") {
                                // Format: "Logged in as luigi SESSION: ..."
                                if let Some(after_logged_in) = username_part.strip_prefix("Logged in as ") {
                                    // Extract just the username part before " SESSION:"
                                    let username_only = after_logged_in.split(" SESSION:").next()
                                        .unwrap_or(after_logged_in)
                                        .trim();
                                    println!("🟡 [DEBUG] Extracted from 'Logged in as': '{}'", username_only);
                                    username_only
                                } else {
                                    // Fallback: split by "logged in as" and take the last part
                                    let extracted = username_part.split("logged in as").last()
                                        .map(|s| s.trim())
                                        .unwrap_or("");
                                    println!("🟡 [DEBUG] Fallback extraction: '{}'", extracted);
                                    extracted
                                }
                            } else {
                                // Format: just the username
                                println!("🟡 [DEBUG] No 'Logged in as' found, using whole part: '{}'", username_part);
                                username_part
                            };
                            
                            println!("🟡 [DEBUG] Final extracted username: '{}'", actual_username);
                            println!("🟡 [DEBUG] Current username state: '{}'", self.username);
                            println!("🟡 [DEBUG] Should set username? (actual_username not empty: {}, self.username empty: {})", 
                                    !actual_username.is_empty(), self.username.is_empty());
                            
                            if !actual_username.is_empty() && self.username.is_empty() {
                                println!("🟡 [DEBUG] Setting username from server response to: '{}'", actual_username);
                                self.username = actual_username.to_string();
                                println!("🟡 [DEBUG] After setting - username: '{}'", self.username);
                            } else {
                                println!("🟡 [DEBUG] NOT setting username from server (actual_username: '{}', username empty: {})", 
                                        actual_username, self.username.is_empty());
                            }
                        } else {
                            println!("🟡 [DEBUG] Server response does not start with 'OK:': '{}'", message);
                        }
                    }
                    println!("🟢 [DEBUG] About to transition to MainActions - username: '{}'", self.username);
                    self.app_state = AppState::MainActions;
                    // Clear any previous error messages and logger for clean transition
                    self.error_message = None;
                    self.logger.clear();
                    self.logger.push(LogMessage {
                        level: LogLevel::Success,
                        message: "Login successful".to_string(),
                    });

                    // Initialize WebSocket connection after successful authentication
                    let ws_svc = chat_service.clone();
                    let ws_token = self.session_token.clone().unwrap_or_default();
                    let ws_config = crate::server::config::ClientConfig::from_env();
                    
                    return Command::batch([
                        // Connect to WebSocket for real-time messaging
                        Command::perform(
                            async move {
                                let mut guard = ws_svc.lock().await;
                                match guard.connect_websocket(&ws_config.websocket_host, ws_config.websocket_port, &ws_token).await {
                                    Ok(_) => Message::WebSocketConnected,
                                    Err(e) => Message::WebSocketError { error: format!("WebSocket connection failed: {}", e) }
                                }
                            },
                            |msg| msg,
                        ),
                        // Auto-clear logger after 2 seconds (same behavior as other views)
                        Command::perform(
                            async move {
                                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                                Message::ClearLog
                            },
                            |msg| msg,
                        )
                    ]);
                } else {
                    self.error_message = Some(message.clone());
                    self.logger.clear(); // Clear previous messages
                    self.logger.push(LogMessage {
                        level: LogLevel::Error,
                        message: message.clone(),
                    });
                    
                    // Auto-clear error logger after 2 seconds to match other flows
                    return Command::perform(
                        async move {
                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                            Message::ClearLog
                        },
                        |msg| msg,
                    );
                }
            
            }
            Message::SessionMissing => {
                self.app_state = AppState::Registration;
                self.logger.clear(); // Clear any previous messages
            }
            Message::Logout => {
                // Clear session token from secure storage
                let _ = session_store::clear_session_token();
                
                // Get session token and host for server logout
                let session_token = self.session_token.clone().unwrap_or_default();
                let host = "127.0.0.1:5000".to_string(); // TODO: get from config
                
                // Logout from server and reset ChatService
                let svc = chat_service.clone();
                return Command::perform(
                    async move {
                        let mut guard = svc.lock().await;
                        match guard.logout(&host, &session_token).await {
                            Ok(_) => println!("[APP] Server logout and ChatService reset completed"),
                            Err(e) => println!("[APP] Logout error: {}, but continuing", e),
                        }
                        Message::LogoutCompleted
                    },
                    |msg| msg,
                );
            }
            Message::LogoutCompleted => {
                // Show logout message temporarily
                self.logger.clear();
                self.logger.push(LogMessage {
                    level: LogLevel::Info,
                    message: "Logout successful".to_string(),
                });
                
                // Reset state including WebSocket polling
                self.session_token = None;
                self.username.clear();
                self.password.clear();
                self.websocket_polling_active = false;  // Stop WebSocket polling
                self.app_state = AppState::Registration;
                self.websocket_polling_active = false; // Stop WebSocket polling
                
                // Clear all cached private chats to force reload on next login
                self.private_chats.clear();
                self.loading_private_chats.clear();
                println!("[APP] 🧹 Cleared all cached private chats and loading states");
                
                // Clear all cached group chats to force reload on next login
                self.group_chats.clear();
                self.loading_group_chats.clear();
                println!("[APP] 🧹 Cleared all cached group chats and loading states");
                
                // Clear logger after a delay for temporary logout message
                return Command::perform(
                    async move {
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        Message::ClearLog
                    },
                    |msg| msg,
                );
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
                
                // If we already have messages cached, don't mark as loading
                if !self.private_chats.contains_key(&username) {
                    self.loading_private_chats.insert(username.clone());
                    
                    // Load messages once - with WebSocket connected, no need for polling
                    return Command::perform(
                        async move { Message::LoadPrivateMessages { with: username } },
                        |msg| msg,
                    );
                }
                
                return Command::none();
            }
            Message::OpenGroupChat(group_id, group_name) => {
                self.app_state = AppState::GroupChat(group_id.clone(), group_name.clone());
                self.current_message_input.clear();
                // Mark this group chat as loading so the UI shows a loader
                self.loading_group_chats.insert(group_id.clone());

                // Load initial messages via WebSocket (no polling needed)
                return Command::perform(
                    async move { Message::LoadGroupMessages { group_id } },
                    |msg| msg,
                );
            }
            Message::OpenUsersList { kind } => {
                self.app_state = AppState::UsersList(kind.clone());
                self.users_search_query.clear();
                self.users_search_results.clear();
                
                // Auto-load users based on kind
                let svc = chat_service.clone();
                let cfg = crate::server::config::ClientConfig::from_env();
                let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                let token = self.session_token.clone().unwrap_or_default();
                
                return Command::perform(
                    async move {
                        let result = if kind == "Online" {
                            UsersService::list_online(&svc, &host, &token).await
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
            Message::OpenCreateGroup => {
                self.app_state = AppState::CreateGroup;
                self.create_group_name.clear();
                self.selected_participants.clear();
                self.users_search_query.clear();
                self.users_search_results.clear();
                
                // Auto-load all users for participant selection
                let svc = chat_service.clone();
                let cfg = crate::server::config::ClientConfig::from_env();
                let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                
                return Command::perform(
                    async move {
                        match UsersService::list_all(&svc, &host).await {
                            Ok(users) => Message::UsersListLoaded { kind: "CreateGroup".to_string(), list: users },
                            Err(_) => Message::UsersListLoaded { kind: "CreateGroup".to_string(), list: vec![] },
                        }
                    },
                    |msg| msg,
                );
            }
            Message::OpenMyGroups => {
                self.app_state = AppState::MyGroups;
                self.loading_groups = true;
                self.my_groups.clear();
                
                // Load user's groups
                if let Some(token) = &self.session_token {
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    let cfg = crate::server::config::ClientConfig::from_env();
                    let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                    
                    return Command::perform(
                        async move {
                            let mut guard = svc.lock().await;
                            match guard.send_command(&host, format!("/my_groups {}", token_clone)).await {
                                Ok(response) => {
                                    if response.starts_with("OK: My groups:") {
                                        let groups_part = response.trim_start_matches("OK: My groups:").trim();
                                        let groups: Vec<(String, String, usize)> = if groups_part.is_empty() {
                                            vec![]
                                        } else {
                                            groups_part.split(',').filter_map(|s| {
                                                let s = s.trim();
                                                if let Some((id, name)) = s.split_once(':') {
                                                    // For now, set member count to 1 (will be improved with server support)
                                                    Some((id.to_string(), name.to_string(), 1))
                                                } else {
                                                    None
                                                }
                                            }).collect()
                                        };
                                        Message::MyGroupsLoaded { groups }
                                    } else {
                                        Message::MyGroupsLoaded { groups: vec![] }
                                    }
                                }
                                Err(_) => Message::MyGroupsLoaded { groups: vec![] },
                            }
                        },
                        |msg| msg,
                    );
                }
            }
            Message::OpenInviteToGroup { group_id, group_name } => {
                self.app_state = AppState::InviteToGroup { group_id: group_id.clone(), group_name };
                self.users_search_query.clear();
                self.users_search_results.clear();
                
                // Auto-load all users for invitation
                let svc = chat_service.clone();
                let cfg = crate::server::config::ClientConfig::from_env();
                let host = format!("{}:{}", cfg.default_host, cfg.default_port);
        let _host = format!("{}:{}", cfg.default_host, cfg.default_port);
        let group_id_for_filter = group_id.clone();
        let token_clone = self.session_token.clone().unwrap_or_default();
                
                return Command::perform(
                    async move {
                        // Get all users and group members to filter
                        let all_users = UsersService::list_all(&svc, &host).await.unwrap_or_default();
                        
                        // Get group members to filter them out
                        let mut guard = svc.lock().await;
            let group_members_resp = guard.send_command(&host, format!("/group_members {} {}", token_clone, group_id_for_filter)).await.unwrap_or_default();
                        drop(guard);
                        
                        // Parse group members (format "OK: Group members: user1, user2")
                        let existing_members: Vec<String> = if group_members_resp.starts_with("OK: Group members:") {
                            group_members_resp.trim_start_matches("OK: Group members:").trim()
                                .split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
                        } else {
                            vec![]
                        };
                        
                        println!("[INVITE] Group members response: {}", group_members_resp);
                        println!("[INVITE] Parsed existing members: {:?}", existing_members);
                        println!("[INVITE] All users before filter: {:?}", all_users);
                        
                        // Filter out existing members and current user
                        let filtered_users: Vec<String> = all_users.into_iter()
                            .filter(|user| !existing_members.contains(user))
                            .collect();
                        
                        println!("[INVITE] Filtered users (available to invite): {:?}", filtered_users);
                        
                        Message::UsersListLoaded { kind: "Invite".to_string(), list: filtered_users }
                    },
                    |msg| msg,
                );
            }
            Message::OpenSendFriendRequest => {
                self.app_state = AppState::SendFriendRequest;
                self.users_search_query.clear();
                self.users_search_results.clear();
                
                // Auto-load all users for friend request
                let svc = chat_service.clone();
                let cfg = crate::server::config::ClientConfig::from_env();
                let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                
                return Command::perform(
                    async move {
                        match UsersService::list_all(&svc, &host).await {
                            Ok(users) => Message::UsersListLoaded { kind: "FriendRequest".to_string(), list: users },
                            Err(_) => Message::UsersListLoaded { kind: "FriendRequest".to_string(), list: vec![] },
                        }
                    },
                    |msg| msg,
                );
            }
            Message::OpenViewFriends => {
                self.app_state = AppState::ViewFriends;
                self.loading = true;
                
                // Load user's friends
                if let Some(token) = &self.session_token {
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    let cfg = crate::server::config::ClientConfig::from_env();
                    let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                    
                    return Command::perform(
                        async move {
                            let mut guard = svc.lock().await;
                            match guard.send_command(&host, format!("/list_friends {}", token_clone)).await {
                                Ok(response) => {
                                    if response.starts_with("OK: Friends:") {
                                        let friends_part = response.trim_start_matches("OK: Friends:").trim();
                                        let friends: Vec<String> = if friends_part.is_empty() {
                                            vec![]
                                        } else {
                                            friends_part.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
                                        };
                                        Message::FriendsLoaded { friends }
                                    } else {
                                        Message::FriendsLoaded { friends: vec![] }
                                    }
                                }
                                Err(_) => Message::FriendsLoaded { friends: vec![] },
                            }
                        },
                        |msg| msg,
                    );
                }
            }
            Message::OpenFriendRequests => {
                self.app_state = AppState::FriendRequests;
                self.loading = true;
                
                // Load user's friend requests
                if let Some(token) = &self.session_token {
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    let cfg = crate::server::config::ClientConfig::from_env();
                    let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                    
                    return Command::perform(
                        async move {
                            let mut guard = svc.lock().await;
                            match guard.send_command(&host, format!("/received_friend_requests {}", token_clone)).await {
                                Ok(response) => {
                                    if response.starts_with("OK: Richieste ricevute:") {
                                        let requests_part = response.trim_start_matches("OK: Richieste ricevute:").trim();
                                        let requests: Vec<(String, String)> = if requests_part.is_empty() {
                                            vec![]
                                        } else {
                                            requests_part.split(" | ").filter_map(|s| {
                                                if let Some((username, message)) = s.trim().split_once(':') {
                                                    Some((username.trim().to_string(), message.trim().to_string()))
                                                } else {
                                                    None
                                                }
                                            }).collect()
                                        };
                                        Message::FriendRequestsLoaded { requests }
                                    } else {
                                        Message::FriendRequestsLoaded { requests: vec![] }
                                    }
                                }
                                Err(_) => Message::FriendRequestsLoaded { requests: vec![] },
                            }
                        },
                        |msg| msg,
                    );
                }
            }
            Message::RejectFriendRequestFromUser { username } => {
                if let Some(token) = &self.session_token {
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    let cfg = crate::server::config::ClientConfig::from_env();
                    let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                    
                    return Command::perform(
                        async move {
                            let mut guard = svc.lock().await;
                            match guard.send_command(&host, format!("/reject_friend_request {} {}", token_clone, username)).await {
                                Ok(response) => {
                                    if response.starts_with("OK:") {
                                        Message::FriendRequestResult { 
                                            success: true, 
                                            message: format!("Friend request from {} rejected.", username) 
                                        }
                                    } else {
                                        Message::FriendRequestResult { 
                                            success: false, 
                                            message: response 
                                        }
                                    }
                                }
                                Err(e) => Message::FriendRequestResult { 
                                    success: false, 
                                    message: format!("Error rejecting friend request: {}", e) 
                                },
                            }
                        },
                        |msg| msg,
                    );
                }
            }
            Message::AcceptFriendRequestFromUser { username } => {
                let token = self.session_token.clone().unwrap_or_default();
                let svc = chat_service.clone();
                let username_clone = username.clone();
                let cfg = crate::server::config::ClientConfig::from_env();
                let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                return Command::perform(
                    async move {
                        let mut guard = svc.lock().await;
                        let cmd = format!("/accept_friend_request {} {}", token, username_clone);
                        match guard.send_command(&host, cmd).await {
                            Ok(response) => {
                                if response.starts_with("OK:") {
                                    Message::FriendRequestResult { success: true, message: "Friend request accepted!".to_string() }
                                } else {
                                    Message::FriendRequestResult { success: false, message: response }
                                }
                            }
                            Err(e) => Message::FriendRequestResult { success: false, message: format!("Error: {}", e) }
                        }
                    },
                    |msg| msg,
                );
            }
            Message::FriendsLoaded { friends } => {
                self.loading = false;
                self.friends_list = friends;
            }
            Message::FriendRequestsLoaded { requests } => {
                self.loading = false;
                self.friend_requests = requests;

                // Auto-clear logger after 2 seconds
                return Command::perform(
                    async move {
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        Message::ClearLog
                    },
                    |msg| msg,
                );
            }
            Message::FriendRequestResult { success, message } => {
                self.logger.push(LogMessage {
                    level: if success { LogLevel::Success } else { LogLevel::Error },
                    message: message.clone(),
                });

                // Remove the processed friend request from the list immediately
                    // This ensures the UI updates instantly without waiting for reload
                    // The message will be parsed to extract the username that was processed
                    if let Some(processed_username) = extract_username_from_friend_action(&message) {
                        self.friend_requests.retain(|(username, _)| username != &processed_username);
                    }
                    // Reload friend requests to remove the processed one
                    let cfg = crate::server::config::ClientConfig::from_env();
                    let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                    let token = self.session_token.clone().unwrap_or_default();
                    let svc = chat_service.clone();
                     return iced::Command::perform(
                        async move {
                            let mut guard = svc.lock().await;
                            let cmd = format!("/received_friend_requests {}", token);
                            match guard.send_command(&host, cmd).await {
                                Ok(response) => {
                                    if response.starts_with("OK:") {
                                        // Parse the response to extract friend requests
                                        let after = response.splitn(3, ':').nth(2).unwrap_or("");
                                        let requests: Vec<(String, String)> = after
                                            .split('|')
                                            .filter_map(|item| {
                                                let item = item.trim();
                                                if item.is_empty() { return None; }
                                                if let Some(colon_pos) = item.find(':') {
                                                    let username = item[..colon_pos].trim().to_string();
                                                    let message = item[colon_pos + 1..].trim().to_string();
                                                    Some((username, message))
                                                } else {
                                                    None
                                                }
                                            })
                                            .collect();
                                        Message::FriendRequestsLoaded { requests }
                                    } else {
                                        Message::FriendRequestsLoaded { requests: vec![] }
                                    }
                                }
                                Err(_) => Message::FriendRequestsLoaded { requests: vec![] }
                            }
                        },
                        |msg| msg,
                    );
                
                
            }
            Message::InviteToGroupResult { success, message } => {
                self.logger.push(LogMessage {
                    level: if success { LogLevel::Success } else { LogLevel::Error },
                    message: message.clone(),
                });
                
                // Auto-clear logger after 2 seconds
                return Command::perform(
                    async move {
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        Message::ClearLog
                    },
                    |msg| msg,
                );
            }
            Message::GroupInviteActionResult { success, message } => {
                self.logger.push(LogMessage {
                    level: if success { LogLevel::Success } else { LogLevel::Error },
                    message: message.clone(),
                });
                
                // Auto-clear logger after 2 seconds and refresh if successful
                if success {
                    return Command::batch([
                        Command::perform(
                            async move {
                                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                                Message::ClearLog
                            },
                            |msg| msg,
                        ),
                        Command::perform(
                            async move { Message::OpenMyGroupInvites },
                            |msg| msg,
                        )
                    ]);
                } else {
                    return Command::perform(
                        async move {
                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                            Message::ClearLog
                        },
                        |msg| msg,
                    );
                }
            }
            Message::CreateGroupInputChanged(name) => {
                self.create_group_name = name;
            }
            Message::ToggleParticipant(username) => {
                if self.selected_participants.contains(&username) {
                    self.selected_participants.remove(&username);
                } else {
                    self.selected_participants.insert(username);
                }
            }
            Message::RemoveParticipant(username) => {
                self.selected_participants.remove(&username);
            }
            Message::CreateGroupSubmit => {
                if !self.create_group_name.trim().is_empty() && !self.selected_participants.is_empty() {
                    if let Some(token) = &self.session_token {
                        let svc = chat_service.clone();
                        let token_clone = token.clone();
                        let name_clone = self.create_group_name.trim().to_string();
                        let participants = self.selected_participants.clone();
                        let cfg = crate::server::config::ClientConfig::from_env();
                        let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                        
                        self.loading = true;
                        
                        return Command::perform(
                            async move {
                                let mut guard = svc.lock().await;
                                let participants_str = participants.into_iter().collect::<Vec<_>>().join(",");
                                match guard.send_command(&host, format!("/create_group {} {} {}", token_clone, name_clone, participants_str)).await {
                                    Ok(response) => {
                                        // Extract group_id from response: "OK: Group 'name' created with ID: uuid"
                                        if let Some(id_part) = response.split("ID: ").nth(1) {
                                            let group_id = id_part.trim().to_string();
                                            Message::GroupCreated { group_id, group_name: name_clone }
                                        } else {
                                            // Fallback: generate a temporary ID (shouldn't happen)
                                            Message::GroupCreated { group_id: format!("temp_{}", chrono::Utc::now().timestamp()), group_name: name_clone }
                                        }
                                        
                                    }
                                    Err(e) => Message::LogError(format!("Errore nella creazione del gruppo: {}", e)),
                                }
                            },
                            |msg| msg,
                        );
                    }
                }
            }
            Message::GroupCreated { group_id, group_name } => {
                self.loading = false;
                self.logger.push(LogMessage {
                    level: LogLevel::Success,
                    message: format!("Group '{}' successfully created!", group_name),
                });
                
                // Navigate to the newly created group and auto-clear logger
                return Command::batch([
                    Command::perform(
                        async move { Message::OpenGroupChat(group_id, group_name) },
                        |msg| msg,
                    ),
                    Command::perform(
                        async move {
                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                            Message::ClearLog
                        },
                        |msg| msg,
                    )
                ]);
                
            }
            Message::MyGroupsLoaded { groups } => {
                self.loading_groups = false;
                self.my_groups = groups;
            }
            Message::InviteUserToGroup { group_id, username } => {
                if let Some(token) = &self.session_token {
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    let group_id_clone = group_id.clone();
                    let username_clone = username.clone();
                    let cfg = crate::server::config::ClientConfig::from_env();
                    let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                    
                    return Command::perform(
                        async move {
                            let mut guard = svc.lock().await;
                            match guard.send_command(&host, format!("/invite {} {} {}", token_clone, username_clone, group_id_clone)).await {
                                Ok(response) => {
                                    if response.starts_with("OK:") {
                                        Message::InviteToGroupResult { 
                                            success: true, 
                                            message: format!("Invite successfully sent toa {}!", username_clone) 
                                        }
                                    } else {
                                        Message::InviteToGroupResult { 
                                            success: false, 
                                            message: response 
                                        }
                                    }
                                }
                                Err(e) => Message::InviteToGroupResult { 
                                    success: false, 
                                    message: format!("Error in message sending: {}", e) 
                                },
                            }
                        },
                        |msg| msg,
                    );
                }
            }
            Message::OpenMyGroupInvites => {
                self.app_state = AppState::MyGroupInvites;
                self.loading_invites = true;
                self.my_group_invites.clear();
                
                // Load user's group invites
                if let Some(token) = &self.session_token {
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    let cfg = crate::server::config::ClientConfig::from_env();
                    let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                    
                    return Command::perform(
                        async move {
                            let mut guard = svc.lock().await;
                            match guard.send_command(&host, format!("/my_group_invites {}", token_clone)).await {
                                Ok(response) => {
                                    if response.starts_with("OK: Group invites:") {
                                        let invites_part = response.trim_start_matches("OK: Group invites:").trim();
                                        let invites: Vec<(i64, String, String)> = if invites_part.is_empty() {
                                            vec![]
                                        } else {
                                            invites_part.split(" | ").filter_map(|s| {
                                                let parts: Vec<&str> = s.trim().split(':').collect();
                                                if parts.len() == 3 {
                                                    if let Ok(invite_id) = parts[0].parse::<i64>() {
                                                        Some((invite_id, parts[1].to_string(), parts[2].to_string()))
                                                    } else {
                                                        None
                                                    }
                                                } else {
                                                    None
                                                }
                                            }).collect()
                                        };
                                        Message::MyGroupInvitesLoaded { invites }
                                    } else {
                                        Message::MyGroupInvitesLoaded { invites: vec![] }
                                    }
                                }
                                Err(_) => Message::MyGroupInvitesLoaded { invites: vec![] },
                            }
                        },
                        |msg| msg,
                    );
                }
            }
            Message::MyGroupInvitesLoaded { invites } => {
                self.loading_invites = false;
                self.my_group_invites = invites;
            }
            Message::AcceptGroupInvite { invite_id } => {
                if let Some(token) = &self.session_token {
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    let cfg = crate::server::config::ClientConfig::from_env();
                    let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                    
                    return Command::perform(
                        async move {
                            let mut guard = svc.lock().await;
                            match guard.send_command(&host, format!("/accept_group_invite {} {}", token_clone, invite_id)).await {
                                Ok(response) => {
                                    if response.starts_with("OK:") {
                                        Message::GroupInviteActionResult { 
                                            success: true, 
                                            message: "Invite accepted!".to_string() 
                                        }
                                    } else {
                                        Message::GroupInviteActionResult { 
                                            success: false, 
                                            message: response 
                                        }
                                    }
                                }
                                Err(e) => Message::GroupInviteActionResult { 
                                    success: false, 
                                    message: format!("Error in accepting the invite: {}", e) 
                                },
                            }
                        },
                        |msg| msg,
                    );
                }
            }
            Message::RejectGroupInvite { invite_id } => {
                if let Some(token) = &self.session_token {
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    let cfg = crate::server::config::ClientConfig::from_env();
                    let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                    
                    return Command::perform(
                        async move {
                            let mut guard = svc.lock().await;
                            match guard.send_command(&host, format!("/reject_group_invite {} {}", token_clone, invite_id)).await {
                                Ok(response) => {
                                    if response.starts_with("OK:") {
                                        Message::GroupInviteActionResult { 
                                            success: true, 
                                            message: "Invito rejected.".to_string() 
                                        }
                                    } else {
                                        Message::GroupInviteActionResult { 
                                            success: false, 
                                            message: response 
                                        }
                                    }
                                }
                                Err(e) => Message::GroupInviteActionResult { 
                                    success: false, 
                                    message: format!("Error in rejecting the invite: {}", e) 
                                },
                            }
                        },
                        |msg| msg,
                    );
                }
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
            Message::UsersListFiltered { list } => {
                self.users_search_results = list.clone();
                return Command::none();
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
            Message::MyGroups => {
                return Command::perform(
                    async { Message::OpenMyGroups },
                    |msg| msg,
                );
            }
            Message::CreateGroup { name: _ } => {
                return Command::perform(
                    async { Message::OpenCreateGroup },
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
                        
                        // Create a local message to add immediately to the UI
                        let local_msg = ChatMessage {
                            sender: self.username.clone(),
                            content: message.clone(),
                            timestamp: chrono::Utc::now().timestamp(),
                            formatted_time: chrono::Utc::now().format("%H:%M").to_string(),
                            sent_at: chrono::Utc::now().timestamp(),
                            is_pending: true,  // This is a temporary local message
                        };
                        
                        // Add message to local cache immediately for instant UI feedback
                        let messages = self.private_chats.entry(to.clone()).or_insert_with(Vec::new);
                        messages.push(local_msg);
                        
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
                                    Message::NoOp  // WebSocket will handle server confirmation
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
            Message::SendGroupMessage { group_id } => {
                if !self.current_message_input.trim().is_empty() {
                    if let Some(token) = &self.session_token {
                        let svc = chat_service.clone();
                        let token_clone = token.clone();
                        let group_id_clone = group_id.clone();
                        let message = self.current_message_input.trim().to_string();
                        let cfg = crate::server::config::ClientConfig::from_env();
                        let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                        
                        // Create a local message to add immediately to the UI
                        let local_msg = ChatMessage {
                            sender: self.username.clone(),
                            content: message.clone(),
                            timestamp: chrono::Utc::now().timestamp(),
                            formatted_time: chrono::Utc::now().format("%H:%M").to_string(),
                            sent_at: chrono::Utc::now().timestamp(),
                            is_pending: true,  // This is a temporary local message
                        };
                        
                        // Add message to local cache immediately for instant UI feedback
                        let messages = self.group_chats.entry(group_id.clone()).or_insert_with(Vec::new);
                        messages.push(local_msg);
                        
                        // Clear input immediately for better UX
                        // If we don't have the chat history cached yet, mark it as loading
                        if !self.group_chats.contains_key(&group_id) {
                            self.loading_group_chats.insert(group_id.clone());
                        }

                        self.current_message_input.clear();
                        
                        return Command::batch([
                            Command::perform(
                                async move {
                                    let mut guard = svc.lock().await;
                                    let _ = guard.send_group_message(&host, &token_clone, &group_id_clone, &message).await;
                                    Message::NoOp  // WebSocket will handle server confirmation
                                },
                                |msg| msg,
                            ),
                            // Auto-scroll to bottom after sending
                            scrollable::snap_to(
                                scrollable::Id::new("group_messages_scroll"),
                                scrollable::RelativeOffset::END
                            )
                        ]);
                    }
                }
            }
            Message::LoadGroupMessages { group_id } => {
                if let Some(token) = &self.session_token {
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    let group_id_clone = group_id.clone();
                    let cfg = crate::server::config::ClientConfig::from_env();
                    let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                    
                    return Command::perform(
                        async move {
                            let mut guard = svc.lock().await;
                            match guard.get_group_messages(&host, &token_clone, &group_id_clone).await {
                                Ok(messages) => Message::GroupMessagesLoaded { group_id: group_id_clone, messages },
                                Err(e) => {
                                    if e.to_string().contains("NOT_A_MEMBER") {
                                        Message::NotAMember { group_id: group_id_clone }
                                    } else {
                                        Message::GroupMessagesLoaded { group_id: group_id_clone, messages: vec![] }
                                    }
                                }
                            }
                        },
                        |msg| msg,
                    );
                }
            }
            Message::GroupMessagesLoaded { group_id, messages } => {
                self.group_chats.insert(group_id.clone(), messages);
                self.loading_group_chats.remove(&group_id);
                
                // Auto-scroll to bottom when messages are loaded
                if let AppState::GroupChat(current_group_id, _) = &self.app_state {
                    if current_group_id == &group_id {
                        return scrollable::snap_to(
                            scrollable::Id::new("group_messages_scroll"),
                            scrollable::RelativeOffset::END
                        );
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
                // Track the latest timestamp from HTTP loaded messages
                if let Some(latest_msg) = messages.iter().max_by_key(|msg| msg.timestamp) {
                    self.last_http_timestamp.insert(with.clone(), latest_msg.timestamp);
                    println!("[APP] 📚 HTTP loaded {} messages for {}, latest timestamp: {}", 
                        messages.len(), with, latest_msg.timestamp);
                } else {
                    println!("[APP] 📚 HTTP loaded 0 messages for {}", with);
                }
                
                self.private_chats.insert(with.clone(), messages);
                self.loading_private_chats.remove(&with);
                
                // Auto-scroll to bottom when messages are loaded (for recipient)
                if let AppState::PrivateChat(current_chat) = &self.app_state {
                    if current_chat == &with {
                        return scrollable::snap_to(
                            scrollable::Id::new("messages_scroll"),
                            scrollable::RelativeOffset::END
                        );
                    }
                }
            }
             Message::LeaveGroup { group_id: _, group_name } => {
                let cfg = crate::server::config::ClientConfig::from_env();
                let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                let token = self.session_token.clone().unwrap_or_default();
                let svc = chat_service.clone();
                let group_name_clone = group_name.clone();
                
                return Command::perform(
                    async move {
                        let mut guard = svc.lock().await;
                        let cmd = format!("/leave_group {} {}", token, group_name_clone);
                        match guard.send_command(&host, cmd).await {
                            Ok(response) => {
                                if response.starts_with("OK:") {
                                    Message::LeaveGroupResult { success: true, message: format!("Left group '{}'", group_name_clone) }
                                } else {
                                    Message::LeaveGroupResult { success: false, message: response }
                                }
                            }
                            Err(e) => Message::LeaveGroupResult { success: false, message: format!("Error: {}", e) }
                        }
                    },
                    |msg| msg,
                );
            }
            Message::LeaveGroupResult { success, message } => {
                use crate::client::gui::views::logger::{LogMessage, LogLevel};
                if success {
                    self.logger.push(LogMessage {
                        level: LogLevel::Success,
                        message: message.clone(),
                    });
                    
                    // CRITICAL: Stop all polling immediately when leaving group
                    self.polling_active = false;
                    self.group_polling_active = false;
                    
                    // Clear group chat data for security
                    self.group_chats.clear();
                    
                    // Return to My Groups view
                    self.app_state = AppState::MyGroups;
                    
                    // Reload groups list to reflect the change and auto-clear logger
                    let svc = chat_service.clone();
                    let token = self.session_token.clone().unwrap_or_default();
                    let cfg = crate::server::config::ClientConfig::from_env();
                    let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                    return Command::batch([
                        Command::perform(
                            async move {
                                let mut guard = svc.lock().await;
                                let cmd = format!("/my_groups {}", token);
                                match guard.send_command(&host, cmd).await {
                                    Ok(response) => {
                                        // Parse response: "OK: My groups: id1:name1, id2:name2"
                                        if response.starts_with("OK: My groups:") {
                                            let after = response.splitn(3, ':').nth(2).unwrap_or("");
                                            let groups: Vec<(String, String, usize)> = after
                                                .split(',')
                                                .map(|s| s.trim())
                                                .filter(|s| !s.is_empty())
                                                .map(|s| {
                                                    if let Some((id, name)) = s.split_once(':') {
                                                        (id.to_string(), name.to_string(), 0) // member_count not used
                                                    } else {
                                                        (s.to_string(), s.to_string(), 0)
                                                    }
                                                })
                                                .collect();
                                            Message::MyGroupsLoaded { groups }
                                        } else {
                                            Message::MyGroupsLoaded { groups: vec![] }
                                        }
                                    },
                                    Err(_) => Message::MyGroupsLoaded { groups: vec![] },
                                }
                            },
                            |msg| msg
                        ),
                        Command::perform(
                            async move {
                                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                                Message::ClearLog
                            },
                            |msg| msg,
                        )
                    ]);
                } else {
                    self.logger.push(LogMessage {
                        level: LogLevel::Error,
                        message: message.clone(),
                    });
                    
                    // Auto-clear error message after 2 seconds
                    return Command::perform(
                        async move {
                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                            Message::ClearLog
                        },
                        |msg| msg,
                    );
                }
            }
            Message::NotAMember { group_id } => {
                use crate::client::gui::views::logger::{LogMessage, LogLevel};
                self.logger.push(LogMessage {
                    level: LogLevel::Error,
                    message: format!("You are no longer a member of group: {}", group_id),
                });
                
                // CRITICAL: Stop all polling immediately
                self.polling_active = false;
                self.group_polling_active = false;
                
                // Clear group data
                self.group_chats.remove(&group_id);
                if let AppState::GroupChat(current_group_id, _) = &self.app_state {
                    if current_group_id == &group_id {
                        // Navigate back to MyGroups if we're in this group
                        self.app_state = AppState::MyGroups;
                        
                        // Reload groups list
                        let svc = chat_service.clone();
                        let token = self.session_token.clone().unwrap_or_default();
                        let cfg = crate::server::config::ClientConfig::from_env();
                        let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                        return Command::perform(
                            async move {
                                let mut guard = svc.lock().await;
                                let cmd = format!("/my_groups {}", token);
                                match guard.send_command(&host, cmd).await {
                                    Ok(response) => {
                                        if response.starts_with("OK: My groups:") {
                                            let after = response.splitn(3, ':').nth(2).unwrap_or("");
                                            let groups: Vec<(String, String, usize)> = after
                                                .split(',')
                                                .map(|s| s.trim())
                                                .filter(|s| !s.is_empty())
                                                .map(|s| {
                                                    if let Some((id, name)) = s.split_once(':') {
                                                        (id.to_string(), name.to_string(), 0)
                                                    } else {
                                                        (s.to_string(), s.to_string(), 0)
                                                    }
                                                })
                                                .collect();
                                            Message::MyGroupsLoaded { groups }
                                        } else {
                                            Message::MyGroupsLoaded { groups: vec![] }
                                        }
                                    },
                                    Err(_) => Message::MyGroupsLoaded { groups: vec![] },
                                }
                            },
                            |msg| msg
                        );
                    }
                }
            }
            Message::DiscardPrivateMessages { with } => {
                if let Some(token) = &self.session_token {
                    let cfg = crate::server::config::ClientConfig::from_env();
                    let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    let with_clone = with.clone();
                    return Command::perform(
                        async move {
                            let mut guard = svc.lock().await;
                            match guard.send_command(&host, format!("/delete_private_messages {} {}", token_clone, with_clone)).await {
                                Ok(response) => {
                                    if response.starts_with("OK:") {
                                        Message::DiscardMessagesResult { 
                                            success: true, 
                                            message: response.trim_start_matches("OK:").trim().to_string(),
                                            username: Some(with_clone),
                                            group_id: None
                                        }
                                    } else {
                                        Message::DiscardMessagesResult { 
                                            success: false, 
                                            message: response,
                                            username: Some(with_clone),
                                            group_id: None
                                        }
                                    }
                                }
                                Err(e) => Message::DiscardMessagesResult { 
                                    success: false, 
                                    message: format!("Error: {}", e),
                                    username: Some(with_clone),
                                    group_id: None
                                }
                            }
                        },
                        |msg| msg,
                    );
                }
                return Command::none();
            }
            Message::DiscardGroupMessages { group_id } => {
                if let Some(token) = &self.session_token {
                    let cfg = crate::server::config::ClientConfig::from_env();
                    let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                    let svc = chat_service.clone();
                    let token_clone = token.clone();
                    let group_id_clone = group_id.clone();
                    return Command::perform(
                        async move {
                            let mut guard = svc.lock().await;
                            match guard.send_command(&host, format!("/delete_group_messages {} {}", token_clone, group_id_clone)).await {
                                Ok(response) => {
                                    if response.starts_with("OK:") {
                                        Message::DiscardMessagesResult { 
                                            success: true, 
                                            message: response.trim_start_matches("OK:").trim().to_string(),
                                            username: None,  // For group messages, username is None
                                            group_id: Some(group_id_clone)
                                        }
                                    } else {
                                        Message::DiscardMessagesResult { 
                                            success: false, 
                                            message: response,
                                            username: None,
                                            group_id: Some(group_id_clone)
                                        }
                                    }
                                }
                                Err(e) => Message::DiscardMessagesResult { 
                                    success: false, 
                                    message: format!("Error: {}", e),
                                    username: None,
                                    group_id: Some(group_id_clone)
                                }
                            }
                        },
                        |msg| msg,
                    );
                }
                return Command::none();
            }
            Message::DiscardMessagesResult { success, message, username, group_id } => {
                use crate::client::gui::views::logger::{LogMessage, LogLevel};
                if success {
                    self.logger.push(LogMessage {
                        level: LogLevel::Success,
                        message: message.clone(),
                    });
                    
                    // Clear messages from local cache 
                    if let Some(target_user) = username {
                        // Private messages
                        println!("[DISCARD] Clearing local cache for user: {}", target_user);
                        self.private_chats.insert(target_user, Vec::new());
                    } else if let Some(target_group_id) = group_id {
                        // Group messages
                        println!("[DISCARD] Clearing local cache for group: {}", target_group_id);
                        self.group_chats.insert(target_group_id, Vec::new());
                    }
                } else {
                    self.logger.push(LogMessage {
                        level: LogLevel::Error,
                        message: message.clone(),
                    });
                }
                
                // Auto-clear logger after 2 seconds (consistent with other operations)
                return Command::perform(
                    async move {
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        Message::ClearLog
                    },
                    |msg| msg,
                );
            }
            Message::NewMessagesReceived { with, messages } => {
                self.loading_private_chats.remove(&with);
                self.private_chats.insert(with, messages);
                return Command::none();
            }
            Message::NewGroupMessagesReceived { group_id, messages: _ } => {
                self.loading_group_chats.remove(&group_id);
                return Command::none();
            }
            Message::StopMessagePolling => {
                self.polling_active = false;
                self.app_state = AppState::MainActions;
                return Command::<Message>::none();
            }
            Message::StopGroupMessagePolling => {
                self.group_polling_active = false;
                self.app_state = AppState::MainActions;
                return Command::<Message>::none();
            }
            Message::WebSocketConnected => {
                self.logger.push(LogMessage {
                    level: LogLevel::Success,
                    message: "WebSocket connected - Real-time messaging enabled".to_string(),
                });
                
                // Start WebSocket message polling only if not already active
                if !self.websocket_polling_active {
                    self.websocket_polling_active = true;
                    println!("[APP] 🚀 Starting WebSocket message polling loop");
                    return Command::perform(
                        async move {
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                            Message::CheckWebSocketMessages
                        },
                        |msg| msg,
                    );
                } else {
                    println!("[APP] 🔄 WebSocket polling already active, skipping restart");
                }
                return Command::none();
            }
            Message::WebSocketError { error } => {
                self.logger.push(LogMessage {
                    level: LogLevel::Error,
                    message: format!("WebSocket error: {}", error),
                });
                // Stop polling on WebSocket error
                self.websocket_polling_active = false;
                return Command::none();
            }
            Message::StartMessagePolling { with } => {
                // With WebSocket connected, we ONLY load messages once initially
                // No polling needed - WebSocket will deliver new messages in real-time
                if !self.private_chats.contains_key(&with) {
                    self.loading_private_chats.insert(with.clone());
                    println!("[APP] Loading initial messages for {} (WebSocket mode - no polling)", with);
                    return Command::perform(
                        async move { Message::LoadPrivateMessages { with } },
                        |msg| msg,
                    );
                } else {
                    println!("[APP] Messages already cached for {} - no need to reload", with);
                }
                return Command::none();
            }
            Message::WebSocketMessageReceived(ws_msg) => {
                match ws_msg {
                    crate::client::services::websocket_client::WebSocketMessage::NewMessage(chat_msg) => {
                        println!("[APP] Received WebSocket message from {}: {}", chat_msg.from_user, chat_msg.content);
                        
                        // Convert IncomingChatMessage to ChatMessage
                        let app_msg = ChatMessage {
                            sender: chat_msg.from_user.clone(),
                            content: chat_msg.content.clone(),
                            timestamp: chat_msg.timestamp,
                            formatted_time: chrono::DateTime::from_timestamp(chat_msg.timestamp, 0)
                                .map(|dt| dt.format("%H:%M").to_string())
                                .unwrap_or_else(|| "??:??".to_string()),
                            sent_at: chat_msg.timestamp,
                            is_pending: false,  // This is a confirmed server message
                        };
                        
                        // Determine the chat key (who we're chatting with)
                        let chat_key = if chat_msg.chat_type == "private" {
                            if let Some(to_user) = &chat_msg.to_user {
                                if to_user == &self.username {
                                    // Message sent TO us, chat key is the sender
                                    chat_msg.from_user.clone()
                                } else {
                                    // Message sent BY us, chat key is the recipient
                                    to_user.clone()
                                }
                            } else {
                                chat_msg.from_user.clone()
                            }
                        } else if chat_msg.chat_type == "group" {
                            // Group message - use group_id as chat key
                            if let Some(group_id) = &chat_msg.group_id {
                                format!("group_{}", group_id)
                            } else {
                                println!("[APP] ERROR: Group message without group_id");
                                return Command::none();
                            }
                        } else {
                            println!("[APP] ERROR: Unknown chat_type: {}", chat_msg.chat_type);
                            return Command::none();
                        };
                        
                        // Add message to the appropriate chat (with deduplication)
                        if chat_msg.chat_type == "private" {
                            let messages = self.private_chats.entry(chat_key.clone())
                                .or_insert_with(Vec::new);
                            
                            // Check if this WebSocket message is newer than the latest HTTP-loaded message
                            let last_http_ts = self.last_http_timestamp.get(&chat_key).copied().unwrap_or(0);
                            
                            if app_msg.timestamp <= last_http_ts {
                                println!("[APP] 🚫 Skipping WebSocket message (timestamp {} <= last HTTP timestamp {} for {})", 
                                    app_msg.timestamp, last_http_ts, chat_key);
                            } else {
                                // Try to replace a pending message with same content from the same sender
                                let mut replaced_pending = false;
                                for existing_msg in messages.iter_mut() {
                                    if existing_msg.is_pending && 
                                       existing_msg.sender == app_msg.sender &&
                                       existing_msg.content == app_msg.content {
                                        // Replace the pending message with the server-confirmed one
                                        *existing_msg = app_msg.clone();
                                        existing_msg.is_pending = false;
                                        replaced_pending = true;
                                        println!("[APP] 🔄 Replaced pending message with server confirmation for {} (timestamp: {})", 
                                            chat_key, app_msg.timestamp);
                                        break;
                                    }
                                }
                                
                                if !replaced_pending {
                                    // Check for exact duplicates (same timestamp, sender, content - real network duplicates)
                                    let is_exact_duplicate = messages.iter().any(|existing_msg| {
                                        existing_msg.sender == app_msg.sender &&
                                        existing_msg.content == app_msg.content &&
                                        existing_msg.timestamp == app_msg.timestamp &&
                                        !existing_msg.is_pending  // Only check confirmed messages for exact duplicates
                                    });
                                    
                                    if !is_exact_duplicate {
                                        let msg_timestamp = app_msg.timestamp; // Save timestamp before move
                                        messages.push(app_msg);
                                        println!("[APP] ✅ Added WebSocket private message to chat with {} (timestamp: {})", 
                                            chat_key, msg_timestamp);
                                    } else {
                                        println!("[APP] ⚠️ Exact duplicate WebSocket message for {} (sender: {}, content: {}, timestamp: {})", 
                                            chat_key, app_msg.sender, app_msg.content, app_msg.timestamp);
                                    }
                                }
                            }
                        } else if chat_msg.chat_type == "group" {
                            // Extract just the group_id from "group_groupid" format
                            let group_id = chat_key.strip_prefix("group_").unwrap_or(&chat_key);
                            let messages = self.group_chats.entry(group_id.to_string())
                                .or_insert_with(Vec::new);
                            
                            // Check if there's a pending message to replace first
                            let replaced_pending = messages.iter_mut().find(|msg| {
                                msg.is_pending && msg.sender == app_msg.sender && msg.content == app_msg.content
                            });
                            
                            if let Some(pending_msg) = replaced_pending {
                                // Replace pending message with server confirmation
                                *pending_msg = app_msg;
                                println!("[APP] 🔄 Replaced pending group message with server confirmation for group {} (timestamp: {})", group_id, chat_msg.timestamp);
                            } else {
                                // Check only for exact timestamp duplicates (network-level duplicates)
                                let is_exact_duplicate = messages.iter().any(|existing_msg| {
                                    existing_msg.sender == app_msg.sender &&
                                    existing_msg.content == app_msg.content &&
                                    existing_msg.timestamp == app_msg.timestamp  // Only exact timestamp matches are duplicates
                                });
                                
                                if !is_exact_duplicate {
                                    messages.push(app_msg);
                                    println!("[APP] ✅ Added WebSocket group message to group {} (timestamp: {})", group_id, chat_msg.timestamp);
                                } else {
                                    println!("[APP] ⚠️ Exact duplicate WebSocket group message for group {} (sender: {}, content: {}, timestamp: {})", 
                                        group_id, chat_msg.from_user, chat_msg.content, chat_msg.timestamp);
                                }
                            }
                        }
                        
                        // If we're currently viewing this chat, auto-scroll to bottom to trigger UI update
                        if chat_msg.chat_type == "private" {
                            if let AppState::PrivateChat(current_chat) = &self.app_state {
                                if current_chat == &chat_key {
                                    // We're currently viewing this private chat - scroll to bottom
                                    return scrollable::snap_to(
                                        scrollable::Id::new("messages_scroll"),
                                        scrollable::RelativeOffset::END
                                    );
                                }
                            }
                        } else if chat_msg.chat_type == "group" {
                            if let AppState::GroupChat(current_group_id, _) = &self.app_state {
                                let group_id = chat_key.strip_prefix("group_").unwrap_or(&chat_key);
                                if current_group_id == group_id {
                                    // We're currently viewing this group chat - scroll to bottom
                                    return scrollable::snap_to(
                                        scrollable::Id::new("messages_scroll"),
                                        scrollable::RelativeOffset::END
                                    );
                                }
                            }
                        }
                        
                        // Not viewing this chat currently, just add the message silently
                        return Command::none();
                    }
                    crate::client::services::websocket_client::WebSocketMessage::UserStatusUpdate { user_id, online } => {
                        println!("[APP] User {} is now {}", user_id, if online { "online" } else { "offline" });
                    }
                    crate::client::services::websocket_client::WebSocketMessage::Error(error) => {
                        println!("[APP] WebSocket error: {}", error);
                        self.logger.push(LogMessage {
                            level: LogLevel::Error,
                            message: format!("WebSocket error: {}", error),
                        });
                    }
                }
                return Command::none();
            }
            Message::CheckWebSocketMessages => {
                // Only continue if polling is still active
                if !self.websocket_polling_active {
                    println!("[APP] 🛑 WebSocket polling stopped, ending loop");
                    return Command::none();
                }
                
                // Check for new WebSocket messages
                let svc = chat_service.clone();
                
                return Command::perform(
                    async move {
                        let mut guard = svc.lock().await;
                        
                        // Check if WebSocket is connected
                        if guard.websocket_receiver.is_none() {
                            // WebSocket not connected, wait and retry
                            println!("[APP] 🔄 WebSocket receiver not available, retrying in 1 second...");
                            drop(guard);
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            return Message::CheckWebSocketMessages;
                        }
                        
                        if let Some(ws_message) = guard.try_receive_websocket_message().await {
                            drop(guard);
                            return Message::WebSocketMessageReceived(ws_message);
                        }
                        
                        drop(guard);
                        // Continue polling after a short delay
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        Message::CheckWebSocketMessages
                    },
                    |msg| msg,
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