use hex_color::HexColor;
use iced::{
    Background, Border, Color, Element, Length, Theme,
    widget::{column, container, text},
};
use std::str::FromStr;

use crate::theme::AshellTheme;

pub fn event_card<'a, Message: 'a>(
    theme: &'a AshellTheme,
    title: impl Into<String>,
    time_range: impl Into<String>,
    color: Option<String>,
    opacity: f32,
    past: bool,
) -> Element<'a, Message> {
    let background = color
        .as_deref()
        .and_then(|color| HexColor::from_str(color).ok())
        .map(|color| Color::from_rgb8(color.r, color.g, color.b))
        .unwrap_or_else(|| theme.iced_theme.extended_palette().background.weak.color);
    let card_opacity = if past { opacity * 0.35 } else { opacity };

    container(
        column!(
            text(title.into()).size(theme.font_size.sm),
            text(time_range.into()).size(theme.font_size.xs),
        )
        .spacing(theme.space.xxs),
    )
    .padding(theme.space.sm)
    .width(Length::Fill)
    .style(move |_theme: &Theme| iced::widget::container::Style {
        background: Background::Color(background.scale_alpha(card_opacity)).into(),
        border: Border::default().rounded(theme.radius.sm),
        ..Default::default()
    })
    .into()
}
