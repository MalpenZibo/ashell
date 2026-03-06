use guido::prelude::*;

use crate::components::icon;
use crate::config::SettingsFormat;

/// A format-aware bar indicator: icon and/or text label.
///
/// The `format` prop controls what is shown:
/// - `Icon` → icon only
/// - `Percentage` / `Time` → label only
/// - `IconAndPercentage` / `IconAndTime` → icon + label
#[component]
pub fn bar_indicator(
    kind: super::IconKind,
    label: Option<String>,
    #[prop(default = "Color::WHITE")] color: Color,
    format: SettingsFormat,
) -> impl Widget {
    let show_icon = matches!(
        format.get(),
        SettingsFormat::Icon | SettingsFormat::IconAndPercentage | SettingsFormat::IconAndTime
    );
    let show_text = matches!(
        format.get(),
        SettingsFormat::Percentage
            | SettingsFormat::Time
            | SettingsFormat::IconAndPercentage
            | SettingsFormat::IconAndTime
    );

    container()
        .layout(
            Flex::row()
                .spacing(4)
                .cross_alignment(CrossAlignment::Center),
        )
        .maybe_child(show_icon.then(|| icon().kind(kind).color(color).font_size(14)))
        .maybe_child(show_text.then(|| {
            text(move || label.get().unwrap_or_default())
                .color(color)
                .font_size(14)
        }))
}
