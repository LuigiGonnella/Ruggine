use iced::{Element, Length, Alignment, Color, Font};
use iced::widget::{Column, Row, Text, Button, Container, Space, row, text, button};
use iced_aw::card::Card;
use crate::client::models::messages::Message;
use crate::client::models::app_state::ChatAppState;

// Harmonized color palette (dark theme)
// Deep navy background and muted indigo cards for a calm, high-contrast look.
const BG_MAIN: Color = Color::from_rgb(0.06, 0.07, 0.18); // Deep navy
const CARD_BG: Color = Color::from_rgb(0.18, 0.19, 0.36); // Muted indigo for card bodies
const HEADER_BAR: Color = Color::from_rgb(0.12, 0.13, 0.26); // Slightly darker band for card headers
const TEXT_PRIMARY: Color = Color::WHITE; // White text for high contrast


// Emoji font and bold font constants (use for emoji and bold titles)
const EMOJI_FONT: Font = Font::with_name("Segoe UI Emoji");
const BOLD_FONT: Font = Font {
    family: iced::font::Family::SansSerif,
    weight: iced::font::Weight::Bold,
    ..Font::DEFAULT
};

// We use iced_aw::Card and built-in iced button themes to keep compatibility.

// Updated `CustomCardStyle` to ensure compatibility with `iced_aw::style::card::CardStyles`.
#[derive(Debug, Clone, Copy, Default)]
struct CustomCardStyle;

impl iced_aw::style::card::StyleSheet for CustomCardStyle {
    // The iced_aw card API expects custom styles to use `iced::Theme` as the
    // associated `Style` type so they can be wrapped in `CardStyles::Custom`.
    type Style = iced::Theme;

    fn active(&self, _: &iced::Theme) -> iced_aw::style::card::Appearance {
        iced_aw::style::card::Appearance {
            background: iced::Background::Color(CARD_BG),
            border_radius: 10.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            // Make card text and controls white for stronger contrast
            head_text_color: TEXT_PRIMARY,
            body_text_color: TEXT_PRIMARY,
            foot_text_color: TEXT_PRIMARY,
            close_color: TEXT_PRIMARY,
            // Use a slightly darker header band so the card head stands out
            head_background: HEADER_BAR.into(),
            body_background: iced::Background::Color(Color::TRANSPARENT),
            ..Default::default()
        }
    }
}

// Build a card with icon, title, detail and a full-width action button + optional secondary link
fn action_card<'a>(icon: &'a str, title: &'a str, detail: &'a str, btn_label: &'a str, action: Message, secondary: Option<(&'a str, Message)>) -> Element<'a, Message> {
    let title_row = Row::new()
        .push(Text::new(icon).font(EMOJI_FONT).size(22).style(iced::theme::Text::Default))
        .push(Space::new(Length::Fixed(8.0), Length::Fixed(0.0)))
        .push(Text::new(title).font(BOLD_FONT).size(20).style(iced::theme::Text::Default));

    let description = Text::new(detail).size(14).style(iced::theme::Text::Default);

    let mut col = Column::new()
        .push(title_row)
        .push(Space::new(Length::Fill, Length::Fixed(8.0)))
        .push(description)
        .push(Space::new(Length::Fill, Length::Fixed(12.0)));

    // Full width primary action button
    // Make action button a reasonable fixed width and center it (not full-screen)
    let action_btn = Button::new(Container::new(Text::new(btn_label)).width(Length::Fill).center_x())
        .style(iced::theme::Button::Primary)
        .on_press(action)
        .width(Length::Fixed(280.0))
        .padding(12);

    col = col.push(Row::new().push(Space::new(Length::Fill, Length::Fixed(0.0))).push(action_btn).push(Space::new(Length::Fill, Length::Fixed(0.0))));

    // Optional secondary link (text style) - centered under primary action with same width/padding
    if let Some((link_label, link_msg)) = secondary {
        col = col.push(Space::new(Length::Fill, Length::Fixed(8.0)));
        let link_btn = Button::new(Container::new(Text::new(link_label)).width(Length::Fill).center_x())
            .style(iced::theme::Button::Secondary)
            .on_press(link_msg)
            .width(Length::Fixed(280.0))
            .padding(10);
        col = col.push(Row::new().push(Space::new(Length::Fill, Length::Fixed(0.0))).push(link_btn).push(Space::new(Length::Fill, Length::Fixed(0.0))));
    }

    Card::new(Text::new(title).font(BOLD_FONT).style(iced::theme::Text::Default), col)
        .padding(16.into())
        .width(Length::Fill)
        .style(iced_aw::style::card::CardStyles::custom(CustomCardStyle))
        .into()
}

// Define a function to return the container appearance
fn bg_main_appearance(_: &iced::Theme) -> iced::widget::container::Appearance {
    iced::widget::container::Appearance {
        background: Some(iced::Background::Color(BG_MAIN)),
        text_color: None,
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

// Update card headers to have a lighter blue color
fn card_header_appearance(_: &iced::Theme) -> iced::widget::container::Appearance {
    iced::widget::container::Appearance {
        background: Some(iced::Background::Color(Color::from_rgb(0.3, 0.3, 0.7))), // Lighter blue for card headers
        text_color: Some(TEXT_PRIMARY),
        border: iced::Border {
            width: 0.0,
            color: Color::TRANSPARENT,
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
    // Header: dark bar with title left and a logout card on the right (emoji visible)
    // Build a small card that visually represents logout and contains a destructive button
    let logout_section = row![
        button(text( "Logout").font(BOLD_FONT))
            .style(iced::theme::Button::Destructive)
            .on_press(Message::Logout)
            .padding(8)
            .width(Length::Fixed(120.0)),
    ].spacing(1).align_items(Alignment::Center);

    // Center the title precisely while keeping logout on the right.
    let title_text = Text::new("Ruggine").font(BOLD_FONT).size(35).style(TEXT_PRIMARY);

    let header_row = Row::new()
        .align_items(Alignment::Center)
        .push(Container::new(title_text).width(Length::Fill).center_x())
        .push(Container::new(logout_section).width(Length::Fixed(120.0)).center_x());

    let header = Container::new(header_row)
        .padding([12, 18])
        .style(iced::theme::Container::Custom(Box::new(bg_main_appearance)));

    // Subtitle: white text with bold username for contrast
    let subtitle_row = Row::new()
        .align_items(Alignment::Center)
        .push(Text::new("Logged in as: ").size(14).style(TEXT_PRIMARY))
        .push(Text::new(state.username.clone()).font(BOLD_FONT).size(14).style(TEXT_PRIMARY));
    let subtitle = Container::new(subtitle_row);

    // Cards
    let users_card = Container::new(action_card(
        "👤",
        "Users",
        "Browse and start private chats",
        "Online Users",
        Message::ListOnlineUsers,
        Some(("All Users", Message::ListAllUsers))
    ))
        // Give each card vertical padding to reveal BG_MAIN between cards
        .padding([10, 18, 10, 18])
        .style(iced::theme::Container::Custom(Box::new(card_header_appearance)));
    let groups_card = Container::new(action_card("👥", "Groups", "Open group chats and manage groups", "My Groups", Message::MyGroups, Some(("Create Group", Message::CreateGroup { name: "NewGroup".to_string() }))))
        .padding([10, 18, 10, 18])
        .style(iced::theme::Container::Custom(Box::new(card_header_appearance)));
    let invites_card = Container::new(action_card("✉️", "Invites", "See pending invites and accept or reject", "View Invites", Message::GetGroupMessagesTest, Some(("Send Invites", Message::SendGroupMessageTest))))
        .padding([10, 18, 10, 18])
        .style(iced::theme::Container::Custom(Box::new(card_header_appearance)));
    let friends_card = Container::new(action_card(
        "🧑‍🤝‍🧑",
        "Friends",
        "Your friends list and quick actions",
        "View Friends",
        Message::OpenFriendRequests,
        Some(("Friend Requests", Message::OpenFriendRequests)),
    ))
    .padding([10, 18, 10, 18])
    .style(iced::theme::Container::Custom(Box::new(card_header_appearance)));
    let content = Column::new()
        .push(header)
        .push(Space::new(Length::Fill, Length::Fixed(12.0)))
        .push(Container::new(subtitle).padding([0, 8, 8, 8]))
        .push(Space::new(Length::Fill, Length::Fixed(8.0)))
        .push(users_card)
        .push(Space::new(Length::Fixed(0.0), Length::Fixed(12.0)))
        .push(groups_card)
        .push(Space::new(Length::Fixed(0.0), Length::Fixed(12.0)))
        .push(invites_card)
        .push(Space::new(Length::Fixed(0.0), Length::Fixed(12.0)))
        .push(friends_card)
        .width(Length::Fill)
        .align_items(Alignment::Center)
        .spacing(8);

    // Restore scrollable_content for scrolling functionality
    let scrollable_content = iced::widget::scrollable(content).width(Length::Fill).height(Length::Fill);

    Container::new(
        Column::new()
            .push(scrollable_content)
            .push(Space::new(Length::Fill, Length::Fixed(18.0)))
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x()
    .center_y()
    .style(iced::theme::Container::Custom(Box::new(bg_main_appearance)))
    .into()
}
