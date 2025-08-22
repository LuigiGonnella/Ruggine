// Widget per la sezione di input
use iced::{Element, widget::{column, text, text_input, button}};
use crate::client::models::messages::Message;

pub fn view<'a>(input: &'a str) -> Element<'a, Message> {
    column![
        text("Scrivi un messaggio:"),
    text_input("Messaggio", input),
        button("Invia").on_press(Message::None)
    ].into()
}
