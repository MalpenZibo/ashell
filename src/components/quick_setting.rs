use guido::prelude::*;

use crate::theme::ThemeColors;

use super::icons::{StaticIcon, icon};

/// A quick-setting button: icon + title + optional subtitle + optional chevron.
/// Active state shows colored background, inactive shows dim.
/// Chevron is inside the button (unified tile) matching ashell layout.
pub fn quick_setting(
    ic: impl Fn() -> StaticIcon + 'static + Clone,
    title: impl Fn() -> String + 'static + Clone,
    subtitle: impl Fn() -> String + 'static + Clone,
    active: impl Fn() -> bool + 'static + Clone,
    on_toggle: impl Fn() + 'static,
    on_submenu: Option<impl Fn() + 'static>,
) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let hovered = create_signal(false);
    let active2 = active.clone();
    let active3 = active.clone();
    let active4 = active.clone();
    let active5 = active.clone();

    let mut btn = container()
        .width(fill())
        .height(50.0)
        .corner_radius(16.0)
        .on_hover(move |h| hovered.set(h))
        .on_click(move || on_toggle())
        .background(move || {
            if active() {
                theme.primary
            } else if hovered.get() {
                Color::rgba(1.0, 1.0, 1.0, 0.1)
            } else {
                Color::rgba(1.0, 1.0, 1.0, 0.05)
            }
        })
        .layout(
            Flex::row()
                .spacing(8.0)
                .cross_alignment(CrossAlignment::Center),
        )
        .padding([0.0, 10.0])
        // Icon
        .child(
            icon(move || ic())
                .color(move || {
                    if active2() {
                        theme.background
                    } else {
                        theme.text
                    }
                })
                .font_size(16.0),
        )
        // Text column
        .child(
            container()
                .layout(Flex::column().spacing(1.0))
                .child(
                    text(move || title())
                        .color(move || {
                            if active3() {
                                theme.background
                            } else {
                                theme.text
                            }
                        })
                        .font_size(12.0),
                )
                .child({
                    let subtitle = subtitle.clone();
                    let active4 = active4.clone();
                    move || {
                        let sub = subtitle();
                        if sub.is_empty() {
                            None
                        } else {
                            let active4 = active4.clone();
                            Some(
                                text(sub)
                                    .color(move || {
                                        if active4() {
                                            Color::rgba(
                                                theme.background.r,
                                                theme.background.g,
                                                theme.background.b,
                                                0.7,
                                            )
                                        } else {
                                            Color::rgba(1.0, 1.0, 1.0, 0.5)
                                        }
                                    })
                                    .font_size(10.0),
                            )
                        }
                    }
                }),
        );

    // Add spacer + chevron inside the button if there's a submenu action
    if let Some(on_sub) = on_submenu {
        btn = btn
            // Fill spacer pushes chevron to the right
            .child(container().width(fill()))
            .child(
                container()
                    .padding([0.0, 4.0])
                    .on_click(move || on_sub())
                    .layout(
                        Flex::row()
                            .main_alignment(MainAlignment::Center)
                            .cross_alignment(CrossAlignment::Center),
                    )
                    .child(
                        icon(StaticIcon::RightChevron)
                            .color(move || {
                                if active5() {
                                    theme.background
                                } else {
                                    theme.text
                                }
                            })
                            .font_size(12.0),
                    ),
            );
    }

    btn
}
