use iced::{Element, widget::{column, text}};
use crate::client::models::messages::Message;
use crate::client::models::app_state::ChatAppState;

pub fn view<'a>(_state: &'a ChatAppState, group_id: &'a str, group_name: &'a str) -> Element<'a, Message> {
    column![
        text(format!("Group Chat View (modulare) for {} ({})", group_name, group_id))
    ].into()
}
