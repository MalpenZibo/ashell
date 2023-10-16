use std::rc::Rc;

use gtk4::traits::GtkWindowExt;
use gtk4::{ApplicationWindow, Widget};
use gtk4_layer_shell::Layer;
use leptos::{create_signal, SignalGet, SignalSet};

use crate::gtk4_wrapper::{center_box, container, overlay, Align, AppCtx, Component, LayerOption};
use crate::modules::{app_launcher, clock, settings, system_info, title, updates, workspaces};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MenuType {
    Settings,
    Updates,
}

pub enum MenuAction {
    Open(Box<dyn Fn() -> (Widget, Align)>),
    Close,
}

pub fn bar(app: AppCtx) -> Widget {
    let (menu, set_menu) = create_signal::<Option<(MenuType, ApplicationWindow)>>(None);

    let toggle_menu = Rc::new(move |menu_type: MenuType, action: MenuAction| {
        let create_menu = |(content, position): (Widget, Align)| -> ApplicationWindow {
            let overlay = |window: ApplicationWindow| {
                overlay()
                    .children(vec![
                        container()
                            .vexpand(true)
                            .hexpand(true)
                            .on_click(move || {
                                window.close();
                                set_menu.set(None);
                            })
                            .into(),
                        container()
                            .class(vec!["menu"])
                            .halign(position)
                            .valign(Align::Start)
                            .children(vec![content])
                            .into(),
                    ])
                    .into()
            };
            app.open_window(
                overlay,
                Some(LayerOption {
                    r#type: Layer::Overlay,
                    exclusive_zone: false,
                    top_anchor: true,
                    bottom_anchor: true,
                    left_anchor: true,
                    right_anchor: true,
                }),
            )
        };

        match (menu.get(), action) {
            (Some((current_menu_type, window)), _) if current_menu_type == menu_type => {
                window.close();
                set_menu.set(None);
            }
            (Some((current_menu_type, window)), MenuAction::Open(delegate))
                if current_menu_type != menu_type =>
            {
                set_menu.set(None);
                window.close();

                let window = create_menu(delegate());
                set_menu.set(Some((menu_type, window)));
            }
            (None, MenuAction::Open(delegate)) => {
                let window = create_menu(delegate());
                set_menu.set(Some((menu_type, window)));
            }
            _ => {}
        }
    });

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
                    updates(toggle_menu.clone()),
                    workspaces(),
                ])
                .into(),
        ))
        .center(Some(container().children(vec![title()]).into()))
        .right(Some(
            container()
                .spacing(4)
                .children(vec![
                    system_info(),
                    container()
                        .children(vec![clock(), settings(toggle_menu.clone())])
                        .into(),
                ])
                .into(),
        ))
        .into()
}
