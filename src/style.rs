use iced::{widget::button, BorderRadius};

pub struct HeaderButtonStyle;

impl button::StyleSheet for HeaderButtonStyle {
    type Style = iced::theme::Theme;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                0.0, 0.0, 0.0,
            ))),
            text_color: iced::Color::from_rgb(1.0, 1.0, 1.0),
            border_radius: BorderRadius::from(12.0),
            border_width: 0.0,
            border_color: iced::Color::TRANSPARENT,
            ..button::Appearance::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                0.2, 0.2, 0.2,
            ))),
            text_color: iced::Color::from_rgb(1.0, 1.0, 1.0),
            border_radius: BorderRadius::from(12.0),
            border_width: 0.0,
            border_color: iced::Color::TRANSPARENT,
            ..button::Appearance::default()
        }
    }

    fn focused(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                0.2, 0.2, 0.2,
            ))),
            text_color: iced::Color::from_rgb(1.0, 1.0, 1.0),
            border_radius: BorderRadius::from(12.0),
            border_width: 0.0,
            border_color: iced::Color::TRANSPARENT,
            ..button::Appearance::default()
        }
    }

    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                0.2, 0.2, 0.2,
            ))),
            text_color: iced::Color::from_rgb(1.0, 1.0, 1.0),
            border_radius: BorderRadius::from(12.0),
            border_width: 0.0,
            border_color: iced::Color::TRANSPARENT,
            ..button::Appearance::default()
        }
    }

    fn disabled(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(iced::Background::Color(iced::Color::from_rgba(
                0.0, 0.0, 0.0, 0.8,
            ))),
            text_color: iced::Color::from_rgb(1.0, 1.0, 1.0),
            border_radius: BorderRadius::from(12.0),
            border_width: 0.0,
            border_color: iced::Color::TRANSPARENT,
            ..button::Appearance::default()
        }
    }
}
