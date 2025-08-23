
use iced::{Element, Length, Alignment, Color};
use iced::widget::{Column, Row, Text, Button, Container, Space};
use crate::client::models::messages::Message;
use crate::client::models::app_state::ChatAppState;

pub fn view(state: &ChatAppState) -> Element<Message> {
    let logout_button = Button::new(Text::new("Logout")).on_press(Message::Logout);

    let content = Column::new()
        // Top logger bar
        .push(crate::client::gui::views::logger::logger_view(&state.logger))
        .push(Space::new(Length::Fill, Length::Fixed(10.0)))
        .push(Row::new()
            .push(logout_button)
            .push(Space::new(Length::Fixed(20.0), Length::Fill))
            .push(Text::new("[Placeholder] Chat, Amici, ecc.").style(Color::from_rgb(0.3, 0.3, 0.3)))
        )
        .align_items(Alignment::Center)
        .spacing(10);

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .into()
}
