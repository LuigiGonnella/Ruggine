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
<<<<<<< HEAD
    StartPrivateChat(String), // username to start chat with
    PrivateMessageChanged(String), // text input changed
    SendPrivateMessage(String), // username to send message to
    PrivateMessagesLoaded { with_user: String, messages: Vec<String> },
    LoadPrivateMessages(String), // username to load messages from
=======
    MessageInputChanged(String),
    SendPrivateMessage { to: String },
    LoadPrivateMessages { with: String },
    PrivateMessagesLoaded { with: String, messages: Vec<crate::client::models::app_state::ChatMessage> },
    // Real-time message updates
    StartMessagePolling { with: String },
    StopMessagePolling,
    NewMessagesReceived { with: String, messages: Vec<crate::client::models::app_state::ChatMessage> },
    TriggerImmediateRefresh { with: String },
>>>>>>> b08dc3b595f658f02b31de5ddc0ef5aa6b30a912
}
