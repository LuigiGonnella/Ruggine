// Widget per la lista dei messaggi
use iced::{Element, widget::column};
use crate::client::models::messages::Message;

pub fn view<'a>(messages: &[String]) -> Element<'a, Message> {
    let items = messages.iter().map(|msg| iced::widget::text(msg).into()).collect::<Vec<_>>();
    column(items).into()
}
