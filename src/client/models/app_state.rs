
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
use crate::client::gui::views::logger::LogMessage;

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
            Msg::ToggleShowPassword => self.show_password = !self.show_password,
            Msg::SubmitLoginOrRegister => {
                self.loading = true;
                self.error_message = None;
                // Qui va la logica di invio comando al server tramite chat_service
            }
            Msg::AuthResult { success, message, token } => {
                self.loading = false;
                if success {
                    self.session_token = token;
                    self.welcome_message = Some(message.clone());
                    self.app_state = AppState::MainActions;
                    let action = if self.is_login { "Login" } else { "Registrazione" };
                    self.logger.push(crate::client::gui::views::logger::LogMessage {
                        level: crate::client::gui::views::logger::LogLevel::Success,
                        message: format!("{} effettuato con successo! Benvenuto, {}.", action, self.username),
                    });
                } else {
                    self.error_message = Some(message.clone());
                    let action = if self.is_login { "Login" } else { "Registrazione" };
                    self.logger.push(crate::client::gui::views::logger::LogMessage {
                        level: crate::client::gui::views::logger::LogLevel::Error,
                        message: format!("{} fallito: {}", action, message),
                    });
                }
            }
            Msg::Logout => {
                self.session_token = None;
                self.app_state = AppState::Registration;
                self.welcome_message = None;
                self.logger.push(crate::client::gui::views::logger::LogMessage {
                    level: crate::client::gui::views::logger::LogLevel::Info,
                    message: "Logout effettuato con successo.".to_string(),
                });
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
            _ => {}
        }
        iced::Command::none()
    }
}
