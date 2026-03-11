use guido::prelude::*;

use crate::{
    components::{ButtonKind, ButtonSize, button},
    theme::ThemeColors,
};

use super::icons::StaticIcon;

/// A list item: clickable transparent button when not selected, success-colored
/// label when selected. Used for device lists (sinks, sources, etc.).
///
/// Use `.trailing(widget)` to add trailing content (e.g. a remove button).
#[component]
pub fn selectable_item(
    kind: StaticIcon,
    label: String,
    selected: bool,
    #[prop(callback)] on_click: (),
    #[prop(slot)] trailing: (),
) -> impl Widget {
    let theme = expect_context::<ThemeColors>();

    let selected = selected.get();
    let on_click = on_click.clone();

    let content_row = container()
        .width(fill())
        .layout(Flex::row().cross_alignment(CrossAlignment::Center))
        .child(
            container().width(fill()).child(
                text(label.get())
                    .color(if selected { theme.success } else { theme.text })
                    .font_size(12),
            ),
        )
        .maybe_child(trailing);

    if selected {
        container()
            .width(fill())
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
            .child(content_row)
            .into_any()
    } else {
        let mut btn = button()
            .size(ButtonSize::Normal)
            .kind(ButtonKind::Transparent)
            .fill_width(true)
            .icon(Some(kind.get().into()))
            .content(content_row);

        if let Some(cb) = on_click {
            btn = btn.on_click(move || cb());
        }

        btn.into_any()
    }
}
