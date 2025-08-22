// Widget di alert per la GUI
use iced::{Element, widget::text};
use crate::client::models::messages::Message;

pub fn view<'a>(msg: &'a str) -> Element<'a, Message> {
    text(format!("ALERT: {}", msg)).into()
}
