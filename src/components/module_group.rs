use crate::{config::AppearanceStyle, theme::AshellTheme};
use iced::{Border, Color, Element, widget::container};

/// Wraps content with the appropriate bar style container.
///
/// - `Solid | Gradient` → pass through as-is
/// - `Islands` → wrap in a container with background color + rounded border
pub fn module_group<'a, Msg: 'static>(
    theme: &'a AshellTheme,
    content: Element<'a, Msg>,
) -> Element<'a, Msg> {
    match theme.bar_style {
        AppearanceStyle::Solid | AppearanceStyle::Gradient => content,
        AppearanceStyle::Islands => container(content)
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
            .into(),
    }
}
