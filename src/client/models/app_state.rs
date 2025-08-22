
#[derive(Debug, Clone)]
pub enum AppState {
    Registration,
    MainActions,
    Chat,
    PrivateChat(String),
    GroupChat(String, String),
    FriendRequests,
}

impl Default for AppState {
    fn default() -> Self {
        AppState::Registration
    }
}


use crate::client::gui::views::registration::HostType;

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
    pub log_messages: Vec<(String, iced::Color)>,
    pub welcome_message: Option<String>,
    pub session_token: Option<String>,
}

impl ChatAppState {
    pub fn update(&mut self, message: crate::client::models::messages::Message, chat_service: &mut crate::client::services::chat_service::ChatService) -> iced::Command<crate::client::models::messages::Message> {
        use crate::client::models::messages::Message as Msg;
        match message {
            Msg::UsernameChanged(val) => self.username = val,
            Msg::PasswordChanged(val) => self.password = val,
            Msg::HostSelected(host) => self.selected_host = host,
            Msg::ManualHostChanged(val) => self.manual_host = val,
            Msg::ToggleLoginRegister => self.is_login = !self.is_login,
            Msg::SubmitLoginOrRegister => {
                self.loading = true;
                self.error_message = None;
                // Qui va la logica di invio comando al server tramite chat_service
                // Esempio: chat_service.login_or_register(...)
            }
            Msg::AuthResult { success, message, token } => {
                self.loading = false;
                if success {
                    self.session_token = token;
                    self.welcome_message = Some(message.clone());
                    self.app_state = AppState::MainActions;
                    self.log_messages.push((format!("[SUCCESS] {}", message), iced::Color::from_rgb(0.0, 0.7, 0.0)));
                } else {
                    self.error_message = Some(message.clone());
                    self.log_messages.push((format!("[ERROR] {}", message), iced::Color::from_rgb(1.0, 0.0, 0.0)));
                }
            }
            Msg::Logout => {
                self.session_token = None;
                self.app_state = AppState::Registration;
                self.welcome_message = None;
                self.log_messages.push(("Logout effettuato".to_string(), iced::Color::from_rgb(0.0, 0.0, 1.0)));
            }
            Msg::LogInfo(msg) => self.log_messages.push((msg, iced::Color::from_rgb(0.0, 0.0, 1.0))),
            Msg::LogSuccess(msg) => self.log_messages.push((msg, iced::Color::from_rgb(0.0, 0.7, 0.0))),
            Msg::LogError(msg) => self.log_messages.push((msg, iced::Color::from_rgb(1.0, 0.0, 0.0))),
            _ => {}
        }
        iced::Command::none()
    }
}
