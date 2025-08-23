
use iced::{Element, Length, Alignment, Color};
use iced::widget::{Column, Row, Text, Button, Container, Space};
use crate::client::models::messages::Message;
use crate::client::models::app_state::ChatAppState;

pub fn view(state: &ChatAppState) -> Element<Message> {
    use iced::widget::Column;
    let logout_button = Button::new(Text::new("Logout")).on_press(Message::Logout);

    // Messaging buttons
    let btn_friends = Button::new(Text::new("Amici / Richieste")).on_press(Message::OpenFriendRequests);
    let btn_private = Button::new(Text::new("Chat Privata (demo) ")).on_press(Message::OpenPrivateChat("alice".to_string()));
    let btn_group = Button::new(Text::new("Chat Gruppo (demo) ")).on_press(Message::OpenGroupChat("group-id-demo".to_string(), "GruppoDemo".to_string()));

    // Quick test actions that trigger network calls (useful while developing)
    let btn_send_group = Button::new(Text::new("Invia gruppo (test)"))
        .on_press(Message::SendGroupMessageTest);
    let btn_get_group = Button::new(Text::new("Leggi gruppo (test)"))
        .on_press(Message::GetGroupMessagesTest);
    let btn_delete_group = Button::new(Text::new("Cancella gruppo (test)"))
        .on_press(Message::DeleteGroupMessagesTest);

    let btn_send_private = Button::new(Text::new("Invia privato (test)"))
        .on_press(Message::SendPrivateMessageTest);
    let btn_get_private = Button::new(Text::new("Leggi privato (test)"))
        .on_press(Message::GetPrivateMessagesTest);
    let btn_delete_private = Button::new(Text::new("Cancella privato (test)"))
        .on_press(Message::DeletePrivateMessagesTest);

    let content = Column::new()
        .push(crate::client::gui::views::logger::logger_view(&state.logger))
        .push(Space::new(Length::Fill, Length::Fixed(8.0)))
        .push(Row::new().spacing(10).push(logout_button))
        .push(Space::new(Length::Fill, Length::Fixed(6.0)))
        .push(Text::new("Messaging").size(24))
        .push(Space::new(Length::Fill, Length::Fixed(6.0)))
        .push(Row::new().spacing(10).push(btn_friends).push(btn_private).push(btn_group))
        .push(Space::new(Length::Fill, Length::Fixed(6.0)))
        .push(Row::new().spacing(10).push(btn_send_group).push(btn_get_group).push(btn_delete_group))
        .push(Space::new(Length::Fill, Length::Fixed(6.0)))
        .push(Row::new().spacing(10).push(btn_send_private).push(btn_get_private).push(btn_delete_private))
        .align_items(Alignment::Center)
        .spacing(8);

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .into()
}
