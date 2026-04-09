use crate::{config::SettingsFormat, theme::AshellTheme, utils::IndicatorState};
use iced::{Alignment, Element, Theme, widget::container, widget::row};

/// Produces a format-aware indicator element based on `SettingsFormat`.
///
/// - `Icon` → just the icon element
/// - `Percentage | Time` → just `text(label)`
/// - `IconAndPercentage | IconAndTime` → `row!(icon, text).spacing(xxs).align_y(Center)`
///
/// When `state` is not `Normal`, wraps the result in a container with
/// the appropriate text color (success, warning, danger).
pub fn format_indicator<'a, Msg: 'static>(
    theme: &AshellTheme,
    format: SettingsFormat,
    icon_element: Element<'a, Msg>,
    label_element: Element<'a, Msg>,
    state: IndicatorState,
) -> Element<'a, Msg> {
    let content = match format {
        SettingsFormat::Icon => icon_element,
        SettingsFormat::Percentage | SettingsFormat::Time => label_element,
        SettingsFormat::IconAndPercentage | SettingsFormat::IconAndTime => {
            row![icon_element, label_element]
                .spacing(theme.space.xxs)
                .align_y(Alignment::Center)
                .into()
        }
    };

    match state {
        IndicatorState::Normal => content,
        _ => container(content)
            .style(move |theme: &Theme| container::Style {
                text_color: Some(match state {
                    IndicatorState::Success => theme.palette().success,
                    IndicatorState::Warning => theme.extended_palette().danger.weak.color,
                    IndicatorState::Danger => theme.palette().danger,
                    IndicatorState::Normal => unreachable!(),
                }),
                ..Default::default()
            })
            .into(),
    }
}
