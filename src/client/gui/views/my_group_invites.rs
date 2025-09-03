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

fn invite_item_appearance(_: &iced::Theme) -> iced::widget::container::Appearance {
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
                .push(Text::new("✉️").font(EMOJI_FONT).size(24))
                .push(Text::new("Group Invites").font(BOLD_FONT).size(24).style(TEXT_PRIMARY))
        )
        .push(Text::new("Accept or reject group invitations").size(14).style(TEXT_SECONDARY));

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
    let content = if state.loading_invites {
        // Loading state
        Container::new(
            Column::new()
                .spacing(16)
                .align_items(Alignment::Center)
                .push(Text::new("⏳").font(EMOJI_FONT).size(32).style(TEXT_SECONDARY))
                .push(Text::new("Loading invites...").font(BOLD_FONT).size(16).style(TEXT_SECONDARY))
                .push(Text::new("Please wait while we fetch your group invitations").size(14).style(TEXT_SECONDARY))
        )
        .width(Length::Fill)
        .center_x()
        .padding(40)
    } else if state.my_group_invites.is_empty() {
        // Empty state
        Container::new(
            Column::new()
                .spacing(16)
                .align_items(Alignment::Center)
                .push(Text::new("✉️").font(EMOJI_FONT).size(48).style(TEXT_SECONDARY))
                .push(Text::new("No group invites").font(BOLD_FONT).size(20).style(TEXT_SECONDARY))
                .push(Text::new("You don't have any pending group invitations.").size(14).style(TEXT_SECONDARY))
                .push(Text::new("When someone invites you to a group, you'll see it here!").size(14).style(TEXT_SECONDARY))
        )
        .width(Length::Fill)
        .center_x()
        .padding(40)
        .style(iced::theme::Container::Custom(Box::new(empty_state_appearance)))
    } else {
        // Invites list
        let mut invites_column = Column::new().spacing(12);
        
        for (invite_id, group_name, invited_by) in &state.my_group_invites {
            let invite_item = Container::new(
                Row::new()
                    .spacing(16)
                    .align_items(Alignment::Center)
                    .push(
                        Container::new(
                            Text::new("👥").font(EMOJI_FONT).size(24)
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
                            .push(Text::new(group_name).font(BOLD_FONT).size(16).style(TEXT_PRIMARY))
                            .push(
                                Text::new(format!("Invited by {}", invited_by))
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
                                .on_press(Message::RejectGroupInvite { invite_id: *invite_id })
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
                                .on_press(Message::AcceptGroupInvite { invite_id: *invite_id })
                                .padding(10)
                                .width(Length::Fixed(80.0))
                            )
                    )
            )
            .padding(16)
            .width(Length::Fill)
            .style(iced::theme::Container::Custom(Box::new(invite_item_appearance)));
            
            invites_column = invites_column.push(invite_item);
        }

        Container::new(
            Scrollable::new(invites_column)
                .width(Length::Fill)
                .height(Length::Fill)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([0, 24])
    };

    // Main layout
    let main_content = Column::new()
        .push(logger_bar)
        .push(header)
        .push(Space::new(Length::Fill, Length::Fixed(16.0)))
        .push(content)
        .push(Space::new(Length::Fill, Length::Fixed(24.0)))
        .width(Length::Fill)
        .height(Length::Fill);

    Container::new(main_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(bg_main_appearance)))
        .into()
}