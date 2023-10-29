use futures_signals::signal::Mutable;
use hyprland::{
    event_listener::EventListener,
    shared::{HyprData, HyprDataActive},
};

use crate::{
    nodes,
    reactive_gtk::{container, Align, Node, NodeBuilder, Dynamic},
};

#[derive(Debug, Clone)]
pub struct Workspace {
    pub id: i32,
    pub monitor: Option<String>,
    pub active: bool,
    pub windows: u16,
}

pub fn workspaces() -> impl Into<Node> {
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

    let workspaces = Mutable::new(get_workspaces());
    tokio::spawn({
        let workspaces = workspaces.clone();
        async move {
            let mut event_listener = EventListener::new();

            event_listener.add_workspace_added_handler({
                let workspaces = workspaces.clone();
                move |_| {
                    workspaces.replace(get_workspaces());
                }
            });

            event_listener.add_workspace_change_handler({
                let workspaces = workspaces.clone();
                move |_| {
                    workspaces.replace(get_workspaces());
                }
            });

            event_listener.add_workspace_destroy_handler({
                let workspaces = workspaces.clone();
                move |_| {
                    workspaces.replace(get_workspaces());
                }
            });

            event_listener.add_workspace_moved_handler({
                let workspaces = workspaces.clone();
                move |_| {
                    workspaces.replace(get_workspaces());
                }
            });

            event_listener
                .start_listener_async()
                .await
                .expect("failed to start workspaces listener");
        }
    });

    let workspaces = workspaces.signal_ref(|w| {
        w.iter()
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
            .collect::<Vec<Node>>()
    });

    container()
        .spacing(4)
        .valign(Align::Center)
        .class(vec!["bar-item", "workspaces"])
        .children(Dynamic(workspaces))
}
