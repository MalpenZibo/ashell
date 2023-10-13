use std::rc::Rc;

use gtk4::traits::GtkWindowExt;
use gtk4::{ApplicationWindow, Widget};
use gtk4_layer_shell::Layer;
use leptos::{create_memo, create_signal, SignalGet, SignalSet};

use crate::gtk4_wrapper::{center_box, container, overlay, Align, AppCtx, Component, LayerOption};
use crate::modules::{app_launcher, clock, settings, system_info, title, updates};

pub fn bar(app: AppCtx) -> Widget {
    let (menu, set_menu) = create_signal::<Option<ApplicationWindow>>(None);

    let open_menu = Rc::new(move |content: Widget| {
        let overlay = |window: ApplicationWindow| {
            overlay()
                .children(vec![
                    container()
                        .class(vec!["overlay"])
                        .vexpand(true)
                        .hexpand(true)
                        .on_click(move || {
                            window.close();
                            set_menu.set(None);
                        })
                        .into(),
                    container()
                        .class(vec!["menu"])
                        .halign(Align::Start)
                        .valign(Align::Start)
                        .children(vec![content])
                        .into(),
                ])
                .into()
        };
        let window = app.open_window(
            overlay,
            Some(LayerOption {
                r#type: Layer::Overlay,
                exclusive_zone: false,
                top_anchor: true,
                bottom_anchor: true,
                left_anchor: true,
                right_anchor: true,
            }),
        );

        set_menu.set(Some(window));
    });

    let close_menu = Rc::new(move || {
        if let Some(menu) = menu.get() {
            menu.close();
            set_menu.set(None);
        }
    });

    let menu_is_open = create_memo(move |_| menu.get().is_some());

    center_box()
        .class(vec!["header-bar"])
        .valign(Align::Center)
        .vexpand(false)
        .left(Some(
            container()
                .spacing(4)
                .valign(Align::Center)
                .vexpand(false)
                .children(vec![
                    app_launcher(),
                    updates(menu_is_open, open_menu.clone(), close_menu.clone()),
                ])
                .into(),
        ))
        .center(Some(container().children(vec![title()]).into()))
        .right(Some(
            container()
                .spacing(4)
                .children(vec![
                    system_info(),
                    container().children(vec![clock(), settings()]).into(),
                ])
                .into(),
        ))
        .into()
}
