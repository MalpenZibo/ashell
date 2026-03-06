use guido::prelude::*;

use super::icons::{IconKind, StaticIcon, icon};
use crate::theme::ThemeColors;

#[component]
pub fn expandable_panel(#[prop(slot)] header: (), #[prop(slot)] body: ()) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let expanded = create_signal(false);
    let header_hovered = create_signal(false);

    container()
        .width(fill())
        .layout(Flex::column().spacing(move || if expanded.get() { 8 } else { 0 }))
        .child(
            // Header row: [slot content] [chevron]
            container()
                .width(fill())
                .padding([6, 8])
                .corner_radius(8)
                .on_click(move || expanded.set(!expanded.get()))
                .on_hover(move |h| header_hovered.set(h))
                .background(move || {
                    if header_hovered.get() {
                        Color::rgba(1.0, 1.0, 1.0, 0.1)
                    } else {
                        Color::TRANSPARENT
                    }
                })
                .layout(
                    Flex::row()
                        .main_alignment(MainAlignment::SpaceBetween)
                        .cross_alignment(CrossAlignment::Center),
                )
                .child(header.unwrap_or_else(|| Box::new(container())))
                .child(
                    icon()
                        .kind(move || -> IconKind {
                            if expanded.get() {
                                StaticIcon::MenuClosed
                            } else {
                                StaticIcon::MenuOpen
                            }
                            .into()
                        })
                        .color(theme.text)
                        .font_size(14),
                ),
        )
        .child(
            container()
                .width(fill())
                .visible(expanded)
                .child(body.unwrap_or_else(|| Box::new(container()))),
        )
}
