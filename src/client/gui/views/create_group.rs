use iced::{Element, Length, Alignment, Color, Font};
use iced::widget::{Column, Row, Text, TextInput, Button, Container, Space};
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
        .push(Text::new("Create a group and invite friends").size(14).style(TEXT_SECONDARY));

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
    let submit_enabled = group_name_valid && !state.loading;

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
                    .on_submit(if submit_enabled { Message::CreateGroupSubmit } else { Message::None })
                    .width(Length::Fill)
                    .padding(12)
                    .size(14)
            )
            .style(iced::theme::Container::Custom(Box::new(input_appearance)))
        );

    // Validation indicator
    let validation_indicator = Row::new()
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
        .width(Length::Fixed(420.0))
        .spacing(24)
        .padding(32)
        .align_items(Alignment::Center)
        .push(
            Column::new()
                .spacing(8)
                .align_items(Alignment::Center)
                .push(Text::new("👥").font(EMOJI_FONT).size(48).style(ACCENT_COLOR))
                .push(Text::new("New Group").font(BOLD_FONT).size(24).style(TEXT_PRIMARY))
                .push(Text::new("Create a group to chat with multiple friends").size(14).style(TEXT_SECONDARY))
        )
        .push(Space::new(Length::Fill, Length::Fixed(16.0)))
        .push(group_name_field)
        .push(Space::new(Length::Fill, Length::Fixed(8.0)))
        .push(validation_indicator)
        .push(Space::new(Length::Fill, Length::Fixed(16.0)))
        .push(submit_button)
        .push(loading_element);

    let card = Container::new(card_content)
        .style(iced::theme::Container::Custom(Box::new(card_appearance)))
        .center_x()
        .center_y();

    // Main layout
    let main_content = Column::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .push(logger_bar)
        .push(header)
        .push(Space::new(Length::Fill, Length::Fixed(16.0)))
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