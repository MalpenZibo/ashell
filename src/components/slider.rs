use guido::prelude::*;

use crate::theme;

use super::icons::{StaticIcon, icon};

/// A slider component with click-to-set and scroll-to-adjust.
/// Renders: [mute icon] [track with colored fill] [value text]
pub fn slider(
    value: Signal<i32>,
    ic: impl Fn() -> StaticIcon + 'static,
    muted: impl Fn() -> bool + 'static,
    on_change: impl Fn(i32) + 'static + Clone,
    on_mute_toggle: impl Fn() + 'static,
) -> impl Widget {
    let track_ref = create_widget_ref();
    // Track last pointer X relative to track for click-to-set
    let last_ptr_x = create_signal(0.0f32);

    let on_change_scroll = on_change.clone();
    let on_change_click = on_change.clone();

    container()
        .width(fill())
        .layout(
            Flex::row()
                .spacing(8.0)
                .cross_axis_alignment(CrossAxisAlignment::Center),
        )
        // Mute icon
        .child({
            let mute_hovered = create_signal(false);
            container()
                .padding(4.0)
                .corner_radius(4.0)
                .on_click(move || on_mute_toggle())
                .on_hover(move |h| mute_hovered.set(h))
                .background(move || {
                    if mute_hovered.get() {
                        Color::rgba(1.0, 1.0, 1.0, 0.1)
                    } else {
                        Color::TRANSPARENT
                    }
                })
                .child(
                    icon(move || ic())
                        .color(move || {
                            if muted() {
                                Color::rgba(1.0, 1.0, 1.0, 0.4)
                            } else {
                                theme::TEXT
                            }
                        })
                        .font_size(16.0),
                )
        })
        // Track
        .child(
            container()
                .widget_ref(track_ref)
                .width(fill())
                .height(8.0)
                .corner_radius(4.0)
                .background(Color::rgba(1.0, 1.0, 1.0, 0.15))
                .on_pointer_move(move |x, _y| {
                    last_ptr_x.set(x);
                })
                .on_click(move || {
                    let rect = track_ref.rect().get();
                    let x = last_ptr_x.get();
                    if rect.width > 0.0 {
                        let pct = ((x / rect.width) * 100.0).round() as i32;
                        let clamped = pct.clamp(0, 100);
                        on_change_click(clamped);
                    }
                })
                .on_scroll(move |_dx, dy, _src| {
                    let cur = value.get();
                    let step = if dy > 0.0 { -5 } else { 5 };
                    let new_val = (cur + step).clamp(0, 100);
                    on_change_scroll(new_val);
                })
                // Fill bar
                .child(
                    container()
                        .height(fill())
                        .width(move || -> Length {
                            let v = value.get();
                            let w = (v as f32 / 100.0 * track_ref.rect().get().width).max(0.0);
                            Length::from(w)
                        })
                        .corner_radius(4.0)
                        .background(theme::LAVENDER),
                ),
        )
        // Value text
        .child(
            container().width(32.0).child(
                text(move || format!("{}%", value.get()))
                    .color(theme::TEXT)
                    .font_size(12.0),
            ),
        )
}
