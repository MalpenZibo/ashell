use guido::prelude::*;

use crate::theme::ThemeColors;

use super::icons::{IconKind, StaticIcon, icon};

/// A slider component with drag-to-adjust, click-to-set, and scroll-to-adjust.
/// Renders: [mute icon] [track with fill bar + thumb] [optional chevron]
#[component]
pub struct Slider {
    #[prop]
    value: i32,
    #[prop]
    ic: StaticIcon,
    #[prop]
    muted: bool,
    #[prop(callback)]
    on_change: fn(i32),
    #[prop(callback)]
    on_mute_toggle: (),
    #[prop(callback)]
    on_chevron: (),
}

impl Slider {
    fn render(&self) -> impl Widget + use<> {
        let theme = expect_context::<ThemeColors>();
        let track_ref = create_widget_ref();
        let dragging = create_signal(false);

        let value = self.value.clone();
        let value2 = self.value.clone();
        let ic = self.ic.clone();
        let muted = self.muted.clone();
        let on_change_down = self.on_change.clone();
        let on_change_move = self.on_change.clone();
        let on_change_scroll = self.on_change.clone();
        let on_chevron = self.on_chevron.clone();

        let mut row = container()
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
                    .on_click_option(self.on_mute_toggle.clone())
                    .on_hover(move |h| mute_hovered.set(h))
                    .background(move || {
                        if mute_hovered.get() {
                            Color::rgba(1.0, 1.0, 1.0, 0.1)
                        } else {
                            Color::TRANSPARENT
                        }
                    })
                    .child(
                        icon().ic(move || IconKind::from(ic.get()))
                            .color(move || {
                                if muted.get() {
                                    Color::rgba(1.0, 1.0, 1.0, 0.4)
                                } else {
                                    theme.text
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
                        if let Some(ref on_change) = on_change_down {
                            let w = track_ref.rect().get().width;
                            if w > 0.0 {
                                let pct = (x / w * 100.0).clamp(0.0, 100.0).round() as i32;
                                on_change(pct);
                            }
                        }
                    })
                    .on_pointer_move(move |x, _y| {
                        if dragging.get() {
                            if let Some(ref on_change) = on_change_move {
                                let w = track_ref.rect().get().width;
                                if w > 0.0 {
                                    let pct = (x / w * 100.0).clamp(0.0, 100.0).round() as i32;
                                    on_change(pct);
                                }
                            }
                        }
                    })
                    .on_mouse_up(move |_x, _y| {
                        dragging.set(false);
                    })
                    .on_scroll(move |_dx, dy, _src| {
                        if let Some(ref on_change) = on_change_scroll {
                            let cur = value.get();
                            let step = if dy > 0.0 { -5 } else { 5 };
                            let new_val = (cur + step).clamp(0, 100);
                            on_change(new_val);
                        }
                    })
                    // Fill bar
                    .child(
                        container()
                            .height(fill())
                            .width(move || -> Length {
                                let v = value2.get();
                                let w = track_ref.rect().get().width;
                                let fill_w = (v as f32 / 100.0 * w - 6.0).max(0.0);
                                Length::from(fill_w)
                            })
                            .corner_radius(3.0)
                            .background(theme.primary),
                    )
                    // Thumb
                    .child(
                        container()
                            .width(4.0)
                            .height(4.0)
                            .corner_radius(4.0)
                            .scale(3.)
                            .background(theme.primary),
                    ),
            );

        // Optional chevron button
        if let Some(on_chev) = on_chevron {
            let chevron_hovered = create_signal(false);
            row = row.child(
                container()
                    .padding(4.0)
                    .corner_radius(4.0)
                    .on_click(move || on_chev())
                    .on_hover(move |h| chevron_hovered.set(h))
                    .background(move || {
                        if chevron_hovered.get() {
                            Color::rgba(1.0, 1.0, 1.0, 0.1)
                        } else {
                            Color::TRANSPARENT
                        }
                    })
                    .child(
                        icon().ic(StaticIcon::RightChevron)
                            .color(theme.text)
                            .font_size(14.0),
                    ),
            );
        }

        row
    }
}
