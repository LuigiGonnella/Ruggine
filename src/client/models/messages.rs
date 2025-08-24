use crate::client::gui::views::registration::HostType;

#[derive(Debug, Clone)]
pub enum Message {
    // Placeholder per tutte le azioni dell'app
    Logout,
    None,
    ManualHostChanged(String),
    UsernameChanged(String),
    PasswordChanged(String),
    HostSelected(HostType),
    ToggleLoginRegister,
    SubmitLoginOrRegister,
    AuthResult { success: bool, message: String, token: Option<String> },
    SessionMissing,
    ClearLog,
    LogInfo(String),
    LogSuccess(String),
    LogError(String),
    ToggleShowPassword,
    // UI navigation and test actions for messaging features
    OpenFriendRequests,
    OpenMainActions,
    OpenPrivateChat(String),
    OpenGroupChat(String, String),
    OpenUsersList { kind: String },
    UsersSearchQueryChanged(String),
    UsersSearch,
    UsersListLoaded { kind: String, list: Vec<String> },
    // Test network actions triggered from main_actions (use defaults in the UI)
    SendGroupMessageTest,
    SendPrivateMessageTest,
    GetGroupMessagesTest,
    GetPrivateMessagesTest,
    DeleteGroupMessagesTest,
    DeletePrivateMessagesTest,
    // Friend system actions
    SendFriendRequest { to: String, message: String },
    AcceptFriendRequest { from: String },
    RejectFriendRequest { from: String },
    ListFriends,
    ReceivedFriendRequests,
    SentFriendRequests,
    // Users and groups actions
    ListOnlineUsers,
    ListAllUsers,
    CreateGroup { name: String },
    MyGroups,
    // Group invite / membership actions
    InviteToGroup { group_id: String, username: String },
    AcceptGroupInvite { invite_id: String },
    RejectGroupInvite { invite_id: String },
    MyGroupInvites,
    JoinGroup { group_id: String },
    LeaveGroup { group_id: String },
    // Private chat messages
    MessageInputChanged(String),
    SendPrivateMessage { to: String },
    LoadPrivateMessages { with: String },
    PrivateMessagesLoaded { with: String, messages: Vec<crate::client::models::app_state::ChatMessage> },
    // Real-time message updates
    StartMessagePolling { with: String },
    StopMessagePolling,
    NewMessagesReceived { with: String, messages: Vec<crate::client::models::app_state::ChatMessage> },
}
