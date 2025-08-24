use iced::{Element, Length, Alignment, Color, widget::{column, text, scrollable, text_input, button, row, container, Column}};
use crate::client::models::messages::Message;
use crate::client::models::app_state::{ChatAppState, PrivateMessage};

const BG_MAIN: Color = Color::from_rgb(0.06, 0.07, 0.18);
const SENT_MESSAGE_BG: Color = Color::from_rgb(0.2, 0.4, 0.8); // Blu per messaggi inviati
const RECEIVED_MESSAGE_BG: Color = Color::from_rgb(0.3, 0.3, 0.3); // Grigio per messaggi ricevuti
const TEXT_PRIMARY: Color = Color::WHITE;

pub fn view<'a>(state: &'a ChatAppState, chat_with: &'a str) -> Element<'a, Message> {
    let mut content = column![];
    
    // Header with back button and chat title
    let header = row![
        button("← Indietro")
            .on_press(Message::OpenMainActions)
            .padding(8),
        text(format!("Chat con {}", chat_with))
            .size(20)
            .style(TEXT_PRIMARY)
    ].spacing(10).align_items(Alignment::Center);
    
    content = content.push(header);
    
    // Messages area
    let mut messages_column = Column::new().spacing(10);
    
    for msg in &state.private_messages {
        let message_bubble = create_message_bubble(msg);
        messages_column = messages_column.push(message_bubble);
    }
    
    let messages_scroll = scrollable(messages_column)
        .height(Length::Fill)
        .width(Length::Fill);
    
    content = content.push(
        container(messages_scroll)
            .height(Length::Fill)
            .padding(10)
    );
    
    // Input area
    let input_row = row![
        text_input("Scrivi un messaggio...", &state.private_message_input)
            .on_input(Message::PrivateMessageChanged)
            .width(Length::Fill)
            .padding(10),
        button("Invia")
            .on_press(Message::SendPrivateMessage(chat_with.to_string()))
            .padding(10)
    ].spacing(10);
    
    content = content.push(input_row);
    
    container(content)
        .padding(20)
        .height(Length::Fill)
        .width(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(|_: &iced::Theme| {
            iced::widget::container::Appearance {
                background: Some(iced::Background::Color(BG_MAIN)),
                ..Default::default()
            }
        })))
        .into()
}

fn create_message_bubble<'a>(msg: &'a PrivateMessage) -> Element<'a, Message> {
    let is_sent_by_me = msg.is_sent_by_me;
    
    // Create message content with timestamp
    let message_text = column![
        text(&msg.content)
            .style(TEXT_PRIMARY)
            .size(14),
        text(&msg.timestamp)
            .style(Color::from_rgb(0.7, 0.7, 0.7))
            .size(11)
    ].spacing(2);
    
    let message_content = container(message_text)
        .padding(10)
        .style(iced::theme::Container::Custom(Box::new(move |_: &iced::Theme| {
            iced::widget::container::Appearance {
                background: Some(iced::Background::Color(
                    if is_sent_by_me { SENT_MESSAGE_BG } else { RECEIVED_MESSAGE_BG }
                )),
                border: iced::Border {
                    radius: 10.0.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                ..Default::default()
            }
        })));
    
    if msg.is_sent_by_me {
        // Messages sent by me: align to the right
        row![
            container("").width(Length::Fill), // Spacer
            message_content
        ].into()
    } else {
        // Messages received: align to the left
        row![
            message_content,
            container("").width(Length::Fill) // Spacer
        ].into()
    }
}
