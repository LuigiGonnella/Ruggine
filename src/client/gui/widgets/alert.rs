// Widget di alert per la GUI
use iced::{Element, widget::text};
use crate::client::models::messages::Message;

pub fn view(msg: &str) -> Element<'_, Message> {
    text(format!("ALERT: {}", msg)).into()
}
