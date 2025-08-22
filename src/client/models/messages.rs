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
    LogInfo(String),
    LogSuccess(String),
    LogError(String),
}
