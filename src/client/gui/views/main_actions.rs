use iced::{Element, Length, Alignment, Color, Font};
use iced::widget::{Column, Row, Text, Button, Container, Space};
use crate::client::models::messages::Message;
use crate::client::models::app_state::ChatAppState;
use crate::client::gui::views::logger::logger_view;

// Modern color palette consistent with registration.rs
const BG_MAIN: Color = Color::from_rgb(0.06, 0.07, 0.18); // Deep navy
const CARD_BG: Color = Color::from_rgb(0.18, 0.19, 0.36); // Muted indigo for card bodies
const INPUT_BG: Color = Color::from_rgb(0.12, 0.13, 0.26); // Input background
const ACCENT_COLOR: Color = Color::from_rgb(0.0, 0.7, 0.3); // Green accent
const TEXT_PRIMARY: Color = Color::WHITE;
const TEXT_SECONDARY: Color = Color::from_rgb(0.7, 0.7, 0.7);

const EMOJI_FONT: Font = Font::with_name("Segoe UI Emoji");
const BOLD_FONT: Font = Font {
    family: iced::font::Family::SansSerif,
    weight: iced::font::Weight::Bold,
    ..Font::DEFAULT
};

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

fn header_appearance(_: &iced::Theme) -> iced::widget::container::Appearance {
    iced::widget::container::Appearance {
        background: Some(iced::Background::Color(INPUT_BG)),
        text_color: Some(TEXT_PRIMARY),
        border: iced::Border {
            width: 0.0,
            color: Color::TRANSPARENT,
            radius: 0.0.into(),
        },
        shadow: iced::Shadow {
            offset: iced::Vector::new(0.0, 2.0),
            blur_radius: 8.0,
            color: Color::TRANSPARENT,
        },
    }
}

// Build a modern action card with icon, title, detail and buttons
fn action_card<'a>(icon: &'a str, title: &'a str, detail: &'a str, btn_label: &'a str, action: Message, secondary: Option<(&'a str, Message)>) -> Element<'a, Message> {
    let title_row = Row::new()
        .spacing(if title == "Invites" { 8 } else { 12 })
        .align_items(Alignment::Center)
        .push(Text::new(icon).font(EMOJI_FONT).size(24).style(TEXT_PRIMARY))
        .push(Text::new(title).font(BOLD_FONT).size(20).style(TEXT_PRIMARY));

    let description = Text::new(detail).size(14).style(Color::from_rgb(0.85, 0.85, 0.85)); // Lighter text for better visibility

    let primary_btn = Button::new(
        Text::new(btn_label)
            .font(BOLD_FONT)
            .size(14)
            .style(TEXT_PRIMARY)
    )
    .style(iced::theme::Button::Primary)
    .on_press(action)
    .padding(12);

    let mut content = Column::new()
        .spacing(16)
        .padding(24)
        .push(title_row)
        .push(description)
        .push(
            Container::new(primary_btn)
                .width(Length::Fill)
                .center_x()
        );

    if let Some((link_label, link_msg)) = secondary {
        let secondary_btn = Button::new(
            Text::new(link_label)
                .size(14)
                .style(Color::from_rgb(0.5, 0.5, 0.6)) // Darker for better contrast
        )
        .style(iced::theme::Button::Secondary)
        .on_press(link_msg)
        .padding(10);
        
        content = content.push(
            Container::new(secondary_btn)
                .width(Length::Fill)
                .center_x()
        );
    }

    Container::new(content)
        .width(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(card_appearance)))
        .into()
}

pub fn view(state: &ChatAppState) -> Element<Message> {
    // Modern header with title and logout button
    let logout_button = Button::new(
        Container::new(
            Row::new()
                .spacing(8)
                .align_items(Alignment::Center)
                .push(Text::new("🚪").font(EMOJI_FONT).size(16))
                .push(Text::new("Logout").font(BOLD_FONT).size(14))
        )
        .width(Length::Fill)
        .center_x()
    )
            .style(iced::theme::Button::Destructive)
            .on_press(Message::Logout)
            .padding(12)
            .width(Length::Fixed(100.0));

    let title_section = Column::new()
        .spacing(4)
        .align_items(Alignment::Center)
        .push(Text::new("Ruggine").font(BOLD_FONT).size(32).style(TEXT_PRIMARY))
        .push(Text::new("Secure Chat Platform").size(14).style(TEXT_SECONDARY));

    let header_row = Row::new()
        .spacing(16)
        .align_items(Alignment::Center)
        .push(Space::new(Length::Fixed(100.0), Length::Fixed(0.0))) // Balance space
        .push(Container::new(title_section).width(Length::Fill).center_x())
        .push(logout_button);

    let header = Container::new(header_row)
        .padding([20, 24])
        .width(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(header_appearance)));

    // User info section
    println!("🟠 [DEBUG] MainActions rendering - state.username: '{}'", state.username);
    let user_info = Container::new(
        Row::new()
            .spacing(8)
        .align_items(Alignment::Center)
            .push(Text::new("👤").font(EMOJI_FONT).size(16).style(TEXT_SECONDARY))
            .push(Text::new("Logged in as:").size(14).style(TEXT_SECONDARY))
            .push(Text::new(&state.username).font(BOLD_FONT).size(14).style(ACCENT_COLOR))
    )
    .width(Length::Fill)
    .center_x()
    .padding([0, 24, 16, 24]);

    // Action cards with modern styling
    let users_card = action_card(
        "👤",
        "Users",
        "Browse and start private chats",
        "Online Users",
        Message::ListOnlineUsers,
        Some(("All Users", Message::ListAllUsers))
    );

    let groups_card = action_card(
        "👥", 
        "Groups", 
        "Open group chats and manage groups", 
        "My Groups", 
        Message::MyGroups, 
        Some(("Create Group", Message::CreateGroup { name: String::new() }))
    );

    let invites_card = action_card(
        "✉️",
        "Invites to groups", 
        "See pending group invites and accept or reject",
        "View Invites",
        Message::OpenMyGroupInvites,
        Some(("Friend Requests", Message::OpenFriendRequests))
    );

    let friends_card = action_card(
        "🧑‍🤝‍🧑",
        "Friends",
        "Your friends list and quick actions",
        "View Friends",
        Message::OpenViewFriends,
        Some(("Send Friend Request", Message::OpenSendFriendRequest))
    );

    // Cards container with proper spacing
    let cards_container = Column::new()
        .spacing(20)
        .padding([0, 24])
        .push(users_card)
        .push(groups_card)
        .push(invites_card)
        .push(friends_card);

    // Top logger bar
    let logger_bar: Element<Message> = if !state.logger.is_empty() {
        Container::new(logger_view(&state.logger))
            .width(Length::Fill)
            .padding([8, 12, 0, 12])
            .into()
    } else {
        Space::new(Length::Fill, Length::Fixed(0.0)).into()
    };

    // Main content with scrollable area
    let main_content = Column::new()
        .push(header)
        .push(user_info)
        .push(
            iced::widget::scrollable(cards_container)
                .width(Length::Fill)
                .height(Length::Fill)
        )
        .push(Space::new(Length::Fill, Length::Fixed(24.0)))
        .width(Length::Fill)
        .height(Length::Fill);

    // Render logger bar above main content
    let final_content = Column::new()
        .push(logger_bar)
        .push(main_content)
        .width(Length::Fill)
        .height(Length::Fill);

    Container::new(final_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(bg_main_appearance)))
        .into()
}