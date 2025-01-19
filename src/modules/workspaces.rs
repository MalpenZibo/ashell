use super::{Module, OnModulePress};
use crate::{
    app,
    config::{AppearanceColor, WorkspaceVisibilityMode, WorkspacesModuleConfig},
    outputs::Outputs,
    style::WorkspaceButtonStyle,
};
use hyprland::{
    dispatch::MonitorIdentifier,
    event_listener::AsyncEventListener,
    shared::{HyprData, HyprDataActive, HyprDataVec},
};
use iced::{
    alignment,
    stream::channel,
    widget::{button, container, text, Row},
    window::Id,
    Element, Length, Subscription,
};
use log::{debug, error};
use std::{
    any::TypeId,
    sync::{Arc, RwLock},
};

#[derive(Debug, Clone)]
pub struct Workspace {
    pub id: i32,
    pub name: String,
    pub monitor_id: Option<usize>,
    pub monitor: String,
    pub active: bool,
    pub windows: u16,
}

fn get_workspaces(enable_workspace_filling: bool) -> Vec<Workspace> {
    let active = hyprland::data::Workspace::get_active().ok();
    let monitors = hyprland::data::Monitors::get()
        .map(|m| m.to_vec())
        .unwrap_or_default();
    let mut workspaces = hyprland::data::Workspaces::get()
        .map(|w| w.to_vec())
        .unwrap_or_default();

    workspaces.sort_by_key(|w| w.id);

    let mut current: usize = 1;

    workspaces
        .into_iter()
        .flat_map(|w| {
            if w.id < 0 {
                vec![Workspace {
                    id: w.id,
                    name: w
                        .name
                        .split(":")
                        .last()
                        .map_or_else(|| "".to_string(), |s| s.to_owned()),
                    monitor_id: Some(w.monitor_id as usize),
                    monitor: w.monitor,
                    active: monitors.iter().any(|m| m.special_workspace.id == w.id),
                    windows: w.windows,
                }]
            } else {
                let missing: usize = w.id as usize - current;
                let mut res = Vec::with_capacity(missing + 1);

                if enable_workspace_filling {
                    for i in 0..missing {
                        res.push(Workspace {
                            id: (current + i) as i32,
                            name: (current + i).to_string(),
                            monitor_id: None,
                            monitor: "".to_string(),
                            active: false,
                            windows: 0,
                        });
                    }
                    current += missing + 1;
                }

                res.push(Workspace {
                    id: w.id,
                    name: w.name.clone(),
                    monitor_id: Some(w.monitor_id as usize),
                    monitor: w.monitor,
                    active: Some(w.id) == active.as_ref().map(|a| a.id),
                    windows: w.windows,
                });

                res
            }
        })
        .collect::<Vec<Workspace>>()
}

pub struct Workspaces {
    workspaces: Vec<Workspace>,
}

impl Workspaces {
    pub fn new(enable_workspace_filling: bool) -> Self {
        Self {
            workspaces: get_workspaces(enable_workspace_filling),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    WorkspacesChanged(Vec<Workspace>),
    ChangeWorkspace(i32),
    ToggleSpecialWorkspace(i32),
}

impl Workspaces {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::WorkspacesChanged(workspaces) => {
                self.workspaces = workspaces;
            }
            Message::ChangeWorkspace(id) => {
                if id > 0 {
                    let already_active = self.workspaces.iter().any(|w| w.active && w.id == id);

                    if !already_active {
                        debug!("changing workspace to: {}", id);
                        let res = hyprland::dispatch::Dispatch::call(
                            hyprland::dispatch::DispatchType::Workspace(
                                hyprland::dispatch::WorkspaceIdentifierWithSpecial::Id(id),
                            ),
                        );

                        if let Err(e) = res {
                            error!("failed to dispatch workspace change: {:?}", e);
                        }
                    }
                }
            }
            Message::ToggleSpecialWorkspace(id) => {
                if let Some(special) = self.workspaces.iter().find(|w| w.id == id && w.id < 0) {
                    debug!("toggle special workspace: {}", id);
                    let res = hyprland::dispatch::Dispatch::call(
                        hyprland::dispatch::DispatchType::FocusMonitor(MonitorIdentifier::Id(
                            special.monitor_id.unwrap_or_default() as i128,
                        )),
                    )
                    .and_then(|_| {
                        hyprland::dispatch::Dispatch::call(
                            hyprland::dispatch::DispatchType::ToggleSpecialWorkspace(Some(
                                special.name.clone(),
                            )),
                        )
                    });

                    if let Err(e) = res {
                        error!("failed to dispatch special workspace toggle: {:?}", e);
                    }
                }
            }
        }
    }
}

impl Module for Workspaces {
    type ViewData<'a> = (
        &'a Outputs,
        Id,
        &'a WorkspacesModuleConfig,
        &'a [AppearanceColor],
        Option<&'a [AppearanceColor]>,
    );
    type SubscriptionData<'a> = &'a WorkspacesModuleConfig;

    fn view(
        &self,
        (outputs, id, config, workspace_colors, special_workspace_colors): Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        let monitor_name = outputs.get_monitor_name(id);

        Some((
            Into::<Element<Message>>::into(
                Row::with_children(
                    self.workspaces
                        .iter()
                        .filter_map(|w| {
                            if config.visibility_mode == WorkspaceVisibilityMode::All
                                || w.monitor == monitor_name.unwrap_or_else(|| &w.monitor)
                                || !outputs.has_name(&w.monitor)
                            {
                                let empty = w.windows == 0;
                                let monitor = w.monitor_id;

                                let color = monitor.map(|m| {
                                    if w.id > 0 {
                                        workspace_colors.get(m).copied()
                                    } else {
                                        special_workspace_colors
                                            .unwrap_or(workspace_colors)
                                            .get(m)
                                            .copied()
                                    }
                                });

                                Some(
                                    button(
                                        container(
                                            if w.id < 0 {
                                                text(w.name.as_str())
                                            } else {
                                                text(w.id)
                                            }
                                            .size(10),
                                        )
                                        .align_x(alignment::Horizontal::Center)
                                        .align_y(alignment::Vertical::Center),
                                    )
                                    .style(WorkspaceButtonStyle(empty, color).into_style())
                                    .padding(if w.id < 0 {
                                        if w.active {
                                            [0, 16]
                                        } else {
                                            [0, 8]
                                        }
                                    } else {
                                        [0, 0]
                                    })
                                    .on_press(if w.id > 0 {
                                        Message::ChangeWorkspace(w.id)
                                    } else {
                                        Message::ToggleSpecialWorkspace(w.id)
                                    })
                                    .width(if w.id < 0 {
                                        Length::Shrink
                                    } else if w.active {
                                        Length::Fixed(32.)
                                    } else {
                                        Length::Fixed(16.)
                                    })
                                    .height(16)
                                    .into(),
                                )
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<Element<'_, _, _>>>(),
                )
                .padding([2, 0])
                .spacing(4),
            )
            .map(app::Message::Workspaces),
            None,
        ))
    }

    fn subscription(
        &self,
        config: Self::SubscriptionData<'_>,
    ) -> Option<Subscription<app::Message>> {
        let id = TypeId::of::<Self>();
        let enable_workspace_filling = config.enable_workspace_filling;

        Some(
            Subscription::run_with_id(
                format!("{:?}-{}", id, enable_workspace_filling),
                channel(10, move |output| async move {
                    let output = Arc::new(RwLock::new(output));
                    loop {
                        let mut event_listener = AsyncEventListener::new();

                        event_listener.add_workspace_added_handler({
                            let output = output.clone();
                            move |e| {
                                debug!("workspace added: {:?}", e);
                                let output = output.clone();
                                Box::pin(async move {
                                    if let Ok(mut output) = output.write() {
                                        output
                                            .try_send(Message::WorkspacesChanged(get_workspaces(
                                                enable_workspace_filling,
                                            )))
                                            .expect(
                                                "error getting workspaces: workspace added event",
                                            );
                                    }
                                })
                            }
                        });

                        event_listener.add_workspace_changed_handler({
                            let output = output.clone();
                            move |e| {
                                debug!("workspace changed: {:?}", e);
                                let output = output.clone();
                                Box::pin(async move {
                                    if let Ok(mut output) = output.write() {
                                        output
                                            .try_send(Message::WorkspacesChanged(get_workspaces(enable_workspace_filling)))
                                            .expect(
                                                "error getting workspaces: workspace change event",
                                            );
                                    }
                                })
                            }
                        });

                        event_listener.add_workspace_deleted_handler({
                            let output = output.clone();
                            move |e| {
                                debug!("workspace deleted: {:?}", e);
                                let output = output.clone();
                                Box::pin(async move {
                                    if let Ok(mut output) = output.write() {
                                        output
                                            .try_send(Message::WorkspacesChanged(get_workspaces(enable_workspace_filling)))
                                            .expect(
                                                "error getting workspaces: workspace destroy event",
                                            );
                                    }
                                })
                            }
                        });

                        event_listener.add_workspace_moved_handler({
                            let output = output.clone();
                            move |e| {
                                debug!("workspace moved: {:?}", e);
                                let output = output.clone();
                                Box::pin(async move {
                                    if let Ok(mut output) = output.write() {
                                        output
                                            .try_send(Message::WorkspacesChanged(get_workspaces(enable_workspace_filling)))
                                            .expect(
                                                "error getting workspaces: workspace moved event",
                                            );
                                    }
                                })
                            }
                        });

                        event_listener.add_changed_special_handler({
                            let output = output.clone();
                            move |e| {
                                debug!("special workspace changed: {:?}", e);
                                let output = output.clone();
                                Box::pin(async move {
                                    if let Ok(mut output) = output.write() {
                                        output
                                    .try_send(Message::WorkspacesChanged(get_workspaces(enable_workspace_filling)))
                                    .expect(
                                        "error getting workspaces: special workspace change event",
                                    );
                                    }
                                })
                            }
                        });

                        event_listener.add_special_removed_handler({
                            let output = output.clone();
                            move |e| {
                                debug!("special workspace removed: {:?}", e);
                                let output = output.clone();
                                Box::pin(async move {
                                    if let Ok(mut output) = output.write() {
                                        output
                                    .try_send(Message::WorkspacesChanged(get_workspaces(enable_workspace_filling)))
                                    .expect(
                                        "error getting workspaces: special workspace removed event",
                                    );
                                    }
                                })
                            }
                        });

                        event_listener.add_window_closed_handler({
                            let output = output.clone();
                            move |_| {
                                let output = output.clone();
                                Box::pin(async move {
                                    if let Ok(mut output) = output.write() {
                                        output
                                            .try_send(Message::WorkspacesChanged(get_workspaces(enable_workspace_filling)))
                                            .expect("error getting workspaces: window close event");
                                    }
                                })
                            }
                        });

                        event_listener.add_window_opened_handler({
                            let output = output.clone();
                            move |_| {
                                let output = output.clone();
                                Box::pin(async move {
                                    if let Ok(mut output) = output.write() {
                                        output
                                            .try_send(Message::WorkspacesChanged(get_workspaces(enable_workspace_filling)))
                                            .expect("error getting workspaces: window open event");
                                    }
                                })
                            }
                        });

                        event_listener.add_window_moved_handler({
                            let output = output.clone();
                            move |_| {
                                let output = output.clone();
                                Box::pin(async move {
                                    if let Ok(mut output) = output.write() {
                                        output
                                            .try_send(Message::WorkspacesChanged(get_workspaces(enable_workspace_filling)))
                                            .expect("error getting workspaces: window moved event");
                                    }
                                })
                            }
                        });

                        event_listener.add_active_monitor_changed_handler({
                            let output = output.clone();
                            move |_| {
                                let output = output.clone();
                                Box::pin(async move {
                                    if let Ok(mut output) = output.write() {
                                        output
                                        .try_send(Message::WorkspacesChanged(get_workspaces(enable_workspace_filling)))
                                        .expect(
                                            "error getting workspaces: active monitor change event",
                                        );
                                    }
                                })
                            }
                        });

                        let res = event_listener.start_listener_async().await;

                        if let Err(e) = res {
                            error!("restarting workspaces listener due to error: {:?}", e);
                        }
                    }
                }),
            )
            .map(app::Message::Workspaces),
        )
    }
}
