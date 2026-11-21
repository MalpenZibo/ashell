use guido::prelude::*;
use styletto::components::button::{pill, pill_text};
use styletto::foundations::control::Size;
use styletto::foundations::tokens::{radius, size};
use styletto::primitives::text::{caption, heading};
use styletto::ui;

use crate::config::Panel;

pub fn panel_width() -> f32 {
    size::scaled(size::S100.px())
}

pub fn zone<F: Fn() + 'static>(name: &str, click: F) -> impl Widget + use<F> {
    pill().on_click(click).child(pill_text(name.to_string()))
}

pub fn panel(panel: Panel) -> Container {
    let t = ui();
    let row = || {
        container()
            .width(fill())
            .height(Size::Md.height())
            .corners(radius::MEDIUM.corners())
            .background(t.colors.surface)
    };

    container()
        .width(panel_width())
        .padding(size::S4.px())
        .layout(Flex::column().spacing(size::S3.px()))
        .child(heading(panel.name().to_string()))
        .child(caption("Placeholder"))
        .child(row())
        .child(row())
        .child(row())
}
