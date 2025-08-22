use iced::{Element, widget::{column, text}};
use crate::client::models::messages::Message;
use crate::client::models::app_state::ChatAppState;

pub fn view(_state: &ChatAppState) -> Element<Message> {
    column![
        text("Friend Requests View (modulare)")
    ].into()
}
