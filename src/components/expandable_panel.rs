use guido::prelude::*;

use super::icons::{StaticIcon, icon};
use crate::theme;

#[component]
pub struct ExpandablePanel {
    #[prop(slot)]
    header: (),
    #[prop(slot)]
    body: (),
}

impl ExpandablePanel {
    fn render(&self) -> impl Widget + use<> {
        let expanded = create_signal(false);
        let header_hovered = create_signal(false);
        let header = self.take_header();
        let body = self.take_body();

        container()
            .width(fill())
            .layout(Flex::column().spacing(move || if expanded.get() { 8.0 } else { 0.0 }))
            .child(
                // Header row: [slot content] [chevron]
                container()
                    .width(fill())
                    .padding_xy(8.0, 6.0)
                    .corner_radius(8.0)
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
                            .main_axis_alignment(MainAxisAlignment::SpaceBetween)
                            .cross_axis_alignment(CrossAxisAlignment::Center),
                    )
                    .child(header.unwrap_or_else(|| Box::new(container())))
                    .child(
                        icon(move || {
                            if expanded.get() {
                                StaticIcon::MenuClosed
                            } else {
                                StaticIcon::MenuOpen
                            }
                        })
                        .color(theme::TEXT)
                        .font_size(14.0),
                    ),
            )
            .child(
                container()
                    .width(fill())
                    .visible(expanded)
                    .child(body.unwrap_or_else(|| Box::new(container()))),
            )
    }
}
