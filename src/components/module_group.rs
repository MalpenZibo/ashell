use crate::{config::BarBackground, theme::AshellTheme};
use iced::{Border, Color, Element, widget::container};

/// Wraps content with the appropriate bar background container.
///
/// - When the bar has a background → pass through as-is (modules blend into the bar)
/// - When the bar has no background → wrap in a container with background color + rounded border
///   (islands look)
pub fn module_group<'a, Msg: 'static>(
    theme: &'a AshellTheme,
    content: Element<'a, Msg>,
) -> Element<'a, Msg> {
    if theme.background == BarBackground::Transparent {
        container(content)
            .style(move |iced_theme: &iced::Theme| container::Style {
                background: Some(
                    iced_theme
                        .palette()
                        .background
                        .scale_alpha(theme.opacity)
                        .into(),
                ),
                border: Border {
                    width: 0.0,
                    radius: theme.radius.lg.into(),
                    color: Color::TRANSPARENT,
                },
                ..container::Style::default()
            })
            .into()
    } else {
        content
    }
}
