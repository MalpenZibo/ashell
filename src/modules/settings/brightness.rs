use guido::prelude::*;

use crate::components::{StaticIcon, bar_indicator, slider};
use crate::config::SettingsFormat;
use crate::services::brightness::{BrightnessCommand, BrightnessService};
use crate::services::compat::ServiceSignal;
use crate::theme::ThemeColors;

fn percent(service: &BrightnessService) -> i32 {
    let current = service.current.value() as f32;
    let max = service.max as f32;
    if max > 0.0 {
        (current / max * 100.0).round() as i32
    } else {
        0
    }
}

/// Bar indicator: brightness icon and/or percentage
pub fn brightness_indicator(
    data: ServiceSignal<BrightnessService>,
    format: SettingsFormat,
) -> impl Widget {
    let theme = expect_context::<ThemeColors>();

    bar_indicator()
        .kind(StaticIcon::Brightness)
        .label(move || data.with(|s| s.as_ref().map(|s| format!("{}%", percent(s)))))
        .color(theme.text)
        .format(format)
}

pub fn slider_view(
    data: ServiceSignal<BrightnessService>,
    svc: Service<BrightnessCommand>,
) -> impl Widget {
    // Derive percentage signal
    let pct = create_signal(0i32);
    create_effect(move || {
        pct.set(data.with(|s| s.as_ref().map(percent).unwrap_or(0)));
    })
    .detach();

    slider()
        .value(pct)
        .kind(StaticIcon::Brightness)
        .muted(false)
        .on_change(move |new_pct| {
            let max = data.with(|s| s.as_ref().map(|s| s.max).unwrap_or(0));
            let raw = (new_pct as f32 / 100.0 * max as f32).round() as u32;
            svc.send(BrightnessCommand(raw));
        })
}
