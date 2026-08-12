use guido::prelude::*;

use crate::{
    components::{buttons::icon_button, icon},
    theme::ThemeColors,
};

use super::icons::{IconKind, StaticIcon};

/// A slider component with drag-to-adjust, click-to-set, and scroll-to-adjust.
/// Renders: [mute icon] [track with fill bar + thumb] [optional chevron]
#[component]
pub fn slider(
    value: i32,
    kind: IconKind,
    muted: bool,
    #[prop(callback)] on_change: fn(i32),
    #[prop(callback)] on_mute_toggle: (),
    #[prop(callback)] on_chevron: (),
    #[prop(default = "false")] expanded: bool,
) -> impl Widget {
    let _ = muted; // TODO: use muted state for visual feedback
    let theme = expect_context::<ThemeColors>();
    let track_ref = create_widget_ref();
    let dragging = create_signal(false);

    let mut row = container()
        .width(fill())
        .layout(
            Flex::row()
                .spacing(8)
                .cross_alignment(CrossAlignment::Center),
        )
        // Mute icon
        .child({
            if let Some(on_mute) = on_mute_toggle {
                icon_button()
                    .icon(move || kind.get())
                    .on_click(move || on_mute.run())
                    .into_any()
            } else {
                container()
                    .layout(Flex::column().cross_alignment(CrossAlignment::Center))
                    .width(at_least(32))
                    .child(icon().kind(kind.get()))
                    .into_any()
            }
        })
        // Track with fill bar + thumb
        .child(
            container()
                .widget_ref(track_ref)
                .width(fill())
                .height(4)
                .corner_radius(3)
                .background(Color::rgba(1.0, 1.0, 1.0, 0.15))
                .layout(Flex::row().cross_alignment(CrossAlignment::Center))
                .on_mouse_down(move |x, _y| {
                    dragging.set(true);
                    if let Some(on_change) = on_change {
                        let w = track_ref.rect().get().width;
                        if w > 0.0 {
                            let pct = (x / w * 100.0).clamp(0.0, 100.0).round() as i32;
                            on_change.run(pct);
                        }
                    }
                })
                .on_pointer_move(move |x, _y| {
                    if dragging.get()
                        && let Some(on_change) = on_change
                    {
                        let w = track_ref.rect().get().width;
                        if w > 0.0 {
                            let pct = (x / w * 100.0).clamp(0.0, 100.0).round() as i32;
                            on_change.run(pct);
                        }
                    }
                })
                .on_mouse_up(move |_x, _y| {
                    dragging.set(false);
                })
                .on_scroll(move |_dx, dy, _src| {
                    if let Some(on_change) = on_change {
                        let cur = value.get();
                        let step = if dy > 0.0 { -5 } else { 5 };
                        let new_val = (cur + step).clamp(0, 100);
                        on_change.run(new_val);
                    }
                })
                // Thumb clearance: the fill resolves against the inner width
                .padding([0, 6, 0, 0])
                // Fill bar: fraction of the track, correct on the very first
                // frame — a measured-rect width lagged one layout behind and
                // flashed to zero whenever the menu content rebuilt
                .child(
                    container()
                        .height(fill())
                        .width(move || fraction((value.get() as f32 / 100.0).clamp(0.0, 1.0)))
                        .corner_radius(3)
                        .background(theme.primary),
                )
                // Thumb
                .child(
                    container()
                        .width(4)
                        .height(4)
                        .corner_radius(4)
                        .scale(3)
                        .background(theme.primary),
                ),
        );

    // Optional chevron button (shows Close when expanded, RightArrow otherwise)
    if let Some(on_chev) = on_chevron {
        row = row.child(
            icon_button()
                .icon(move || -> IconKind {
                    if expanded.get() {
                        StaticIcon::Close
                    } else {
                        StaticIcon::RightArrow
                    }
                    .into()
                })
                .on_click(move || on_chev.run()),
        );
    }

    row
}
