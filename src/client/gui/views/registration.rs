use iced::{Element, Length, Alignment, Color, Background, Border, Theme, Font};
use iced::widget::{Column, Row, Text, TextInput, Button, PickList, Container, Space};
use crate::client::models::messages::Message;
use crate::client::models::app_state::ChatAppState;
use crate::client::gui::views::logger::logger_view;

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

pub fn view(state: &ChatAppState) -> Element<Message> {
    let username = &state.username;
    let password = &state.password;
    let selected_host = state.selected_host.clone();
    let manual_host = &state.manual_host;
    let is_login = state.is_login;
    let error_message = state.error_message.clone();
    let loading = state.loading;
    let show_password = state.show_password; // aggiungi show_password a ChatAppState

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
        .style(Color::from_rgb(0.18, 0.22, 0.28)) // blu scuro
        .horizontal_alignment(iced::alignment::Horizontal::Center);

    // Tabs - usando stili built-in
    let login_tab = if is_login {
        Button::new(
            Text::new("Login")
                .horizontal_alignment(iced::alignment::Horizontal::Center)
                .style(Color::from_rgb(0.18, 0.22, 0.28)) // blu scuro
        )
        .style(iced::theme::Button::Primary)
        .width(Length::Fill)
        .padding([8, 16])
    } else {
        Button::new(
            Text::new("Login")
                .horizontal_alignment(iced::alignment::Horizontal::Center)
                .style(Color::from_rgb(0.4, 0.45, 0.5)) // grigio scuro
        )
        .on_press(Message::ToggleLoginRegister)
        .style(iced::theme::Button::Secondary)
        .width(Length::Fill)
        .padding([8, 16])
    };

    let register_tab = if !is_login {
        Button::new(
            Text::new("Register")
                .horizontal_alignment(iced::alignment::Horizontal::Center)
                .style(Color::from_rgb(0.18, 0.22, 0.28)) // blu scuro
        )
        .style(iced::theme::Button::Primary)
        .width(Length::Fill)
        .padding([8, 16])
    } else {
        Button::new(
            Text::new("Register")
                .horizontal_alignment(iced::alignment::Horizontal::Center)
                .style(Color::from_rgb(0.4, 0.45, 0.5)) // grigio scuro
        )
        .on_press(Message::ToggleLoginRegister)
        .style(iced::theme::Button::Secondary)
        .width(Length::Fill)
        .padding([8, 16])
    };

    let tabs = Row::new()
        .spacing(0)
        .push(login_tab)
        .push(register_tab);

    // Input fields
    let email_input = TextInput::new("Email", username)
        .on_input(Message::UsernameChanged)
        .width(Length::Fill)
        .padding(12);

    let password_input = TextInput::new("Password", password)
        .on_input(Message::PasswordChanged)
        .secure(!show_password)
        .width(Length::Fill)
        .padding(12);
    let toggle_button = Button::new(
        Text::new(if show_password { "🙈" } else { "👁️" })
            .font(Font::with_name("Segoe UI Emoji"))
    )
    .on_press(Message::ToggleShowPassword)
    .padding([0, 8]);
    // Rimossa password_overlay, ora non serve più
    let password_row = Row::new()
        .push(password_input)
        .push(toggle_button);

    // Submit button
    let submit_button = if submit_enabled {
        Button::new(
            Text::new(if is_login { "Login" } else { "Register" })
                .horizontal_alignment(iced::alignment::Horizontal::Center)
                .style(Color::from_rgb(1.0, 1.0, 1.0))
        )
        .on_press(Message::SubmitLoginOrRegister)
        .style(iced::theme::Button::Primary)
        .width(Length::Fill)
        .padding(12)
    } else {
        Button::new(
            Text::new(if is_login { "Login" } else { "Register" })
                .horizontal_alignment(iced::alignment::Horizontal::Center)
                .style(Color::from_rgb(0.6, 0.7, 0.8))
        )
        .style(iced::theme::Button::Secondary)
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
        .push(password_row)
        .push(Space::new(Length::Fill, Length::Fixed(10.0)))
        .push(submit_button)
        .push(logger_view(&state.logger))
        .push(error_element)
        .push(loading_element);

    let card = Container::new(card_content)
        .style(iced::theme::Container::Box)
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
        .style(iced::theme::Container::default())
        .into()
}