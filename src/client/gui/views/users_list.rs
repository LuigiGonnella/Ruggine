use iced::{Element, Length, Alignment, Color};
use iced::widget::{Column, Row, Text, Button, Container, TextInput, Scrollable, Space};
use crate::client::models::messages::Message;
use crate::client::models::app_state::ChatAppState;

// Modern color palette consistent with registration.rs and main_actions.rs
const BG_MAIN: Color = Color::from_rgb(0.06, 0.07, 0.18);
const CARD_BG: Color = Color::from_rgb(0.18, 0.19, 0.36);
const INPUT_BG: Color = Color::from_rgb(0.12, 0.13, 0.26);
const TEXT_PRIMARY: Color = Color::WHITE;
const TEXT_SECONDARY: Color = Color::from_rgb(0.7, 0.7, 0.7);

use iced::Font;
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
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.2),
        },
    }
}

fn search_container_appearance(_: &iced::Theme) -> iced::widget::container::Appearance {
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

fn user_item_appearance(_: &iced::Theme) -> iced::widget::container::Appearance {
    iced::widget::container::Appearance {
        background: Some(iced::Background::Color(CARD_BG)),
        text_color: Some(TEXT_PRIMARY),
        border: iced::Border {
            width: 1.0,
            color: Color::from_rgb(0.2, 0.2, 0.3),
            radius: 12.0.into(),
        },
        shadow: iced::Shadow {
            offset: iced::Vector::new(0.0, 2.0),
            blur_radius: 6.0,
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.2),
        },
    }
}

pub fn view<'a>(state: &'a ChatAppState, kind: &'a str) -> Element<'a, Message> {
    // Modern header with back button and title
    let back_button = Button::new(
        Container::new(
            Row::new()
                .spacing(8)
                .align_items(Alignment::Center)
                .push(Text::new("←").font(EMOJI_FONT).size(18))
                .push(Text::new("Back").font(BOLD_FONT).size(14))
        )
        .width(Length::Fill)
        .center_x()
    )
    .style(iced::theme::Button::Secondary)
    .on_press(Message::OpenMainActions)
    .padding(12)
    .width(Length::Fixed(100.0));

    let title_section = Column::new()
        .spacing(4)
        .align_items(Alignment::Center)
        .push(
            Row::new()
                .spacing(8)
                .align_items(Alignment::Center)
                .push(Text::new("👥").font(EMOJI_FONT).size(24))
                .push(Text::new(format!("{} Users", kind)).font(BOLD_FONT).size(24).style(TEXT_PRIMARY))
        )
        .push(Text::new("Find and connect with users").size(14).style(TEXT_SECONDARY));

    let header_row = Row::new()
        .spacing(16)
        .align_items(Alignment::Center)
        .push(back_button)
        .push(Container::new(title_section).width(Length::Fill).center_x())
        .push(Space::new(Length::Fixed(100.0), Length::Fixed(0.0))); // Balance space

    let header = Container::new(header_row)
        .padding([20, 24])
        .width(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(header_appearance)));

    // Modern search section
    let search_input_field = Container::new(
        TextInput::new("Search username...", &state.users_search_query)
            .on_input(Message::UsersSearchQueryChanged)
            .on_submit(Message::UsersSearch) // Make submittable with Enter key
            .padding(12)
            .size(14)
            .width(Length::Fill)
    )
    .style(iced::theme::Container::Custom(Box::new(input_appearance)));

    let search_button = Button::new(
        Container::new(
            Row::new()
                .spacing(8)
                .align_items(Alignment::Center)
                .push(Text::new("🔍").font(EMOJI_FONT).size(16))
                .push(Text::new("Search").font(BOLD_FONT).size(14))
        )
        .width(Length::Fill)
        .center_x()
    )
    .style(iced::theme::Button::Primary)
    .on_press(Message::UsersSearch)
    .padding(12)
    .width(Length::Fixed(120.0));

    let search_row = Row::new()
        .spacing(12)
        .align_items(Alignment::Center)
        .push(search_input_field)
        .push(search_button);

    let search_section = Container::new(
        Column::new()
            .spacing(12)
            .padding(24)
            .push(
                Row::new()
                    .spacing(8)
                    .align_items(Alignment::Center)
                    .push(Text::new("🔍").font(EMOJI_FONT).size(18))
                    .push(Text::new("Search Users").font(BOLD_FONT).size(16).style(TEXT_PRIMARY))
            )
            .push(search_row)
    )
    .width(Length::Fill)
    .style(iced::theme::Container::Custom(Box::new(search_container_appearance)));

    // Modern results list
    let mut list_col = Column::new().spacing(8);
    if state.users_search_results.is_empty() {
        list_col = list_col.push(
            Container::new(
                Column::new()
                    .spacing(8)
                    .align_items(Alignment::Center)
                    .push(Text::new("🔍").font(EMOJI_FONT).size(32).style(TEXT_SECONDARY))
                    .push(Text::new("No users found").font(BOLD_FONT).size(16).style(TEXT_SECONDARY))
                    .push(Text::new("Try searching for a different username").size(14).style(TEXT_SECONDARY))
            )
            .width(Length::Fill)
            .center_x()
            .padding(40)
        );
    } else {
        for username in state.users_search_results.iter() {
            let user_item = Container::new(
                Row::new()
                    .spacing(16)
                .align_items(Alignment::Center)
                    .push(
                        Container::new(
                            Text::new("👤").font(EMOJI_FONT).size(20)
                        )
                        .padding(8)
                        .style(iced::theme::Container::Custom(Box::new(|_: &iced::Theme| {
                            iced::widget::container::Appearance {
                                background: Some(iced::Background::Color(INPUT_BG)),
                                border: iced::Border {
                                    radius: 8.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }
                        })))
                    )
                    .push(
                        Column::new()
                            .spacing(2)
                            .push(Text::new(username).font(BOLD_FONT).size(16).style(TEXT_PRIMARY))
                            .push(Text::new("User").size(12).style(TEXT_SECONDARY))
                    )
                    .push(Space::new(Length::Fill, Length::Fixed(0.0)))
                    .push(
                        Button::new(
                            Container::new(
                                Row::new()
                                    .spacing(6)
                                    .align_items(Alignment::Center)
                                    .push(Text::new("💬").font(EMOJI_FONT).size(14))
                                    .push(Text::new("Message").font(BOLD_FONT).size(12))
                            )
                            .width(Length::Fill)
                            .center_x()
                        )
                        .style(iced::theme::Button::Primary)
                        .on_press(Message::OpenPrivateChat(username.clone()))
                        .padding(10)
                        .width(Length::Fixed(100.0))
                    )
            )
            .padding(16)
            .width(Length::Fill)
            .style(iced::theme::Container::Custom(Box::new(user_item_appearance)));
            
            list_col = list_col.push(user_item);
        }
    }

    let results_section = Container::new(
        Column::new()
            .spacing(12)
            .padding([24, 24, 0, 24])
            .push(
                Row::new()
                    .spacing(8)
                    .align_items(Alignment::Center)
                    .push(Text::new("📋").font(EMOJI_FONT).size(18))
                    .push(Text::new("Search Results").font(BOLD_FONT).size(16).style(TEXT_PRIMARY))
                    .push(Space::new(Length::Fill, Length::Fixed(0.0)))
                    .push(
                        Container::new(
                            Text::new(format!("{} users", state.users_search_results.len()))
                                .size(12)
                                .style(TEXT_SECONDARY)
                        )
                        .padding([4, 8])
                        .style(iced::theme::Container::Custom(Box::new(|_: &iced::Theme| {
                            iced::widget::container::Appearance {
                                background: Some(iced::Background::Color(INPUT_BG)),
                                border: iced::Border {
                                    radius: 12.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }
                        })))
                    )
            )
            .push(
                Scrollable::new(list_col)
                    .width(Length::Fill)
                    .height(Length::Fill)
            )
    )
    .width(Length::Fill)
    .height(Length::Fill);

    // Main content layout
    let content = Column::new()
        .push(header)
        .push(Space::new(Length::Fill, Length::Fixed(16.0)))
        .push(
            Container::new(search_section)
                .padding([0, 24])
        )
        .push(Space::new(Length::Fill, Length::Fixed(16.0)))
        .push(results_section)
        .push(Space::new(Length::Fill, Length::Fixed(24.0)))
        .width(Length::Fill)
        .height(Length::Fill);

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(bg_main_appearance)))
        .into()
}
