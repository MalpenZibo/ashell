use gtk4::Widget;
use hyprland::{
    async_closure,
    event_listener::AsyncEventListener,
    shared::{HyprData, HyprDataActive},
};
use leptos::{create_memo, create_signal, SignalGet, SignalSet};

use crate::gtk4_wrapper::{container, spawn, Align, Component};

#[derive(Debug, Clone)]
pub struct Workspace {
    pub id: i32,
    pub monitor: Option<String>,
    pub active: bool,
    pub windows: u16,
}

pub fn workspaces() -> Widget {
    let get_workspaces = || {
        let active = hyprland::data::Workspace::get_active().unwrap();
        let workspaces = hyprland::data::Workspaces::get().unwrap();

        let mut sorted: Vec<hyprland::data::Workspace> = workspaces.collect();
        sorted.sort_by_key(|w| w.id);

        let mut current: usize = 1;
        let s = sorted
            .iter()
            .flat_map(|w| {
                let missing: usize = w.id as usize - current;
                let mut res = Vec::with_capacity(missing + 1);
                for i in 0..missing {
                    res.push(Workspace {
                        id: (current + i) as i32,
                        monitor: None,
                        active: false,
                        windows: 0,
                    });
                }
                current += missing + 1;
                res.push(Workspace {
                    id: w.id,
                    monitor: Some(w.monitor.to_string()),
                    active: w.id == active.id,
                    windows: w.windows,
                });

                res
            })
            .collect::<Vec<Workspace>>();

        s
    };

    let (workspaces, set_workspaces) = create_signal(get_workspaces());
    spawn({
        async move {
            let mut event_listener = AsyncEventListener::new();

            event_listener.add_workspace_added_handler(async_closure!(move |_| {
                set_workspaces.set(get_workspaces());
            }));

            event_listener.add_workspace_change_handler(async_closure!(move |_| {
                set_workspaces.set(get_workspaces());
            }));

            event_listener.add_workspace_destroy_handler(async_closure!(move |_| {
                set_workspaces.set(get_workspaces());
            }));

            event_listener.add_workspace_moved_handler(async_closure!(move |_| {
                set_workspaces.set(get_workspaces());
            }));

            let _ = event_listener.start_listener_async().await;
        }
    });

    let workspace_indicators = create_memo(move |_| {
        workspaces
            .get()
            .iter()
            .map(|w| {
                container()
                    .class(if w.windows > 0 {
                        vec!["workspace"]
                    } else {
                        vec!["workspace", "empty"]
                    })
                    .size((if w.active { 32 } else { 16 }, 16))
                    .into()
            })
            .collect::<Vec<Widget>>()
    });

    container()
        .spacing(4)
        .valign(Align::Center)
        .class(vec!["header-label"])
        .children(workspace_indicators)
        .into()
}
