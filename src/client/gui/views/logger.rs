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
    // Show only the latest message as an alert bar (single message at a time)
    if let Some(log) = messages.iter().next_back() {
        let log_color = log.color();
        let bg_color = log_color;
        Container::new(
            Row::new()
                .spacing(12)
                .push(
                    Text::new(log.emoji())
                        .font(Font::with_name("Segoe UI Emoji"))
                        .size(20)
                        .style(iced::Color::WHITE)
                )
                .push(Text::new(&log.message).size(18).style(iced::Color::WHITE))
        )
        .padding([12, 16])
        .width(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(move |_: &iced::Theme| {
            iced::widget::container::Appearance {
                background: Some(iced::Background::Color(bg_color)),
                text_color: Some(iced::Color::WHITE),
                border: iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                shadow: iced::Shadow {
                    offset: iced::Vector::new(0.0, 4.0),
                    blur_radius: 12.0,
                    color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.3),
                },
            }
        })))
        .into()
    } else {
        // Empty placeholder
        iced::widget::Space::new(iced::Length::Fill, iced::Length::Fixed(0.0)).into()
    }
}
