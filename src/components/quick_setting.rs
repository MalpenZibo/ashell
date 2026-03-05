use guido::prelude::*;

use crate::{
    components::{ButtonHierarchy, ButtonKind, ButtonSize, button, buttons::icon_button},
    theme::ThemeColors,
};

use super::icons::{IconKind, StaticIcon};

/// A quick-setting button: icon + title + optional subtitle + optional chevron.
/// Active state shows colored background, inactive shows dim.
/// Chevron is inside the button (unified tile) matching ashell layout.
#[component]
pub fn quick_setting(
    ic: StaticIcon,
    title: String,
    subtitle: String,
    active: bool,
    #[prop(callback)] on_toggle: (),
    #[prop(callback)] on_submenu: (),
    #[prop(default = "false")] expanded: bool,
) -> impl Widget {
    let theme = expect_context::<ThemeColors>();

    let on_toggle = on_toggle.clone();
    let on_submenu = on_submenu.clone();

    // Build inner content: [title column (fill)] [optional chevron]
    let mut inner = container()
        .layout(
            Flex::row()
                .spacing(8)
                .cross_alignment(CrossAlignment::Center),
        )
        .width(fill())
        .height(fill())
        // Text column
        .child(
            container()
                .layout(
                    Flex::column()
                        .main_alignment(MainAlignment::Center)
                        .spacing(2),
                )
                .width(fill())
                .height(fill())
                .child(
                    text(move || title.get())
                        .color(move || {
                            if active.get() {
                                theme.background
                            } else {
                                theme.text
                            }
                        })
                        .font_size(12),
                )
                .child({
                    move || {
                        let sub = subtitle.get();
                        if sub.is_empty() {
                            None
                        } else {
                            Some(
                                text(sub)
                                    .nowrap()
                                    .color(move || {
                                        if active.get() {
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

    // Add chevron button if there's a submenu action
    if let Some(on_sub) = on_submenu {
        inner = inner.child(
            icon_button()
                .size(ButtonSize::Small)
                .kind(move || {
                    if expanded.get() {
                        ButtonKind::Solid
                    } else {
                        ButtonKind::Transparent
                    }
                })
                .hierarchy(move || {
                    if expanded.get() {
                        // Expanded (close button): inverted colors on tile
                        if active.get() {
                            ButtonHierarchy::Custom {
                                bg: theme.background,
                                fg: theme.text,
                            }
                        } else {
                            ButtonHierarchy::Secondary
                        }
                    } else {
                        // Collapsed chevron
                        if active.get() {
                            ButtonHierarchy::Custom {
                                bg: Color::TRANSPARENT,
                                fg: theme.background,
                            }
                        } else {
                            ButtonHierarchy::Secondary
                        }
                    }
                })
                .icon(move || {
                    IconKind::Static(if expanded.get() {
                        StaticIcon::Close
                    } else {
                        StaticIcon::RightChevron
                    })
                })
                .on_click(move || on_sub()),
        );
    }

    // Wrap in a Large Solid button with reactive hierarchy + icon
    let mut btn = button()
        .size(ButtonSize::Large)
        .kind(ButtonKind::Solid)
        .icon(move || Some(IconKind::from(ic.get())))
        .hierarchy(move || {
            if active.get() {
                ButtonHierarchy::Primary
            } else {
                ButtonHierarchy::Secondary
            }
        })
        .content(inner);

    if let Some(on_toggle) = on_toggle {
        btn = btn.on_click(move || on_toggle());
    }

    btn
}
