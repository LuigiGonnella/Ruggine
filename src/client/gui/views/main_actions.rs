
use iced::{Element, Length, Alignment, Color};
use iced::widget::{Column, Row, Text, Button, Container, Space};
use crate::client::models::messages::Message;
use crate::client::models::app_state::ChatAppState;

pub fn view(state: &ChatAppState) -> Element<Message> {
    let welcome = state.welcome_message.clone().unwrap_or_else(|| "Benvenuto!".to_string());
    let logout_button = Button::new(Text::new("Logout")).on_press(Message::Logout);
    let log_area = Column::with_children(
        state.log_messages.iter().rev().take(10).map(|(msg, color)| {
            Text::new(msg).style(iced::theme::Text::Color(*color)).into()
        }).collect::<Vec<_>>()
    ).spacing(2);

    let content = Column::new()
        .push(Text::new(welcome).size(32).style(Color::from_rgb(0.0, 0.7, 0.0)))
        .push(Space::new(Length::Fill, Length::Fixed(10.0)))
        .push(Row::new()
            .push(logout_button)
            .push(Space::new(Length::Fixed(20.0), Length::Fill))
            .push(Text::new("[Placeholder] Chat, Amici, ecc.").style(Color::from_rgb(0.3, 0.3, 0.3)))
        )
        .push(Space::new(Length::Fill, Length::Fixed(20.0)))
        .push(Text::new("Log eventi:").size(18))
        .push(log_area)
        .align_items(Alignment::Center)
        .spacing(10);

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .into()
}
