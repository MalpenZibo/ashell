use futures_signals::signal_vec::{MutableVec, SignalVecExt};
use hyprland::{
    event_listener::EventListener,
    shared::{HyprData, HyprDataActive},
};
use tokio::spawn;

use crate::reactive_gtk::{Align, Box, Component, Node};

#[derive(Debug, Clone)]
pub struct Workspace {
    pub id: i32,
    pub monitor: Option<String>,
    pub active: bool,
    pub windows: u16,
}

pub fn worspaces() -> Node {
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
            .collect();

        s
    };

    let workspaces = MutableVec::new_with_values(get_workspaces());

    let workspaces1 = workspaces.clone();
    spawn(async move {
        let mut event_listener = EventListener::new();

        let workspaces2 = workspaces1.clone();
        event_listener.add_workspace_added_handler(move |_| {
            workspaces2.lock_mut().replace_cloned(get_workspaces());
        });

        let workspaces3 = workspaces1.clone();
        event_listener.add_workspace_change_handler(move |_| {
            workspaces3.lock_mut().replace_cloned(get_workspaces());
        });

        let workspaces4 = workspaces1.clone();
        event_listener.add_workspace_destroy_handler(move |_| {
            workspaces4.lock_mut().replace_cloned(get_workspaces());
        });

        event_listener.add_workspace_moved_handler(move |_| {
            workspaces1.lock_mut().replace_cloned(get_workspaces());
        });

        event_listener
            .start_listener_async()
            .await
            .expect("failed to start listener");
    });

    Box::default()
        .class(&["bg", "ph-3", "rounded-m"])
        .spacing(4)
        .children_signal_vec(workspaces.signal_vec_cloned().map(|w| {
            Box::default()
                .class(if w.windows > 0 {
                    &["rounded-l", "interactive", "bg-accent"]
                } else {
                    &["rounded-l", "interactive", "bg-dark-3"]
                })
                .on_click(move || {
                    hyprland::dispatch::Dispatch::call(
                        hyprland::dispatch::DispatchType::Workspace(
                            hyprland::dispatch::WorkspaceIdentifierWithSpecial::Id(w.id),
                        ),
                    )
                    .expect("failed to dispatch workspace change");
                })
                .valign(Align::Center)
                .homogeneous(true)
                .size((16, 16))
                .children(vec![Box::default()
                    .class(&["rounded-l", "bg-dark-4"])
                    .size((12, 12))
                    .halign(Align::Center)
                    .valign(Align::Center)
                    .visible(w.windows > 0 && !w.active)
                    .into()])
                .into()
        }))
        .into()
}
