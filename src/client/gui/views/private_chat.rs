use iced::{Element, widget::{column, text}};
use crate::client::models::messages::Message;
use crate::client::models::app_state::ChatAppState;

pub fn view<'a>(_state: &'a ChatAppState, username: &'a str) -> Element<'a, Message> {
    column![
        text(format!("Private Chat View (modulare) with {}", username))
    ].into()
}
