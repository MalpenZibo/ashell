use crate::{
    clock::clock,
    reactive_gtk::{CenterBox, Context},
    screenshare::screenshare,
    settings::settings,
    system_info::system_info,
    title::title,
    updates::update_button,
    workspaces::worspaces,
};

use futures_signals::signal::Mutable;
use gtk::ApplicationWindow;

use crate::{
    launcher::launch_rofi,
    reactive_gtk::{Box, Component, Label, Node},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MenuType {
    Updates,
    Settings,
}

fn application_button() -> Node {
    Box::default()
        .class(&["rounded-m", "bg", "interactive"])
        .on_click(launch_rofi)
        .children(vec![Label::default().class(&["ph-2"]).text("󱗼").into()])
        .into()
}

fn right(ctx: Context, menu: Mutable<Option<(ApplicationWindow, Node, MenuType)>>) -> Node {
    Box::default()
        .spacing(4)
        .children(vec![
            system_info(),
            Box::default()
                .children(vec![clock(), settings(ctx, menu)])
                .into(),
        ])
        .into()
}

pub fn create_shell_bar(ctx: Context) -> Node {
    let menu: Mutable<Option<(ApplicationWindow, Node, MenuType)>> = Mutable::new(None);
    CenterBox::default()
        .class(&["text-bold", "ph-1", "pv-1"])
        .children((
            Some(
                Box::default()
                    .spacing(4)
                    .children(vec![
                        application_button(),
                        update_button(ctx.clone(), menu.clone()),
                        screenshare(),
                        worspaces(),
                    ])
                    .into(),
            ),
            Some(title()),
            Some(right(ctx, menu)),
        ))
        .into()
}
