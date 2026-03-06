use guido::prelude::*;

use crate::components::{StaticIcon, bar_indicator, slider};
use crate::config::SettingsFormat;
use crate::services::brightness::{BrightnessCmd, BrightnessDataSignals};
use crate::theme::ThemeColors;

/// Bar indicator: brightness icon and/or percentage
pub fn brightness_indicator(data: BrightnessDataSignals, format: SettingsFormat) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let current = data.current;
    let max = data.max;

    bar_indicator()
        .kind(StaticIcon::Brightness)
        .label(move || {
            let c = current.get() as f32;
            let m = max.get() as f32;
            let pct = if m > 0.0 { (c / m * 100.0).round() as i32 } else { 0 };
            Some(format!("{pct}%"))
        })
        .color(theme.text)
        .format(format)
}

pub fn slider_view(
    data: BrightnessDataSignals,
    svc: Service<BrightnessCmd>,
) -> impl Widget {
    let current = data.current;
    let max = data.max;

    // Derive percentage signal
    let pct = create_signal(0i32);
    create_effect(move || {
        let c = current.get() as f32;
        let m = max.get() as f32;
        let p = if m > 0.0 { (c / m * 100.0).round() as i32 } else { 0 };
        pct.set(p);
    })
    .detach();

    let svc_change = svc.clone();
    slider()
        .value(pct)
        .kind(StaticIcon::Brightness)
        .muted(false)
        .on_change(move |new_pct| {
            let m = max.get();
            let raw = (new_pct as f32 / 100.0 * m as f32).round() as u32;
            svc_change.send(BrightnessCmd::Set(raw));
        })
}
