use guido::prelude::*;

use crate::theme;

use super::icons::{StaticIcon, icon};

/// A slider component with drag-to-adjust, click-to-set, and scroll-to-adjust.
/// Renders: [mute icon] [track with fill bar + thumb] [optional chevron]
pub fn slider(
    value: Signal<i32>,
    ic: impl Fn() -> StaticIcon + 'static,
    muted: impl Fn() -> bool + 'static,
    on_change: impl Fn(i32) + 'static + Clone,
    on_mute_toggle: impl Fn() + 'static,
    on_chevron: Option<impl Fn() + 'static>,
) -> impl Widget {
    let track_ref = create_widget_ref();
    let dragging = create_signal(false);

    let on_change_down = on_change.clone();
    let on_change_move = on_change.clone();
    let on_change_scroll = on_change.clone();

    let row = container()
        .width(fill())
        .layout(
            Flex::row()
                .spacing(8.0)
                .cross_alignment(CrossAlignment::Center),
        )
        // Mute icon
        .child({
            let mute_hovered = create_signal(false);
            container()
                .padding(4.0)
                .corner_radius(4.0)
                .on_click(on_mute_toggle)
                .on_hover(move |h| mute_hovered.set(h))
                .background(move || {
                    if mute_hovered.get() {
                        Color::rgba(1.0, 1.0, 1.0, 0.1)
                    } else {
                        Color::TRANSPARENT
                    }
                })
                .child(
                    icon(ic)
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
        // Track with fill bar + thumb
        .child(
            container()
                .widget_ref(track_ref)
                .width(fill())
                .height(4.0)
                .corner_radius(3.0)
                .background(Color::rgba(1.0, 1.0, 1.0, 0.15))
                .layout(Flex::row().cross_alignment(CrossAlignment::Center))
                .on_mouse_down(move |x, _y| {
                    dragging.set(true);
                    let w = track_ref.rect().get().width;
                    if w > 0.0 {
                        let pct = (x / w * 100.0).clamp(0.0, 100.0).round() as i32;
                        on_change_down(pct);
                    }
                })
                .on_pointer_move(move |x, _y| {
                    if dragging.get() {
                        let w = track_ref.rect().get().width;
                        if w > 0.0 {
                            let pct = (x / w * 100.0).clamp(0.0, 100.0).round() as i32;
                            on_change_move(pct);
                        }
                    }
                })
                .on_mouse_up(move |_x, _y| {
                    dragging.set(false);
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
                            let w = track_ref.rect().get().width;
                            let fill_w = (v as f32 / 100.0 * w - 6.0).max(0.0);
                            Length::from(fill_w)
                        })
                        .corner_radius(3.0)
                        .background(theme::LAVENDER),
                )
                // Thumb
                .child(
                    container()
                        .width(4.0)
                        .height(4.0)
                        .corner_radius(4.0)
                        .scale(3.)
                        .background(theme::LAVENDER),
                ),
        );

    // Optional chevron button
    if let Some(on_chev) = on_chevron {
        let chevron_hovered = create_signal(false);
        row.child(
            container()
                .padding(4.0)
                .corner_radius(4.0)
                .on_click(on_chev)
                .on_hover(move |h| chevron_hovered.set(h))
                .background(move || {
                    if chevron_hovered.get() {
                        Color::rgba(1.0, 1.0, 1.0, 0.1)
                    } else {
                        Color::TRANSPARENT
                    }
                })
                .child(
                    icon(StaticIcon::RightChevron)
                        .color(theme::TEXT)
                        .font_size(14.0),
                ),
        )
    } else {
        row
    }
}
