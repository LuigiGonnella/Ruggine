use iced::{Element, Length, Alignment, Color, Background, Border};
use iced::widget::{Column, Row, Text, TextInput, Button, PickList, Container, Space};
use iced::widget::{button, container, text_input};
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
            HostType::Localhost => "Localhost".to_string(),
            HostType::Remote => "Remote".to_string(),
            HostType::Manual => "Manual".to_string(),
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
            HostType::Localhost => "Localhost",
            HostType::Remote => "Remote",
            HostType::Manual => "Manual",
        }
    }
}

impl Default for HostType {
    fn default() -> Self {
        HostType::Localhost
    }
}

// Custom styles
struct DarkContainerStyle;
impl container::StyleSheet for DarkContainerStyle {
    type Style = iced::Theme;
    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.11, 0.15, 0.18))),
            border: Border::default(),
            ..Default::default()
        }
    }
}

struct CardStyle;
impl container::StyleSheet for CardStyle {
    type Style = iced::Theme;
    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.15, 0.20, 0.24))),
            border: Border {
                radius: 12.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

struct InputStyle;
impl text_input::StyleSheet for InputStyle {
    type Style = iced::Theme;
    
    fn active(&self, _style: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: Background::Color(Color::from_rgb(0.20, 0.26, 0.30)),
            border: Border {
                radius: 8.0.into(),
                width: 1.0,
                color: Color::from_rgb(0.25, 0.31, 0.35),
            },
            icon_color: Color::from_rgb(0.6, 0.7, 0.8),
        }
    }
    
    fn focused(&self, style: &Self::Style) -> text_input::Appearance {
        let mut appearance = self.active(style);
        appearance.border.color = Color::from_rgb(0.0, 0.68, 0.9);
        appearance
    }
    
    fn placeholder_color(&self, _style: &Self::Style) -> Color {
        Color::from_rgb(0.5, 0.6, 0.7)
    }
    
    fn value_color(&self, _style: &Self::Style) -> Color {
        Color::from_rgb(0.9, 0.95, 1.0)
    }
    
    fn disabled_color(&self, _style: &Self::Style) -> Color {
        Color::from_rgb(0.4, 0.5, 0.6)
    }
    
    fn selection_color(&self, _style: &Self::Style) -> Color {
        Color::from_rgb(0.0, 0.68, 0.9)
    }
    
    fn disabled(&self, style: &Self::Style) -> text_input::Appearance {
        self.active(style)
    }
    
    fn hovered(&self, style: &Self::Style) -> text_input::Appearance {
        self.active(style)
    }
}

struct PrimaryButtonStyle;
impl button::StyleSheet for PrimaryButtonStyle {
    type Style = iced::Theme;
    
    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.0, 0.68, 0.9))),
            text_color: Color::from_rgb(1.0, 1.0, 1.0),
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
    
    fn hovered(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.0, 0.75, 0.95))),
            text_color: Color::from_rgb(1.0, 1.0, 1.0),
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
    
    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        self.hovered(style)
    }
    
    fn disabled(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.3, 0.4, 0.5))),
            text_color: Color::from_rgb(0.6, 0.7, 0.8),
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

struct TabButtonStyle {
    is_active: bool,
}

impl button::StyleSheet for TabButtonStyle {
    type Style = iced::Theme;
    
    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: None,
            text_color: if self.is_active {
                Color::from_rgb(1.0, 1.0, 1.0)
            } else {
                Color::from_rgb(0.6, 0.7, 0.8)
            },
            border: Border {
                width: if self.is_active { 2.0 } else { 0.0 },
                color: Color::from_rgb(0.0, 0.68, 0.9),
                radius: 0.0.into(),
            },
            ..Default::default()
        }
    }
    
    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let mut appearance = self.active(style);
        appearance.text_color = Color::from_rgb(1.0, 1.0, 1.0);
        appearance
    }
    
    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        self.hovered(style)
    }
    
    fn disabled(&self, style: &Self::Style) -> button::Appearance {
        self.active(style)
    }
}

pub fn view(state: &ChatAppState) -> Element<Message> {
    let username = &state.username;
    let password = &state.password;
    let selected_host = state.selected_host.clone();
    let manual_host = &state.manual_host;
    let is_login = state.is_login;
    let error_message = state.error_message.clone();
    let loading = state.loading;

    // Validazione
    let username_valid = !username.is_empty() && username.len() >= 3 && username.chars().all(|c| c.is_alphanumeric());
    let password_valid = !password.is_empty() && password.len() >= 6;
    let submit_enabled = username_valid && password_valid && !loading;

    // Host selector in top right
    let host_selector = Container::new(
        PickList::new(
            &HostType::all()[..],
            Some(selected_host.clone()),
            Message::HostSelected,
        )
        .placeholder("Select host")
        .width(Length::Fixed(120.0))
    )
    .width(Length::Fill)
    .align_x(iced::alignment::Horizontal::Right);

    // Manual host input (if needed)
    let manual_host_input: Element<Message> = if selected_host == HostType::Manual {
        Container::new(
            TextInput::new("Enter host...", manual_host)
                .on_input(Message::ManualHostChanged)
                .style(InputStyle)
                .width(Length::Fill)
                .padding(12)
        )
        .padding([0, 0, 16, 0])
        .into()
    } else {
        Space::new(Length::Fill, Length::Fixed(0.0)).into()
    };

    // Title
    let title = Text::new("Ruggine")
        .size(36)
        .style(Color::from_rgb(1.0, 1.0, 1.0))
        .horizontal_alignment(iced::alignment::Horizontal::Center);

    // Tabs
    let login_tab = Button::new(
        Text::new("Login")
            .horizontal_alignment(iced::alignment::Horizontal::Center)
    )
    .on_press(if !is_login { Message::ToggleLoginRegister } else { Message::None })
    .style(TabButtonStyle { is_active: is_login })
    .width(Length::Fill)
    .padding([8, 16]);

    let register_tab = Button::new(
        Text::new("Register")
            .horizontal_alignment(iced::alignment::Horizontal::Center)
    )
    .on_press(if is_login { Message::ToggleLoginRegister } else { Message::None })
    .style(TabButtonStyle { is_active: !is_login })
    .width(Length::Fill)
    .padding([8, 16]);

    let tabs = Row::new()
        .spacing(0)
        .push(login_tab)
        .push(register_tab);

    // Input fields
    let email_input = TextInput::new("Email", username)
        .on_input(Message::UsernameChanged)
        .style(InputStyle)
        .width(Length::Fill)
        .padding(12);

    let password_input = TextInput::new("Password", password)
        .on_input(Message::PasswordChanged)
        .password()
        .style(InputStyle)
        .width(Length::Fill)
        .padding(12);

    // Submit button
    let submit_button = if submit_enabled {
        Button::new(
            Text::new(if is_login { "Login" } else { "Register" })
                .horizontal_alignment(iced::alignment::Horizontal::Center)
        )
        .on_press(Message::SubmitLoginOrRegister)
        .style(PrimaryButtonStyle)
        .width(Length::Fill)
        .padding(12)
    } else {
        Button::new(
            Text::new(if is_login { "Login" } else { "Register" })
                .horizontal_alignment(iced::alignment::Horizontal::Center)
        )
        .style(PrimaryButtonStyle)
        .width(Length::Fill)
        .padding(12)
    };

    // Error message
    let error_element: Element<Message> = if let Some(msg) = error_message {
        Text::new(msg)
            .style(Color::from_rgb(1.0, 0.4, 0.4))
            .horizontal_alignment(iced::alignment::Horizontal::Center)
            .into()
    } else {
        Space::new(Length::Fill, Length::Fixed(0.0)).into()
    };

    // Loading indicator
    let loading_element: Element<Message> = if loading {
        Text::new("Loading...")
            .style(Color::from_rgb(0.6, 0.8, 1.0))
            .horizontal_alignment(iced::alignment::Horizontal::Center)
            .into()
    } else {
        Space::new(Length::Fill, Length::Fixed(0.0)).into()
    };

    // Main card content
    let card_content = Column::new()
        .width(Length::Fixed(400.0))
        .spacing(20)
        .padding(32)
        .align_items(Alignment::Center)
        .push(title)
        .push(Space::new(Length::Fill, Length::Fixed(20.0)))
        .push(tabs)
        .push(Space::new(Length::Fill, Length::Fixed(20.0)))
        .push(email_input)
        .push(password_input)
        .push(Space::new(Length::Fill, Length::Fixed(10.0)))
        .push(submit_button)
        .push(error_element)
        .push(loading_element);

    let card = Container::new(card_content)
        .style(CardStyle)
        .center_x()
        .center_y();

    // Main layout with host selector at top
    let main_content = Column::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .push(
            Container::new(host_selector)
                .width(Length::Fill)
                .padding([16, 20, 0, 20])
        )
        .push(manual_host_input)
        .push(
            Container::new(card)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x()
                .center_y()
        );

    Container::new(main_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(DarkContainerStyle)
        .into()
}