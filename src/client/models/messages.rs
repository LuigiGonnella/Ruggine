use crate::client::gui::views::registration::HostType;

#[derive(Debug, Clone)]
pub enum Message {
    // Placeholder per tutte le azioni dell'app
    NoOp,  // No operation - used when we need to return a message but do nothing
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
    OpenPrivateChat(String),
    OpenGroupChat(String, String),
    OpenUsersList { kind: String },
    UsersSearchQueryChanged(String),
    UsersSearch,
    UsersListLoaded { kind: String, list: Vec<String> },
    UsersListFiltered { list: Vec<String> },
    // Test network actions triggered from main_actions (use defaults in the UI)
    SendGroupMessageTest,
    SendPrivateMessageTest,
    GetGroupMessagesTest,
    GetPrivateMessagesTest,
    DeleteGroupMessagesTest,
    DeletePrivateMessagesTest,
    // Friend system actions
    SendFriendRequest { to: String, message: String },
    ListFriends,
    ReceivedFriendRequests,
    SentFriendRequests,
    // Users and groups actions
    ListOnlineUsers,
    ListAllUsers,
    CreateGroup { name: String },
    MyGroups,
    GroupsListLoaded { groups: Vec<String> },
    // Group invite / membership actions
    InviteToGroup { group_id: String, username: String },
    MyGroupInvites,
    JoinGroup { group_id: String },
    // Private chat messages
    MessageInputChanged(String),
    SendPrivateMessage { to: String },
    LoadPrivateMessages { with: String },
    PrivateMessagesLoaded { with: String, messages: Vec<crate::client::models::app_state::ChatMessage> },
    // Real-time message updates
    StartMessagePolling { with: String },
    StopMessagePolling,
    NewMessagesReceived { with: String, messages: Vec<crate::client::models::app_state::ChatMessage> },
    TriggerImmediateRefresh { with: String },
    // Navigation with polling control
    OpenMainActions,
    // Group chat messages
    SendGroupMessage { group_id: String },
    LoadGroupMessages { group_id: String },
    GroupMessagesLoaded { group_id: String, messages: Vec<crate::client::models::app_state::ChatMessage> },
    // Real-time group message updates
    StartGroupMessagePolling { group_id: String },
    StopGroupMessagePolling,
    NewGroupMessagesReceived { group_id: String, messages: Vec<crate::client::models::app_state::ChatMessage> },
    TriggerImmediateGroupRefresh { group_id: String },
    // Group management
    OpenCreateGroup,
    OpenMyGroups,
    OpenInviteToGroup { group_id: String, group_name: String },
    CreateGroupInputChanged(String),
    CreateGroupSubmit,
    GroupCreated { group_id: String, group_name: String },
    // Participant selection for group creation
    ToggleParticipant(String),
    RemoveParticipant(String),
    MyGroupsLoaded { groups: Vec<(String, String, usize)> }, // (id, name, member_count)
    InviteUserToGroup { group_id: String, username: String },
    // Group invites management
    OpenMyGroupInvites,
    MyGroupInvitesLoaded { invites: Vec<(i64, String, String)> }, // (invite_id, group_name, invited_by)
    AcceptGroupInvite { invite_id: i64 },
    RejectGroupInvite { invite_id: i64 },
    GroupInviteActionResult { success: bool, message: String },
    // Leave group
    LeaveGroup { group_id: String, group_name: String},
    LeaveGroupResult { success: bool, message: String },
    // Error handling for group membership
    NotAMember { group_id: String },
    // Discard messages feature
    DiscardPrivateMessages { with: String },
    DiscardGroupMessages { group_id: String },
    // Friend system
    OpenSendFriendRequest,
    OpenViewFriends,
    SendFriendRequestToUser { username: String, message: String },
    FriendRequestResult { success: bool, message: String },
    // Friend request management
    AcceptFriendRequestFromUser { username: String },
    RejectFriendRequestFromUser { username: String },
    FriendsLoaded { friends: Vec<String> },
    FriendRequestsLoaded { requests: Vec<(String, String)> },
    InviteToGroupResult{success: bool, message: String},
    DiscardMessagesResult { success: bool, message: String, username: Option<String>, group_id: Option<String> },
    // WebSocket connection messages
    WebSocketConnected,
    WebSocketError { error: String },
    // Real-time WebSocket messages
    WebSocketMessageReceived(crate::client::services::websocket_client::WebSocketMessage),
    CheckWebSocketMessages,
    // Logout completion
    LogoutCompleted,
}