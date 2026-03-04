use guido::prelude::*;

use crate::theme::ThemeColors;

use super::icons::{IconKind, StaticIcon, icon};

/// A quick-setting button: icon + title + optional subtitle + optional chevron.
/// Active state shows colored background, inactive shows dim.
/// Chevron is inside the button (unified tile) matching ashell layout.
#[component]
pub fn quick_setting(
    ic: StaticIcon,
    title: String,
    subtitle: String,
    active: bool,
    #[prop(callback)]
    on_toggle: (),
    #[prop(callback)]
    on_submenu: (),
) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let hovered = create_signal(false);

    let ic = ic.clone();
    let title = title.clone();
    let subtitle = subtitle.clone();
    let active1 = active.clone();
    let active2 = active.clone();
    let active3 = active.clone();
    let active4 = active.clone();
    let active5 = active.clone();
    let on_submenu = on_submenu.clone();

    let mut btn = container()
        .width(fill())
        .height(50)
        .corner_radius(16)
        .on_hover(move |h| hovered.set(h))
        .on_click_option(on_toggle.clone())
        .background(move || {
            if active1.get() {
                theme.primary
            } else if hovered.get() {
                Color::rgba(1.0, 1.0, 1.0, 0.1)
            } else {
                Color::rgba(1.0, 1.0, 1.0, 0.05)
            }
        })
        .layout(
            Flex::row()
                .spacing(8)
                .cross_alignment(CrossAlignment::Center),
        )
        .padding([0, 10])
        // Icon
        .child(
            icon().ic(move || IconKind::from(ic.get()))
                .color(move || {
                    if active2.get() {
                        theme.background
                    } else {
                        theme.text
                    }
                })
                .font_size(16),
        )
        // Text column
        .child(
            container()
                .layout(Flex::column().spacing(1))
                .child(
                    text(move || title.get())
                        .color(move || {
                            if active3.get() {
                                theme.background
                            } else {
                                theme.text
                            }
                        })
                        .font_size(12),
                )
                .child({
                    let subtitle = subtitle.clone();
                    let active4 = active4.clone();
                    move || {
                        let sub = subtitle.get();
                        if sub.is_empty() {
                            None
                        } else {
                            let active4 = active4.clone();
                            Some(
                                text(sub)
                                    .color(move || {
                                        if active4.get() {
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
                                    .font_size(10),
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
                    .padding([0, 4])
                    .on_click(move || on_sub())
                    .layout(
                        Flex::row()
                            .main_alignment(MainAlignment::Center)
                            .cross_alignment(CrossAlignment::Center),
                    )
                    .child(
                        icon().ic(StaticIcon::RightChevron)
                            .color(move || {
                                if active5.get() {
                                    theme.background
                                } else {
                                    theme.text
                                }
                            })
                            .font_size(12),
                    ),
            );
    }

    btn
}
