use iced::Application;
fn main() -> iced::Result {
    ruggine_modulare::client::gui::app::ChatApp::run(iced::Settings::default())
}
