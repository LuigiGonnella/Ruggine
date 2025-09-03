use iced::{Element, Length, Alignment, Color, Font};
use iced::widget::{Column, Row, Text, Button, Container, Space, Scrollable};
use crate::client::models::messages::Message;
use crate::client::models::app_state::ChatAppState;
use crate::client::gui::views::logger::logger_view;

// Modern color palette consistent with other views
const BG_MAIN: Color = Color::from_rgb(0.06, 0.07, 0.18);
const CARD_BG: Color = Color::from_rgb(0.18, 0.19, 0.36);
const INPUT_BG: Color = Color::from_rgb(0.12, 0.13, 0.26);
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

fn request_item_appearance(_: &iced::Theme) -> iced::widget::container::Appearance {
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

fn empty_state_appearance(_: &iced::Theme) -> iced::widget::container::Appearance {
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

pub fn view(state: &ChatAppState) -> Element<Message> {
    // Top logger bar
    let logger_bar = if !state.logger.is_empty() {
        Container::new(logger_view(&state.logger))
            .width(Length::Fill)
            .padding([8, 12, 0, 12])
            .style(iced::theme::Container::Custom(Box::new(|_: &iced::Theme| {
                iced::widget::container::Appearance {
                    background: Some(iced::Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.8))),
                    ..Default::default()
                }
            })))
    } else {
        Container::new(Space::new(Length::Fill, Length::Fixed(0.0)))
            .width(Length::Fill)
    };

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
                .push(Text::new("📨").font(EMOJI_FONT).size(24))
                .push(Text::new("Friend Requests").font(BOLD_FONT).size(24).style(TEXT_PRIMARY))
        )
        .push(Text::new("Accept or reject friend requests").size(14).style(TEXT_SECONDARY));

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

    // Content area
    let content = if state.loading {
        // Loading state
        Container::new(
            Column::new()
                .spacing(16)
                .align_items(Alignment::Center)
                .push(Text::new("⏳").font(EMOJI_FONT).size(32).style(TEXT_SECONDARY))
                .push(Text::new("Loading friend requests...").font(BOLD_FONT).size(16).style(TEXT_SECONDARY))
                .push(Text::new("Please wait while we fetch your friend requests").size(14).style(TEXT_SECONDARY))
        )
        .width(Length::Fill)
        .center_x()
        .padding(40)
    } else if state.friend_requests.is_empty() {
        // Empty state
        Container::new(
            Column::new()
                .spacing(16)
                .align_items(Alignment::Center)
                .push(Text::new("📨").font(EMOJI_FONT).size(48).style(TEXT_SECONDARY))
                .push(Text::new("No friend requests").font(BOLD_FONT).size(20).style(TEXT_SECONDARY))
                .push(Text::new("You don't have any pending friend requests.").size(14).style(TEXT_SECONDARY))
                .push(Text::new("When someone sends you a friend request, you'll see it here!").size(14).style(TEXT_SECONDARY))
        )
        .width(Length::Fill)
        .center_x()
        .padding(40)
        .style(iced::theme::Container::Custom(Box::new(empty_state_appearance)))
    } else {
        // Friend requests list
        let mut requests_column = Column::new().spacing(12);
        
        for (username, message) in &state.friend_requests {
            let request_item = Container::new(
                Row::new()
                    .spacing(16)
                    .align_items(Alignment::Center)
                    .push(
                        Container::new(
                            Text::new("👤").font(EMOJI_FONT).size(24)
                        )
                        .padding(12)
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
                            .spacing(4)
                            .push(Text::new(username).font(BOLD_FONT).size(16).style(TEXT_PRIMARY))
                            .push(
                                Text::new(if message.is_empty() { "Friend request" } else { message })
                                    .size(12)
                                    .style(TEXT_SECONDARY)
                            )
                    )
                    .push(Space::new(Length::Fill, Length::Fixed(0.0)))
                    .push(
                        Row::new()
                            .spacing(8)
                            .push(
                                Button::new(
                                    Container::new(
                                        Row::new()
                                            .spacing(6)
                                            .align_items(Alignment::Center)
                                            .push(Text::new("❌").font(EMOJI_FONT).size(14))
                                            .push(Text::new("Reject").font(BOLD_FONT).size(12))
                                    )
                                    .width(Length::Fill)
                                    .center_x()
                                )
                                .style(iced::theme::Button::Destructive)
                                .on_press(Message::RejectFriendRequestFromUser { username: username.clone() })
                                .padding(10)
                                .width(Length::Fixed(80.0))
                            )
                            .push(
                                Button::new(
                                    Container::new(
                                        Row::new()
                                            .spacing(6)
                                            .align_items(Alignment::Center)
                                            .push(Text::new("✅").font(EMOJI_FONT).size(14))
                                            .push(Text::new("Accept").font(BOLD_FONT).size(12))
                                    )
                                    .width(Length::Fill)
                                    .center_x()
                                )
                                .style(iced::theme::Button::Primary)
                                .on_press(Message::AcceptFriendRequestFromUser { username: username.clone() })
                                .padding(10)
                                .width(Length::Fixed(80.0))
                            )
                    )
            )
            .padding(16)
            .width(Length::Fill)
            .style(iced::theme::Container::Custom(Box::new(request_item_appearance)));
            
            requests_column = requests_column.push(request_item);
        }

        Container::new(
            Scrollable::new(requests_column)
                .width(Length::Fill)
                .height(Length::Fill)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([0, 24])
    };

    // Main layout
    let main_content = Column::new()
        .push(header)
        .push(Space::new(Length::Fill, Length::Fixed(16.0)))
        .push(content)
        .push(Space::new(Length::Fill, Length::Fixed(24.0)))
        .width(Length::Fill)
        .height(Length::Fill);

    // Main layout with logger overlay using Column
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
