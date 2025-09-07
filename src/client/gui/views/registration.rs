use iced::{Element, Length, Alignment, Color, Font};
use iced::widget::{Column, Row, Text, TextInput, Button, PickList, Container, Space};
use crate::client::models::messages::Message;
use crate::client::models::app_state::ChatAppState;
use crate::client::gui::views::logger::logger_view;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HostType {
    #[default]
    Localhost,
    Remote,
    Manual,
}

impl std::fmt::Display for HostType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            HostType::Localhost => "Localhost",
            HostType::Remote => "Remote", 
            HostType::Manual => "Manual",
        };
        write!(f, "{}", s)
    }
}

const ALL_HOSTS: [HostType; 3] = [HostType::Localhost, HostType::Remote, HostType::Manual];

impl HostType {
    pub fn all() -> &'static [HostType] {
        &ALL_HOSTS
    }
}

// Consistent color palette with main_actions and private_chat
const BG_MAIN: Color = Color::from_rgb(0.06, 0.07, 0.18); // Deep navy
const CARD_BG: Color = Color::from_rgb(0.18, 0.19, 0.36); // Muted indigo for card bodies
const INPUT_BG: Color = Color::from_rgb(0.12, 0.13, 0.26); // Input background
const ACCENT_COLOR: Color = Color::from_rgb(0.0, 0.7, 0.3); // Green accent
const TEXT_PRIMARY: Color = Color::WHITE;
const TEXT_SECONDARY: Color = Color::from_rgb(0.7, 0.7, 0.7);

const BOLD_FONT: Font = Font {
    family: iced::font::Family::SansSerif,
    weight: iced::font::Weight::Bold,
    ..Font::DEFAULT
};

const EMOJI_FONT: Font = Font::with_name("Segoe UI Emoji");

// Custom container styles
fn bg_main_appearance(_: &iced::Theme) -> iced::widget::container::Appearance {
    iced::widget::container::Appearance {
        background: Some(iced::Background::Color(BG_MAIN)),
        text_color: Some(TEXT_PRIMARY),
        border: iced::Border {
            width: 0.0,
            color: Color::TRANSPARENT,
            radius: 0.0.into(),
        },
        shadow: iced::Shadow {
            offset: iced::Vector::new(0.0, 0.0),
            blur_radius: 0.0,
            color: Color::TRANSPARENT,
        },
    }
}

fn card_appearance(_: &iced::Theme) -> iced::widget::container::Appearance {
    iced::widget::container::Appearance {
        background: Some(iced::Background::Color(CARD_BG)),
        text_color: Some(TEXT_PRIMARY),
        border: iced::Border {
            width: 0.0,
            color: Color::TRANSPARENT,
            radius: 16.0.into(),
        },
        shadow: iced::Shadow {
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 12.0,
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
        },
    }
}

fn input_appearance(_: &iced::Theme) -> iced::widget::container::Appearance {
    iced::widget::container::Appearance {
        background: Some(iced::Background::Color(INPUT_BG)),
        text_color: Some(TEXT_PRIMARY),
        border: iced::Border {
            width: 1.0,
            color: Color::from_rgb(0.3, 0.3, 0.4),
            radius: 12.0.into(),
        },
        shadow: iced::Shadow {
            offset: iced::Vector::new(0.0, 0.0),
            blur_radius: 0.0,
            color: Color::TRANSPARENT,
        },
    }
}

fn host_selector_appearance(_: &iced::Theme) -> iced::widget::container::Appearance {
    iced::widget::container::Appearance {
        background: Some(iced::Background::Color(INPUT_BG)),
        text_color: Some(TEXT_PRIMARY),
        border: iced::Border {
            width: 1.0,
            color: Color::from_rgb(0.3, 0.3, 0.4),
            radius: 8.0.into(),
        },
        shadow: iced::Shadow {
            offset: iced::Vector::new(0.0, 0.0),
            blur_radius: 0.0,
            color: Color::TRANSPARENT,
        },
    }
}

pub fn view(state: &ChatAppState) -> Element<Message> {
    let username = &state.username;
    let password = &state.password;
    let selected_host = state.selected_host;
    let manual_host = &state.manual_host;
    let is_login = state.is_login;
    let loading = state.loading;
    let show_password = state.show_password;

    // Validation
    let username_valid = !username.is_empty() && username.len() >= 3 && username.chars().all(|c| c.is_alphanumeric());
    let password_valid = !password.is_empty() && password.len() >= 6;
    let host_valid = match selected_host {
        HostType::Manual => !manual_host.is_empty() && manual_host.contains(':'),
        _ => true,
    };
    let submit_enabled = username_valid && password_valid && host_valid && !loading;

    // Top logger bar
    let logger_bar = if !state.logger.is_empty() {
        Container::new(logger_view(&state.logger))
            .width(Length::Fill)
            .padding([8, 12, 0, 12])
    } else {
        Container::new(Space::new(Length::Fill, Length::Fixed(0.0)))
            .width(Length::Fill)
    };

    // Host selector in top right with modern styling
    let host_selector = Container::new(
        Row::new()
            .spacing(8)
            .align_items(Alignment::Center)
            .push(Text::new("🌐").font(EMOJI_FONT).size(16).style(TEXT_SECONDARY))
            .push(
                PickList::new(
                    HostType::all(),
                    Some(selected_host),
                    Message::HostSelected,
                )
                .placeholder("Select host")
                .width(Length::Fixed(120.0))
            )
    )
    .padding(8)
    .style(iced::theme::Container::Custom(Box::new(host_selector_appearance)));

    let host_row = Container::new(
        Row::new()
            .push(Space::new(Length::Fill, Length::Fixed(0.0)))
            .push(host_selector)
    )
    .width(Length::Fill)
    .padding([16, 20, 0, 20]);

    // Manual host input with modern styling
    let manual_host_input: Element<Message> = if selected_host == HostType::Manual {
        Container::new(
            Column::new()
                .spacing(8)
                .push(
                    Text::new("Server Address")
                        .size(14)
                        .style(TEXT_SECONDARY)
                )
                .push(
                    Container::new(
                        TextInput::new("host:port (e.g., 127.0.0.1:5000)", manual_host)
                            .on_input(Message::ManualHostChanged)
                            .on_submit(if submit_enabled { Message::SubmitLoginOrRegister } else { Message::None })
                            .width(Length::Fill)
                            .padding(12)
                            .size(14)
                    )
                    .style(iced::theme::Container::Custom(Box::new(input_appearance)))
                )
        )
        .width(Length::Fixed(400.0))
        .padding([0, 0, 16, 0])
        .into()
    } else {
        Space::new(Length::Fill, Length::Fixed(0.0)).into()
    };

    // Main title with modern typography
    let title = Text::new("Ruggine")
        .size(42)
        .font(BOLD_FONT)
        .style(TEXT_PRIMARY)
        .horizontal_alignment(iced::alignment::Horizontal::Center);

    let subtitle = Text::new("Secure Chat Platform")
        .size(16)
        .style(TEXT_SECONDARY)
        .horizontal_alignment(iced::alignment::Horizontal::Center);

    // Modern tab system
    let login_tab = if is_login {
        Button::new(
            Container::new(
                Text::new("Login")
                    .font(BOLD_FONT)
                    .size(16)
                    .horizontal_alignment(iced::alignment::Horizontal::Center)
                    .style(TEXT_PRIMARY)
            )
            .width(Length::Fill)
            .center_x()
        )
        .style(iced::theme::Button::Primary)
        .width(Length::Fill)
        .padding([12, 16])
    } else {
        Button::new(
            Container::new(
                Text::new("Login")
                    .size(16)
                    .horizontal_alignment(iced::alignment::Horizontal::Center)
                    .style(TEXT_SECONDARY)
            )
            .width(Length::Fill)
            .center_x()
        )
        .on_press(Message::ToggleLoginRegister)
        .style(iced::theme::Button::Secondary)
        .width(Length::Fill)
        .padding([12, 16])
    };

    let register_tab = if !is_login {
        Button::new(
            Container::new(
                Text::new("Register")
                    .font(BOLD_FONT)
                    .size(16)
                    .horizontal_alignment(iced::alignment::Horizontal::Center)
                    .style(TEXT_PRIMARY)
            )
            .width(Length::Fill)
            .center_x()
        )
        .style(iced::theme::Button::Primary)
        .width(Length::Fill)
        .padding([12, 16])
    } else {
        Button::new(
            Container::new(
                Text::new("Register")
                    .size(16)
                    .horizontal_alignment(iced::alignment::Horizontal::Center)
                    .style(TEXT_SECONDARY)
            )
            .width(Length::Fill)
            .center_x()
        )
        .on_press(Message::ToggleLoginRegister)
        .style(iced::theme::Button::Secondary)
        .width(Length::Fill)
        .padding([12, 16])
    };

    let tabs = Row::new()
        .spacing(2)
        .push(login_tab)
        .push(register_tab);

    // Modern input fields with labels
    let username_field = Column::new()
        .spacing(8)
        .push(
            Row::new()
                .spacing(8)
                .align_items(Alignment::Center)
                .push(Text::new("👤").font(EMOJI_FONT).size(16).style(TEXT_SECONDARY))
                .push(Text::new("Username").size(14).style(TEXT_SECONDARY))
        )
        .push(
            Container::new(
                TextInput::new("Enter your username", username)
                    .on_input(Message::UsernameChanged)
                    .on_submit(if submit_enabled { Message::SubmitLoginOrRegister } else { Message::None })
                    .width(Length::Fill)
                    .padding(12)
                    .size(14)
            )
            .style(iced::theme::Container::Custom(Box::new(input_appearance)))
        );

    let password_field = Column::new()
        .spacing(8)
        .push(
            Row::new()
                .spacing(8)
                .align_items(Alignment::Center)
                .push(Text::new("🔒").font(EMOJI_FONT).size(16).style(TEXT_SECONDARY))
                .push(Text::new("Password").size(14).style(TEXT_SECONDARY))
        )
        .push(
            Container::new(
                Row::new()
                    .align_items(Alignment::Center)
                    .push(
                        TextInput::new("Enter your password", password)
                            .on_input(Message::PasswordChanged)
                            .on_submit(if submit_enabled { Message::SubmitLoginOrRegister } else { Message::None })
                            .secure(!show_password)
                            .width(Length::Fill)
                            .padding(12)
                            .size(14)
                    )
                    .push(
                        Button::new(
                            Text::new(if show_password { "🙈" } else { "👁️" })
                                .font(EMOJI_FONT)
                                .size(16)
                        )
                        .on_press(Message::ToggleShowPassword)
                        .style(iced::theme::Button::Text)
                        .padding([8, 12])
                    )
            )
            .style(iced::theme::Container::Custom(Box::new(input_appearance)))
        );

    // Validation indicators
    let validation_indicators = Column::new()
        .spacing(4)
        .push(
            Row::new()
                .spacing(8)
                .align_items(Alignment::Center)
                .push(
                    Text::new(if username_valid { "✅" } else { "❌" })
                        .font(EMOJI_FONT)
                        .size(12)
                )
                .push(
                    Text::new("Username (3+ alphanumeric characters)")
                        .size(12)
                        .style(if username_valid { ACCENT_COLOR } else { TEXT_SECONDARY })
                )
        )
        .push(
            Row::new()
                .spacing(8)
                .align_items(Alignment::Center)
                .push(
                    Text::new(if password_valid { "✅" } else { "❌" })
                        .font(EMOJI_FONT)
                        .size(12)
                )
                .push(
                    Text::new("Password (6+ characters)")
                        .size(12)
                        .style(if password_valid { ACCENT_COLOR } else { TEXT_SECONDARY })
                )
        );

    // Modern submit button
    let submit_button = if submit_enabled {
        Button::new(
            Container::new(
                Row::new()
                    .spacing(8)
                    .align_items(Alignment::Center)
                    .push(
                        Text::new(if is_login { "🚀" } else { "✨" })
                            .font(EMOJI_FONT)
                            .size(16)
                    )
                    .push(
                        Text::new(if is_login { "Sign In" } else { "Create Account" })
                            .font(BOLD_FONT)
                            .size(16)
                            .style(TEXT_PRIMARY)
                    )
            )
            .width(Length::Fill)
            .center_x()
        )
        .on_press(Message::SubmitLoginOrRegister)
        .style(iced::theme::Button::Primary)
        .width(Length::Fill)
        .padding(16)
    } else {
        Button::new(
            Container::new(
                Row::new()
                    .spacing(8)
                    .align_items(Alignment::Center)
                    .push(
                        Text::new("⏳")
                            .font(EMOJI_FONT)
                            .size(16)
                    )
                    .push(
                        Text::new(if loading { "Connecting..." } else if is_login { "Sign In" } else { "Create Account" })
                            .size(16)
                            .style(TEXT_SECONDARY)
                    )
            )
            .width(Length::Fill)
            .center_x()
        )
        .style(iced::theme::Button::Secondary)
        .width(Length::Fill)
        .padding(16)
    };

    // Loading indicator
    let loading_element: Element<Message> = if loading {
        Container::new(
            Row::new()
                .spacing(8)
                .align_items(Alignment::Center)
                .push(Text::new("⏳").font(EMOJI_FONT).size(16))
                .push(
                    Text::new("Establishing secure connection...")
                        .size(14)
                        .style(ACCENT_COLOR)
                )
        )
        .width(Length::Fill)
        .center_x()
        .padding(8)
        .into()
    } else {
        Space::new(Length::Fill, Length::Fixed(0.0)).into()
    };

    // Main card content with modern spacing and layout
    let card_content = Column::new()
        .width(Length::Fixed(420.0))
        .spacing(24)
        .padding(32)
        .align_items(Alignment::Center)
        .push(
            Column::new()
                .spacing(8)
                .align_items(Alignment::Center)
                .push(title)
                .push(subtitle)
        )
        .push(Space::new(Length::Fill, Length::Fixed(8.0)))
        .push(tabs)
        .push(Space::new(Length::Fill, Length::Fixed(8.0)))
        .push(username_field)
        .push(password_field)
        .push(Space::new(Length::Fill, Length::Fixed(8.0)))
        .push(validation_indicators)
        .push(Space::new(Length::Fill, Length::Fixed(8.0)))
        .push(submit_button)
        .push(loading_element);

    let card = Container::new(card_content)
        .style(iced::theme::Container::Custom(Box::new(card_appearance)))
        .center_x()
        .center_y();

    // Center the manual host input
    let centered_manual_input = Container::new(manual_host_input)
        .width(Length::Fill)
        .center_x();

    // Main layout
    let main_content = Column::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .push(logger_bar)
        .push(host_row)
        .push(centered_manual_input)
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
        .style(iced::theme::Container::Custom(Box::new(bg_main_appearance)))
        .into()
}