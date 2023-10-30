use std::rc::Rc;

use futures_signals::signal::Mutable;
use gtk4_layer_shell::Layer;

use crate::{
    app::{AppCtx, CloseHandle, LayerOption},
    modules::{app_launcher, clock, settings, system_info, title, updates, workspaces},
    nodes,
    reactive_gtk::{centerbox, container, overlay, Align, Node, NodeBuilder},
};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MenuType {
    Settings,
    Updates,
}

pub enum MenuAction {
    Open(Box<dyn Fn() -> (Node, Align)>),
    Close,
}

pub fn bar(app: AppCtx) -> impl Into<Node> {
    let menu: Mutable<Option<(MenuType, CloseHandle)>> = Mutable::new(None);

    let toggle_menu = {
        let menu = menu.clone();
        Rc::new(move |menu_type: MenuType, menu_action: MenuAction| {
            let create_menu = {
                let menu = menu.clone();
                |(content, position): (Node, Align)| -> CloseHandle {
                    let overlay = |close_handle: CloseHandle| {
                        overlay().children(nodes![
                            container().vexpand(true).hexpand(true).on_click(move || {
                                close_handle();
                                menu.replace(None);
                            }),
                            container()
                                .class(vec!["menu"])
                                .halign(position)
                                .valign(Align::Start)
                                .children(nodes![content])
                        ])
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
                }
            };

            match (menu.read_only().get_cloned(), menu_action) {
                (Some((current_menu_type, close)), _) if current_menu_type == menu_type => {
                    close();
                    menu.replace(None);
                }
                (Some((current_menu_type, close)), MenuAction::Open(delegate))
                    if current_menu_type != menu_type =>
                {
                    menu.replace(None);
                    close();

                    let close_handle = create_menu(delegate());
                    menu.replace(Some((menu_type, close_handle)));
                }
                (None, MenuAction::Open(delegate)) => {
                    let window = create_menu(delegate());
                    menu.replace(Some((menu_type, window)));
                }
                _ => {}
            }
        })
    };

    centerbox()
        .class(vec!["bar"])
        .valign(Align::Center)
        .vexpand(false)
        .start(Some(
            container()
                .spacing(4)
                .vexpand(false)
                .valign(Align::Center)
                .children(nodes![app_launcher(), updates(
                        toggle_menu.clone()), workspaces()]),
        ))
        .center(Some(
            container()
                .vexpand(false)
                .valign(Align::Center)
                .children(nodes![title()]),
        ))
        .end(Some(
            container()
                .spacing(4)
                .vexpand(false)
                .valign(Align::Center)
                .children(nodes![
                    system_info(),
                    container().children(nodes!(clock(), settings(toggle_menu)))
                ]),
        ))
}
