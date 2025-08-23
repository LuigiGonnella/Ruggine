
#[derive(Debug, Clone)]
pub enum AppState {
    CheckingSession,
    Registration,
    MainActions,
    Chat,
    PrivateChat(String),
    GroupChat(String, String),
    FriendRequests,
}

impl Default for AppState {
    fn default() -> Self {
    AppState::CheckingSession
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
                // Qui va la logica di invio comando al server tramite chat_service
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
                // Clear persisted session and reset state
                let _ = session_store::clear_session_token();
                self.session_token = None;
                self.app_state = AppState::Registration;
                self.welcome_message = None;
                self.logger.clear();
                self.logger.push(crate::client::gui::views::logger::LogMessage {
                    level: crate::client::gui::views::logger::LogLevel::Info,
                    message: "Logout effettuato con successo.".to_string(),
                });
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
            _ => {}
        }
        iced::Command::none()
    }
}
