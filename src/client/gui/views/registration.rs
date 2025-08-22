use iced::{Element, Length, Alignment, Color, Background};
use iced::widget::{Column, Row, Text, TextInput, Button, PickList, Container, Space};
use iced::widget::{button, container};
use crate::client::models::messages::Message;
use crate::client::models::app_state::ChatAppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostType {
    Localhost,
    Remote,
    Manual,
}

impl ToString for HostType {
    fn to_string(&self) -> String {
        match self {
            HostType::Localhost => "localhost".to_string(),
            HostType::Remote => "remote host".to_string(),
            HostType::Manual => "manual".to_string(),
        }
    }
}


const ALL_HOSTS: [HostType; 3] = [HostType::Localhost, HostType::Remote, HostType::Manual];

impl HostType {
    pub fn all() -> &'static [HostType] {
        &ALL_HOSTS
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            HostType::Localhost => "localhost",
            HostType::Remote => "remote host",
            HostType::Manual => "manual",
        }
    }
}

impl Default for HostType {
    fn default() -> Self {
        HostType::Localhost
    }
}

pub fn view(state: &ChatAppState) -> Element<Message> {
    let username = &state.username;
    let password = &state.password;
    let selected_host = state.selected_host.clone(); // Assicurati che ChatAppState usi HostType da registration.rs
    let manual_host = &state.manual_host;
    let is_login = state.is_login;
    let error_message = state.error_message.clone();
    let loading = state.loading;
    let log_messages = &state.log_messages;

    // Validazione username
    let username_valid = !username.is_empty() && username.len() >= 3 && username.chars().all(|c| c.is_alphanumeric());
    let username_hint = if username.is_empty() {
        "Inserisci uno username"
    } else if username.len() < 3 {
        "Minimo 3 caratteri"
    } else if !username.chars().all(|c| c.is_alphanumeric()) {
        "Solo caratteri alfanumerici"
    } else {
        "Username valido"
    };
    let username_hint_color = if username_valid { Color::from_rgb(0.0, 0.7, 0.0) } else { Color::from_rgb(1.0, 0.5, 0.0) };

    // Validazione password
    let password_valid = !password.is_empty() && password.len() >= 6;
    let password_hint = if password.is_empty() {
        "Inserisci una password"
    } else if password.len() < 6 {
        "Minimo 6 caratteri"
    } else {
        "Password valida"
    };
    let password_hint_color = if password_valid { Color::from_rgb(0.0, 0.7, 0.0) } else { Color::from_rgb(1.0, 0.5, 0.0) };

    // Pulsante submit abilitato solo se validi
    let submit_enabled = username_valid && password_valid && !loading;

    let host_picklist = PickList::new(
        &HostType::all()[..],
        Some(selected_host.clone()),
        Message::HostSelected,
    );
    let manual_host_input: Element<Message> = if selected_host == HostType::Manual {
        Container::new(
            TextInput::new("Host...", manual_host)
                .on_input(Message::ManualHostChanged)
        ).into()
    } else {
        Container::new(Space::new(Length::Fill, Length::Fixed(0.0))).into()
    };

    let username_input = TextInput::new("Username", username)
        .on_input(Message::UsernameChanged);
    let username_hint_text = Text::new(username_hint).style(username_hint_color);
    let password_input = TextInput::new("Password", password)
        .on_input(Message::PasswordChanged);
    let password_hint_text = Text::new(password_hint).style(password_hint_color);

    let submit_button = if submit_enabled {
        Button::new(Text::new(if is_login { "Login" } else { "Registrati" }))
            .on_press(Message::SubmitLoginOrRegister)
    } else {
        Button::new(Text::new(if is_login { "Login" } else { "Registrati" }))
    };

    let error_text = if let Some(msg) = error_message {
        Text::new(msg).style(Color::from_rgb(1.0, 0.0, 0.0))
    } else {
        Text::new("")
    };
    let loading_text = if loading {
        Text::new("Caricamento...").style(Color::from_rgb(0.5, 0.5, 1.0))
    } else {
        Text::new("")
    };

    let log_area = Column::with_children(
        log_messages.iter().map(|(msg, color)| {
            Text::new(msg).style(iced::theme::Text::Color(*color)).into()
        }).collect::<Vec<_>>()
    ).spacing(2);

    // Build a centered auth card for a more professional initial screen
    // --- styles -------------------------------------------------
    struct CardStyle;
    impl container::StyleSheet for CardStyle {
        type Style = iced::Theme;
        fn appearance(&self, _style: &Self::Style) -> container::Appearance {
            container::Appearance {
                background: Some(Background::Color(Color { r: 0.09, g: 0.15, b: 0.18, a: 1.0 })),
                ..Default::default()
            }
        }
    }

    struct PrimaryButtonStyle;
    impl button::StyleSheet for PrimaryButtonStyle {
        type Style = iced::Theme;
        fn active(&self, _style: &Self::Style) -> button::Appearance {
            button::Appearance {
                background: Some(Background::Color(Color { r: 0.0, g: 0.68, b: 0.9, a: 1.0 })),
                text_color: Color::from_rgb(0.0, 0.0, 0.0),
                ..Default::default()
            }
        }
        fn hovered(&self, style: &Self::Style) -> button::Appearance {
            let mut s = self.active(style);
            s.background = Some(Background::Color(Color { r: 0.0, g: 0.75, b: 0.95, a: 1.0 }));
            s
        }
    }

    struct TabButtonStyle;
    impl button::StyleSheet for TabButtonStyle {
        type Style = iced::Theme;
        fn active(&self, _style: &Self::Style) -> button::Appearance {
            button::Appearance {
                background: Some(Background::Color(Color { r: 0.06, g: 0.12, b: 0.14, a: 1.0 })),
                text_color: Color::from_rgb(0.85, 0.95, 0.98),
                ..Default::default()
            }
        }
        fn hovered(&self, style: &Self::Style) -> button::Appearance { self.active(style) }
    }
    let tabs = Row::new()
        .spacing(10)
        .push(Button::new(Text::new("Login")).on_press(Message::ToggleLoginRegister).style(TabButtonStyle))
        .push(Button::new(Text::new("Register")).on_press(Message::ToggleLoginRegister).style(TabButtonStyle));

    let card = Column::new()
        .width(Length::Fixed(420.0))
        .spacing(12)
        .padding(16)
        .align_items(Alignment::Center)
        .push(Text::new("Ruggine").size(36).style(iced::theme::Text::Color(Color::from_rgb(0.95,0.98,1.0))))
        .push(Row::new().push(Space::new(Length::Fill, Length::Fixed(0.0))).push(host_picklist).push(manual_host_input))
        .push(tabs)
        .push(username_input.width(Length::Fill))
        .push(username_hint_text)
        .push(password_input.width(Length::Fill))
        .push(password_hint_text)
        .push(
            if submit_enabled {
                submit_button.style(PrimaryButtonStyle)
            } else {
                submit_button
            }
        )
        .push(error_text)
        .push(loading_text)
        .push(log_area);
    Container::new(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .style(CardStyle)
        .into()
}

