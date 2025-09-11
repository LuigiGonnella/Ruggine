use iced::{Element, Length, Alignment, Color, Font};
use iced::widget::{Column, Row, Text, TextInput, Button, Container, Scrollable, Space, scrollable};
use crate::client::models::messages::Message;
use crate::client::models::app_state::ChatAppState;

// Color palette per chat moderna (WhatsApp-like)
const BG_MAIN: Color = Color::from_rgb(0.06, 0.07, 0.18); // Deep navy
const CHAT_BG: Color = Color::from_rgb(0.08, 0.09, 0.20); // Slightly lighter for chat area
const MY_MESSAGE_BG: Color = Color::from_rgb(0.0, 0.7, 0.3); // Green for my messages (WhatsApp style)
const OTHER_MESSAGE_BG: Color = Color::from_rgb(0.2, 0.4, 0.8); // Blue for received messages
const INPUT_BG: Color = Color::from_rgb(0.12, 0.13, 0.26); // Input background
const TEXT_PRIMARY: Color = Color::WHITE;
const TEXT_SECONDARY: Color = Color::from_rgb(0.7, 0.7, 0.7);

const BOLD_FONT: Font = Font {
    family: iced::font::Family::SansSerif,
    weight: iced::font::Weight::Bold,
    ..Font::DEFAULT
};

const EMOJI_FONT: Font = Font::with_name("Segoe UI Emoji");


pub fn view<'a>(state: &'a ChatAppState, group_id: &'a str, group_name: &'a str) -> Element<'a, Message> {
    // Header con nome gruppo e pulsante back
    let back_btn = Button::new(Text::new("← Back").size(16))
        .on_press(Message::StopGroupMessagePolling)
        .style(iced::theme::Button::Secondary)
        .padding(8);

    let group_info = Column::new()
        .push(Text::new(group_name).font(BOLD_FONT).size(20).style(TEXT_PRIMARY))
        .push(Text::new("Group Chat").size(12).style(TEXT_SECONDARY))
        .spacing(2);

    let discard_btn = Button::new(Text::new("🗑️").font(EMOJI_FONT).size(16))
        .on_press(Message::DiscardGroupMessages { group_id: group_id.to_string() })
        .style(iced::theme::Button::Destructive)
        .padding(8);

    // Pulsante per aggiungere membri
    let add_member_btn = Button::new(Text::new("➕").font(EMOJI_FONT).size(16))
        .on_press(Message::OpenInviteToGroup { 
            group_id: group_id.to_string(), 
            group_name: group_name.to_string() 
        })
        .style(iced::theme::Button::Secondary)
        .padding(8);

    // Pulsante per lasciare il gruppo
    let leave_group_btn = Button::new(Text::new("🚪").font(EMOJI_FONT).size(16))
        .on_press(Message::LeaveGroup { 
            group_id: group_id.to_string(), 
            group_name: group_name.to_string() 
        })
        .style(iced::theme::Button::Destructive)
        .padding(8);

    let header = Container::new(
        Row::new()
            .spacing(12)
            .align_items(Alignment::Center)
            .push(back_btn)
            .push(group_info)
            .push(Space::new(Length::Fill, Length::Fixed(0.0)))
            .push(add_member_btn)
            .push(leave_group_btn)
            .push(discard_btn)
    )
    .padding([12, 16])
    .width(Length::Fill)
    .style(iced::theme::Container::Custom(Box::new(|_: &iced::Theme| {
        iced::widget::container::Appearance {
            background: Some(iced::Background::Color(INPUT_BG)),
            ..Default::default()
        }
    })));

    // Area messaggi
    let messages_area = build_messages_area(state, group_id);

    // Input area
    let input_area = build_input_area(state, group_id);

    // Layout principale
    let content = Column::new()
        .push(header)
        .push(messages_area)
        .push(input_area)
        .width(Length::Fill)
        .height(Length::Fill);

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(|_: &iced::Theme| {
            iced::widget::container::Appearance {
                background: Some(iced::Background::Color(BG_MAIN)),
                ..Default::default()
            }
        })))
        .into()
}

fn build_messages_area<'a>(state: &'a ChatAppState, group_id: &'a str) -> Element<'a, Message> {
    let mut messages_column = Column::new().spacing(8).padding([12, 16]);

    // Check if messages are discarded for this group

    // Show cached messages or appropriate placeholder
    if let Some(chat_messages) = state.group_chats.get(group_id) {
        if chat_messages.is_empty() {
            messages_column = messages_column.push(
                Container::new(
                    Text::new("Nessun messaggio ancora. Inizia la conversazione!")
                        .size(14)
                        .style(TEXT_SECONDARY)
                )
                .width(Length::Fill)
                .center_x()
                .padding(20)
            );
        } else {
            for msg in chat_messages.iter() {
                let is_my_message = msg.sender == state.username;
                let message_bubble = create_message_bubble(msg, is_my_message);
                messages_column = messages_column.push(message_bubble);
            }
        }
    } else {
        // No cached messages: show loading placeholder
        messages_column = messages_column.push(
            Container::new(
                Text::new("Caricamento messaggi...")
                    .size(14)
                    .style(TEXT_SECONDARY)
            )
            .width(Length::Fill)
            .center_x()
            .padding(20)
        );
    }

    // Aggiungi un po' di spazio in fondo per evitare che l'ultimo messaggio sia troppo vicino all'input
    messages_column = messages_column.push(Space::new(Length::Fixed(0.0), Length::Fixed(20.0)));

    // Scrollable container per i messaggi
    let scrollable_messages = Scrollable::new(messages_column)
            .width(Length::Fill)
            .height(Length::Fill)
            .id(scrollable::Id::new("group_messages_scroll"));

    Container::new(scrollable_messages)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(iced::theme::Container::Custom(Box::new(|_: &iced::Theme| {
        iced::widget::container::Appearance {
            background: Some(iced::Background::Color(CHAT_BG)),
            ..Default::default()
        }
    })))
    .into()
}

fn create_message_bubble(msg: &crate::client::models::app_state::ChatMessage, is_my_message: bool) -> Element<'_, Message> {
    let bubble_color = if is_my_message { MY_MESSAGE_BG } else { OTHER_MESSAGE_BG };

    // For group messages, show sender name if it's not my message
    let message_header = if !is_my_message {
        Some(
            Text::new(&msg.sender)
                .size(15)
                .font(BOLD_FONT)
                .style(Color::from_rgb(1.0, 1.0, 0.8)) // Slightly warm white for better visibility
        )
    } else {
        None
    };

    let mut message_content = Column::new().spacing(2);
    
    // Add sender name for group messages (only for others' messages)
    if let Some(header) = message_header {
        message_content = message_content.push(header);
    }
    
    message_content = message_content
        .push(Text::new(&msg.content).size(14).style(TEXT_PRIMARY))
        .push(Space::new(Length::Fixed(0.0), Length::Fixed(4.0)))
        .push(Text::new(&msg.formatted_time).size(10).style(TEXT_SECONDARY));

    let bubble = Container::new(message_content)
        .padding([8, 12])
        .style(iced::theme::Container::Custom(Box::new(move |_: &iced::Theme| {
            iced::widget::container::Appearance {
                background: Some(iced::Background::Color(bubble_color)),
                border: iced::Border {
                    radius: 12.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })))
        .width(Length::Fixed(280.0));

    // Create alignment container
    let alignment = if is_my_message { 
        iced::alignment::Horizontal::Right 
    } else { 
        iced::alignment::Horizontal::Left 
    };

    Container::new(bubble)
        .width(Length::Fill)
        .align_x(alignment)
        .into()
}

fn build_input_area<'a>(state: &'a ChatAppState, group_id: &'a str) -> Element<'a, Message> {
    // Create the TextInput and wrap it in a Container to reproduce the
    // desired background, border and radius without implementing a
    // custom `text_input::StyleSheet` trait. This keeps the style while
    // avoiding trait mismatch issues across iced versions.
    let raw_input = TextInput::new("Scrivi un messaggio al gruppo...", &state.current_message_input)
        .on_input(Message::MessageInputChanged)
        .on_submit(Message::SendGroupMessage { group_id: group_id.to_string() })
        .padding(12)
        .size(14)
        .width(Length::Fill);

    let message_input = Container::new(raw_input)
        .padding(0)
        .width(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(|_: &iced::Theme| {
            iced::widget::container::Appearance {
                background: Some(iced::Background::Color(INPUT_BG)),
                border: iced::Border {
                    radius: 20.0.into(),
                    width: 1.0,
                    color: Color::from_rgb(0.3, 0.3, 0.3),
                },
                ..Default::default()
            }
        })));

    let send_button = Button::new(Text::new("Invia").size(14))
        .on_press(Message::SendGroupMessage { group_id: group_id.to_string() })
        .style(iced::theme::Button::Primary)
        .padding([12, 16]);

    let input_row = Row::new()
        .spacing(8)
        .align_items(Alignment::Center)
        .push(message_input)
        .push(send_button);

    Container::new(input_row)
        .padding([12, 16])
        .width(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(|_: &iced::Theme| {
            iced::widget::container::Appearance {
                background: Some(iced::Background::Color(INPUT_BG)),
                border: iced::Border {
                    width: 1.0,
                    color: Color::from_rgb(0.2, 0.2, 0.2),
                    ..Default::default()
                },
                ..Default::default()
            }
        })))
        .into()
}