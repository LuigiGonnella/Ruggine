use iced::{Element, Length, Alignment, Color};
use iced::widget::{Column, Row, Text, Button, Container, TextInput, Scrollable, Space};
use crate::client::models::messages::Message;
use crate::client::models::app_state::ChatAppState;

// Reuse styling constants from main_actions (duplicating a few for simplicity)
const BG_MAIN: Color = Color::from_rgb(0.06, 0.07, 0.18);
const TEXT_PRIMARY: Color = Color::WHITE;

pub fn view<'a>(state: &'a ChatAppState, kind: &'a str) -> Element<'a, Message> {
    // Bound search input to state.users_search_query
    let search_input = TextInput::new("Search username...", &state.users_search_query)
        .on_input(Message::UsersSearchQueryChanged)
        .padding(8)
        .size(16)
        .width(Length::Fill);

    let search_btn = Button::new(Text::new("Search")).on_press(Message::UsersSearch).padding(8);

    let back_btn = Button::new(Text::new("← Back").size(16)).on_press(Message::OpenMainActions).padding(6);

    let header = Row::new()
        .spacing(12)
        .align_items(Alignment::Center)
        .push(back_btn)
        .push(Text::new(format!("{} Users", kind)).size(22).style(TEXT_PRIMARY));

    // Build results list from state.users_search_results
    let mut list_col = Column::new().spacing(8);
    if state.users_search_results.is_empty() {
        list_col = list_col.push(Text::new("No results").style(TEXT_PRIMARY));
    } else {
        for username in state.users_search_results.iter() {
            let row = Row::new()
                .spacing(12)
                .align_items(Alignment::Center)
                .push(Text::new(username).style(TEXT_PRIMARY))
                .push(Space::new(Length::Fill, Length::Fixed(0.0)))
                .push(Button::new(Text::new("Send Message")).on_press(Message::OpenPrivateChat(username.clone())).padding(6));
            list_col = list_col.push(row);
        }
    }

    let content = Column::new()
        .push(header)
        .push(Space::new(Length::Fixed(0.0), Length::Fixed(8.0)))
        .push(Row::new().push(search_input).push(Space::new(Length::Fixed(8.0), Length::Fixed(0.0))).push(search_btn))
        .push(Space::new(Length::Fixed(0.0), Length::Fixed(12.0)))
        .push(Scrollable::new(list_col).height(Length::Fill));

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .style(iced::theme::Container::Custom(Box::new(|_: &iced::Theme| iced::widget::container::Appearance { background: Some(iced::Background::Color(BG_MAIN)), ..Default::default() })))
        .into()
}
