// Stile custom per container colorato
pub struct MyLogStyle {
    pub color: iced::Color,
}
impl iced::widget::container::StyleSheet for MyLogStyle {
    type Style = (); // nessun custom style
    fn appearance(&self, _style: &Self::Style) -> Appearance {
        Appearance {
            background: Some(self.color.into()),
            text_color: Some(iced::Color::WHITE),
            ..Default::default()
        }
    }
}
use iced::{Element, Font, Length};
use iced::widget::{Row, Text};
use iced::widget::Container;
use iced::widget::container::Appearance;

#[derive(Debug, Clone)]
pub enum LogLevel {
    Success,
    Error,
    Info,
    Warning,
}

#[derive(Debug, Clone)]
pub struct LogMessage {
    pub level: LogLevel,
    pub message: String,
}

impl LogMessage {
    pub fn emoji(&self) -> &'static str {
        match self.level {
            LogLevel::Success => "✅",
            LogLevel::Error => "❌",
            LogLevel::Info => "ℹ️",
            LogLevel::Warning => "⚠️",
        }
    }
    pub fn color(&self) -> iced::Color {
        match self.level {
            LogLevel::Success => iced::Color::from_rgb(0.2, 0.8, 0.4),
            LogLevel::Error => iced::Color::from_rgb(1.0, 0.2, 0.2),
            LogLevel::Info => iced::Color::from_rgb(0.2, 0.6, 1.0),
            LogLevel::Warning => iced::Color::from_rgb(1.0, 0.8, 0.0),
        }
    }
}

pub fn logger_view(messages: &[LogMessage]) -> Element<'_, crate::client::models::messages::Message> {
    let mut col = iced::widget::Column::new().spacing(8);
    for log in messages.iter().rev().take(3) {
        col = col.push(
            Container::new(
                Row::new()
                    .spacing(8)
                    .push(
                        Text::new(log.emoji())
                            .font(Font::with_name("Segoe UI Emoji"))
                            .size(20)
                            .style(log.color())
                    )
                    .push(Text::new(&log.message).size(16))
            )
            .padding([6, 12])
            .width(Length::Fill)
            .style(iced::theme::Container::Box)
        );
    }
    col.into()
}
