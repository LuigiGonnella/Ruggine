use iced::Application;
fn main() -> iced::Result {
    // load environment from .env (optional)
    let _ = dotenvy::dotenv();
    ruggine_modulare::client::gui::app::ChatApp::run(iced::Settings::default())
}
