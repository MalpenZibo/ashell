use guido::prelude::*;

use crate::{
    components::{ButtonKind, ButtonSize, button},
    theme::ThemeColors,
};

use super::icons::StaticIcon;

/// A list item: clickable transparent button when not selected, success-colored
/// label when selected. Used for device lists (sinks, sources, etc.).
#[component]
pub fn selectable_item(
    kind: StaticIcon,
    label: String,
    selected: bool,
    #[prop(callback)] on_click: (),
) -> impl Widget {
    let theme = expect_context::<ThemeColors>();

    let selected = selected.get();
    let on_click = on_click.clone();

    // Outer container so both branches share the same return type
    let mut outer = container().width(fill());

    if selected {
        // Just a success-colored label with icon
        outer = outer.child(
            container()
                .height(32)
                .padding([0, 8])
                .layout(
                    Flex::row()
                        .spacing(8)
                        .cross_alignment(CrossAlignment::Center),
                )
                .child(
                    super::icons::icon()
                        .kind(kind.get())
                        .color(theme.success)
                        .font_size(14),
                )
                .child(text(label.get()).color(theme.success).font_size(12)),
        );
    } else {
        // Transparent clickable button
        let mut btn = button()
            .size(ButtonSize::Normal)
            .kind(ButtonKind::Transparent)
            .fill_width(true)
            .icon(Some(kind.get().into()))
            .content(text(label.get()).color(theme.text).font_size(12));

        if let Some(cb) = on_click {
            btn = btn.on_click(move || cb());
        }

        outer = outer.child(btn);
    }

    outer
}
