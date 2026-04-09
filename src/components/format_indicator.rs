use crate::{config::SettingsFormat, theme::AshellTheme};
use iced::{Alignment, Element, widget::row};

/// Produces a format-aware indicator element based on `SettingsFormat`.
///
/// - `Icon` → just the icon element
/// - `Percentage | Time` → just `text(label)`
/// - `IconAndPercentage | IconAndTime` → `row!(icon, text).spacing(xxs).align_y(Center)`
///
/// Callers are responsible for computing the appropriate icon and label,
/// and for wrapping the result in `MouseArea`/`container` for interaction.
pub fn format_indicator<'a, Msg: 'static>(
    theme: &AshellTheme,
    format: SettingsFormat,
    icon_element: Element<'a, Msg>,
    label_element: Element<'a, Msg>,
) -> Element<'a, Msg> {
    match format {
        SettingsFormat::Icon => icon_element,
        SettingsFormat::Percentage | SettingsFormat::Time => label_element,
        SettingsFormat::IconAndPercentage | SettingsFormat::IconAndTime => {
            row![icon_element, label_element]
                .spacing(theme.space.xxs)
                .align_y(Alignment::Center)
                .into()
        }
    }
}
