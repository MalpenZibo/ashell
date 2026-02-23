use guido::prelude::*;

use crate::components::{StaticIcon, slider};
use crate::services::brightness::{BrightnessCmd, BrightnessDataSignals};

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
    slider(
        pct,
        || StaticIcon::Brightness,
        || false, // brightness can't be muted
        move |new_pct| {
            let m = max.get();
            let raw = (new_pct as f32 / 100.0 * m as f32).round() as u32;
            svc_change.send(BrightnessCmd::Set(raw));
        },
        || {}, // no mute toggle for brightness
        None::<fn()>,
    )
}
