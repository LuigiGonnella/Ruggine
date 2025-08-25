use iced::{Element, Length, Alignment, Color, Font};
use iced::widget::{Column, Row, Text, TextInput, Button, Container, Space, Scrollable, Checkbox};
use crate::client::models::messages::Message;
use crate::client::models::app_state::ChatAppState;
use crate::client::gui::views::logger::logger_view;

// Modern color palette consistent with other views
const BG_MAIN: Color = Color::from_rgb(0.06, 0.07, 0.18);
const CARD_BG: Color = Color::from_rgb(0.18, 0.19, 0.36);
const INPUT_BG: Color = Color::from_rgb(0.12, 0.13, 0.26);
const ACCENT_COLOR: Color = Color::from_rgb(0.0, 0.7, 0.3);
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
                .push(Text::new("➕").font(EMOJI_FONT).size(24))
                .push(Text::new("Create New Group").font(BOLD_FONT).size(24).style(TEXT_PRIMARY))
        )
        .push(Text::new("Create a group and select participants").size(14).style(TEXT_SECONDARY));

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

    // Group name input validation
    let group_name_valid = !state.create_group_name.trim().is_empty() && state.create_group_name.len() >= 3;
    let has_participants = !state.selected_participants.is_empty();
    let submit_enabled = group_name_valid && has_participants && !state.loading;

    // Main form card
    let group_name_field = Column::new()
        .spacing(8)
        .push(
            Row::new()
                .spacing(8)
                .align_items(Alignment::Center)
                .push(Text::new("👥").font(EMOJI_FONT).size(16).style(TEXT_SECONDARY))
                .push(Text::new("Group Name").size(14).style(TEXT_SECONDARY))
        )
        .push(
            Container::new(
                TextInput::new("Enter group name", &state.create_group_name)
                    .on_input(Message::CreateGroupInputChanged)
                    .width(Length::Fill)
                    .padding(12)
                    .size(14)
            )
            .style(iced::theme::Container::Custom(Box::new(input_appearance)))
        );

    // Search field for participants
    let search_field = Column::new()
        .spacing(8)
        .push(
            Row::new()
                .spacing(8)
                .align_items(Alignment::Center)
                .push(Text::new("🔍").font(EMOJI_FONT).size(16).style(TEXT_SECONDARY))
                .push(Text::new("Search Users").size(14).style(TEXT_SECONDARY))
        )
        .push(
            Container::new(
                TextInput::new("Search username...", &state.users_search_query)
                    .on_input(Message::UsersSearchQueryChanged)
                    .on_submit(Message::UsersSearch)
                    .width(Length::Fill)
                    .padding(12)
                    .size(14)
            )
            .style(iced::theme::Container::Custom(Box::new(input_appearance)))
        );

    // Selected participants display
    let selected_section = if state.selected_participants.is_empty() {
        Column::new()
            .spacing(8)
            .push(
                Row::new()
                    .spacing(8)
                    .align_items(Alignment::Center)
                    .push(Text::new("👤").font(EMOJI_FONT).size(16).style(TEXT_SECONDARY))
                    .push(Text::new("Selected Participants").size(14).style(TEXT_SECONDARY))
            )
            .push(
                Container::new(
                    Text::new("No participants selected yet")
                        .size(12)
                        .style(TEXT_SECONDARY)
                )
                .padding(12)
                .width(Length::Fill)
                .style(iced::theme::Container::Custom(Box::new(input_appearance)))
            )
    } else {
        let mut selected_row = Row::new().spacing(8);
        for username in &state.selected_participants {
            selected_row = selected_row.push(
                Container::new(
                    Row::new()
                        .spacing(4)
                        .align_items(Alignment::Center)
                        .push(Text::new(username).size(12).style(TEXT_PRIMARY))
                        .push(
                            Button::new(Text::new("×").size(12))
                                .on_press(Message::RemoveParticipant(username.clone()))
                                .style(iced::theme::Button::Destructive)
                                .padding(2)
                        )
                )
                .padding([4, 8])
                .style(iced::theme::Container::Custom(Box::new(|_: &iced::Theme| {
                    iced::widget::container::Appearance {
                        background: Some(iced::Background::Color(ACCENT_COLOR)),
                        border: iced::Border {
                            radius: 12.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                })))
            );
        }

        Column::new()
            .spacing(8)
            .push(
                Row::new()
                    .spacing(8)
                    .align_items(Alignment::Center)
                    .push(Text::new("👤").font(EMOJI_FONT).size(16).style(TEXT_SECONDARY))
                    .push(Text::new(format!("Selected Participants ({})", state.selected_participants.len())).size(14).style(TEXT_SECONDARY))
            )
            .push(
                Container::new(selected_row)
                    .padding(12)
                    .width(Length::Fill)
                    .style(iced::theme::Container::Custom(Box::new(input_appearance)))
            )
    };

    // Available users list
    let mut users_list = Column::new().spacing(8);
    for username in &state.users_search_results {
        if username != &state.username && !state.selected_participants.contains(username) {
            let user_item = Container::new(
                Row::new()
                    .spacing(12)
                    .align_items(Alignment::Center)
                    .push(Text::new("👤").font(EMOJI_FONT).size(16))
                    .push(Text::new(username).size(14).style(TEXT_PRIMARY))
                    .push(Space::new(Length::Fill, Length::Fixed(0.0)))
                    .push(
                        Checkbox::new("", state.selected_participants.contains(username))
                            .on_toggle(move |_| Message::ToggleParticipant(username.clone()))
                    )
            )
            .padding(12)
            .width(Length::Fill)
            .style(iced::theme::Container::Custom(Box::new(user_item_appearance)));
            
            users_list = users_list.push(user_item);
        }
    }

    let users_section = Column::new()
        .spacing(8)
        .push(
            Row::new()
                .spacing(8)
                .align_items(Alignment::Center)
                .push(Text::new("📋").font(EMOJI_FONT).size(16).style(TEXT_SECONDARY))
                .push(Text::new("Available Users").size(14).style(TEXT_SECONDARY))
        )
        .push(
            Container::new(
                Scrollable::new(users_list)
                    .height(Length::Fixed(200.0))
            )
            .width(Length::Fill)
        );

    // Validation indicators
    let validation_indicators = Column::new()
        .spacing(4)
        .push(
            Row::new()
                .spacing(8)
                .align_items(Alignment::Center)
                .push(
                    Text::new(if group_name_valid { "✅" } else { "❌" })
                        .font(EMOJI_FONT)
                        .size(12)
                )
                .push(
                    Text::new("Group name (3+ characters)")
                        .size(12)
                        .style(if group_name_valid { ACCENT_COLOR } else { TEXT_SECONDARY })
                )
        )
        .push(
            Row::new()
                .spacing(8)
                .align_items(Alignment::Center)
                .push(
                    Text::new(if has_participants { "✅" } else { "❌" })
                        .font(EMOJI_FONT)
                        .size(12)
                )
                .push(
                    Text::new("At least one participant selected")
                        .size(12)
                        .style(if has_participants { ACCENT_COLOR } else { TEXT_SECONDARY })
                )
        );

    // Submit button
    let submit_button = if submit_enabled {
        Button::new(
            Container::new(
                Row::new()
                    .spacing(8)
                    .align_items(Alignment::Center)
                    .push(
                        Text::new("🚀")
                            .font(EMOJI_FONT)
                            .size(16)
                    )
                    .push(
                        Text::new("Create Group")
                            .font(BOLD_FONT)
                            .size(16)
                            .style(TEXT_PRIMARY)
                    )
            )
            .width(Length::Fill)
            .center_x()
        )
        .on_press(Message::CreateGroupSubmit)
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
                        Text::new(if state.loading { "Creating..." } else { "Create Group" })
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
    let loading_element: Element<Message> = if state.loading {
        Container::new(
            Row::new()
                .spacing(8)
                .align_items(Alignment::Center)
                .push(Text::new("⏳").font(EMOJI_FONT).size(16))
                .push(
                    Text::new("Creating group...")
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

    // Main card content
    let card_content = Column::new()
        .width(Length::Fixed(500.0))
        .spacing(20)
        .padding(32)
        .push(group_name_field)
        .push(search_field)
        .push(selected_section)
        .push(users_section)
        .push(validation_indicators)
        .push(Space::new(Length::Fill, Length::Fixed(8.0)))
        .push(submit_button)
        .push(loading_element);

    let card = Container::new(card_content)
        .style(iced::theme::Container::Custom(Box::new(card_appearance)))
        .center_x();

    // Main layout
    let main_content = Column::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .push(logger_bar)
        .push(header)
        .push(Space::new(Length::Fill, Length::Fixed(16.0)))
        .push(
            Scrollable::new(
                Container::new(card)
                    .width(Length::Fill)
                    .center_x()
                    .padding([0, 24])
            )
            .width(Length::Fill)
            .height(Length::Fill)
        );

    Container::new(main_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(bg_main_appearance)))
        .into()
}