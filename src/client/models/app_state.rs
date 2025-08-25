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
    CreateGroup,
    MyGroups,
    InviteToGroup { group_id: String, group_name: String },
    MyGroupInvites,
    SendFriendRequest,
    ViewFriends,
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

impl ChatAppState {
    pub fn update(&mut self, message: Message, chat_service: &Arc<Mutex<ChatService>>) -> Command<Message> {
        use crate::client::gui::views::logger::{LogMessage, LogLevel};
        use crate::client::utils::session_store;
        use crate::client::services::users_service::UsersService;
        
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
                
                // Show logout message temporarily
                self.logger.clear();
                self.logger.push(LogMessage {
                    level: LogLevel::Info,
                    message: "Logout successful".to_string(),
                });
                
                // Send logout command if we have a token
                if let Some(token) = &self.session_token {
                    let svc = chat_service.clone();
                let cfg = crate::server::config::ClientConfig::from_env();
                let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                    let cfg = crate::server::config::ClientConfig::from_env();
                    let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                    let token_clone = token.clone();
                    
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
                // Mark this private chat as loading so the UI shows a loader
                self.loading_private_chats.insert(username.clone());

                // Start message polling for real-time updates
                return Command::perform(
                    async move { Message::StartMessagePolling { with: username } },
                    |msg| msg,
                );
            }
            Message::OpenGroupChat(group_id, group_name) => {
                self.app_state = AppState::GroupChat(group_id.clone(), group_name.clone());
                self.current_message_input.clear();
                // Mark this group chat as loading so the UI shows a loader
                self.loading_group_chats.insert(group_id.clone());

                // Start message polling for real-time updates
                return Command::perform(
                    async move { Message::StartGroupMessagePolling { group_id } },
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
                
                return Command::perform(
                    async move {
                        // Get all users and group members to filter
                        let all_users = UsersService::list_all(&svc, &host).await.unwrap_or_default();
                        
                        // Get group members to filter them out
                        let mut guard = svc.lock().await;
                        let group_members_resp = guard.send_command(&host, format!("/group_members {} {}", token, group_id_for_filter)).await.unwrap_or_default();
                        drop(guard);
                        
                        // Parse group members (assuming format "OK: Members: user1, user2")
                        let existing_members: Vec<String> = if group_members_resp.starts_with("OK: Members:") {
                            group_members_resp.trim_start_matches("OK: Members:").trim()
                                .split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
                        } else {
                            vec![]
                        };
                        
                        // Filter out existing members
                        let filtered_users: Vec<String> = all_users.into_iter()
                            .filter(|user| !existing_members.contains(user))
                            .collect();
                        
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
                    let token_clone = token.clone();
                    let cfg = crate::server::config::ClientConfig::from_env();
                    let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                    
                    return Command::perform(
                        async move {
                            let mut guard = svc.lock().await;
                            match guard.send_command(&host, format!("/accept_friend_request {} {}", token_clone, username)).await {
                                Ok(response) => {
                                    if response.starts_with("OK:") {
                                        Message::FriendRequestResult { 
                                            success: true, 
                                            message: format!("Friend request from {} accepted!", username) 
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
                                    message: format!("Error accepting friend request: {}", e) 
                                },
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
            Message::FriendsLoaded { friends } => {
                self.loading = false;
                self.friends_list = friends;
            }
            Message::FriendRequestsLoaded { requests } => {
                self.loading = false;
                self.friend_requests = requests;
            }
            Message::FriendRequestResult { success, message } => {
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
                                        if response.starts_with("OK:") {
                                            // Extract group ID from response if available, otherwise use name as ID
                                            let group_id = uuid::Uuid::new_v4().to_string(); // Temporary ID
                                            Message::GroupCreated { group_id, group_name: name_clone }
                                        } else {
                                            Message::LogError(response)
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
                    message: format!("Gruppo '{}' creato con successo!", group_name),
                });
                
                // Navigate to the newly created group
                return Command::perform(
                    async move { Message::OpenGroupChat(group_id, group_name) },
                    |msg| msg,
                );
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
                                            message: format!("Invito inviato a {} con successo!", username_clone) 
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
                                    message: format!("Errore nell'invio dell'invito: {}", e) 
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
                                            message: "Invito accettato! Ora fai parte del gruppo.".to_string() 
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
                                    message: format!("Errore nell'accettazione dell'invito: {}", e) 
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
                                            message: "Invito rifiutato.".to_string() 
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
                                    message: format!("Errore nel rifiuto dell'invito: {}", e) 
                                },
                            }
                        },
                        |msg| msg,
                    );
                }
            }
            Message::GroupInviteActionResult { success, message } => {
                self.logger.push(LogMessage {
                    level: if success { LogLevel::Success } else { LogLevel::Error },
                    message,
                });
                
                // Refresh invites list after action
                if success {
                    return Command::perform(
                        async move { Message::OpenMyGroupInvites },
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
            Message::SendGroupMessage { group_id } => {
                if !self.current_message_input.trim().is_empty() {
                    if let Some(token) = &self.session_token {
                        let svc = chat_service.clone();
                        let token_clone = token.clone();
                        let group_id_clone = group_id.clone();
                        let message = self.current_message_input.trim().to_string();
                        let cfg = crate::server::config::ClientConfig::from_env();
                        let host = format!("{}:{}", cfg.default_host, cfg.default_port);
                        
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
                                    Message::TriggerImmediateGroupRefresh { group_id: group_id_clone }
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
                                Err(_) => Message::GroupMessagesLoaded { group_id: group_id_clone, messages: vec![] },
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
            // Placeholder implementations for other messages
            _ => {
                // Handle other messages as needed
            }
        }
        
        Command::none()
    }
}