use crate::{
    config::{WorkspaceVisibilityMode, WorkspacesModuleConfig},
    outputs::Outputs,
    theme::AshellTheme,
};
use hyprland::{
    dispatch::MonitorIdentifier,
    event_listener::AsyncEventListener,
    shared::{HyprData, HyprDataActive, HyprDataVec},
};
use iced::{
    Element, Length, Subscription, alignment,
    stream::channel,
    widget::{MouseArea, Row, button, container, text},
    window::Id,
};
use itertools::Itertools;
use log::{debug, error};
use std::{
    any::TypeId,
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Displayed {
    Active,
    Visible,
    Hidden,
}

#[derive(Debug, Clone)]
pub struct Workspace {
    pub id: i32,
    pub name: String,
    pub monitor_id: Option<i128>,
    pub monitor: String,
    pub displayed: Displayed,
    pub windows: u16,
}

#[derive(Debug, Clone)]
pub struct VirtualDesktop {
    pub active: bool,
    pub windows: u16,
}

fn get_workspaces(config: &WorkspacesModuleConfig) -> Vec<Workspace> {
    let active = hyprland::data::Workspace::get_active().ok();
    let monitors = hyprland::data::Monitors::get()
        .map(|m| m.to_vec())
        .unwrap_or_default();
    let workspaces = hyprland::data::Workspaces::get()
        .map(|w| w.to_vec())
        .unwrap_or_default();

    // in some cases we can get duplicate workspaces, so we need to deduplicate them
    let workspaces: Vec<_> = workspaces.into_iter().unique_by(|w| w.id).collect();

    // We need capacity for at least all the existing entries.
    let mut result: Vec<Workspace> = Vec::with_capacity(workspaces.len());

    let (special, normal): (Vec<_>, Vec<_>) = workspaces.into_iter().partition(|w| w.id < 0);

    // map special workspaces
    if !config.disable_special_workspaces {
        for w in special.iter() {
            // Special workspaces are active if they are assigned to any monitor.
            // Currently a special and normal workspace can be active at the same time on the same monitor.
            let active = monitors.iter().any(|m| m.special_workspace.id == w.id);
            result.push(Workspace {
                id: w.id,
                name: w
                    .name
                    .split(":")
                    .last()
                    .map_or_else(|| "".to_string(), |s| s.to_owned()),
                monitor_id: w.monitor_id,
                monitor: w.monitor.clone(),
                displayed: if active {
                    Displayed::Active
                } else {
                    Displayed::Hidden
                },
                windows: w.windows,
            });
        }
    }

    if config.enable_virtual_desktops {
        let monitor_count = monitors.len();
        let mut virtual_desktops: HashMap<i32, VirtualDesktop> = HashMap::new();

        // map normal workspaces
        for w in normal.iter() {
            // Calculate the virtual desktop ID based on the workspace ID and the number of workspaces per virtual desktop
            let vdesk_id = ((w.id - 1) / monitor_count as i32) + 1;

            if let Some(vdesk) = virtual_desktops.get_mut(&vdesk_id) {
                vdesk.windows += w.windows;
                vdesk.active = vdesk.active || Some(w.id) == active.as_ref().map(|a| a.id);
            } else {
                virtual_desktops.insert(
                    vdesk_id,
                    VirtualDesktop {
                        active: Some(w.id) == active.as_ref().map(|a| a.id),
                        windows: w.windows,
                    },
                );
            }
        }

        // Add virtual desktops to the result as workspaces
        virtual_desktops.into_iter().for_each(|(id, vdesk)| {
            // Try to get a name from the config, default to ID
            let idx = (id - 1) as usize;
            let display_name = config
                .workspace_names
                .get(idx)
                .cloned()
                .unwrap_or_else(|| id.to_string());
            let active = if vdesk.active {
                Displayed::Active
            } else {
                Displayed::Hidden
            };
            result.push(Workspace {
                id,
                name: display_name,
                monitor_id: None,
                monitor: "".to_string(),
                displayed: active,
                windows: vdesk.windows,
            });
        });
    } else {
        // map normal workspaces
        for w in normal.iter() {
            let display_name = if w.id > 0 {
                let idx = (w.id - 1) as usize;
                config
                    .workspace_names
                    .get(idx)
                    .cloned()
                    .unwrap_or_else(|| w.id.to_string())
            } else {
                w.name.clone()
            };
            let active = active.as_ref().is_some_and(|a| a.id == w.id);
            let visible = monitors.iter().any(|m| m.active_workspace.id == w.id);
            result.push(Workspace {
                id: w.id,
                name: display_name,
                monitor_id: w.monitor_id,
                monitor: w.monitor.clone(),
                displayed: match (active, visible) {
                    (true, _) => Displayed::Active,
                    (false, true) => Displayed::Visible,
                    (false, false) => Displayed::Hidden,
                },
                windows: w.windows,
            });
        }
    }

    if !config.enable_workspace_filling || normal.is_empty() {
        // nothing more to do, early return
        result.sort_by_key(|w| w.id);
        return result;
    };

    // To show workspaces that don't exist in Hyprland we need to create fake ones
    let existing_ids = result.iter().map(|w| w.id).collect_vec();
    let mut max_id = *existing_ids
        .iter()
        .filter(|&&id| id > 0) // filter out special workspaces
        .max()
        .unwrap_or(&0);
    if let Some(max_workspaces) = config.max_workspaces
        && max_workspaces > max_id as u32
    {
        max_id = max_workspaces as i32;
    }
    let missing_ids: Vec<i32> = (1..=max_id)
        .filter(|id| !existing_ids.contains(id))
        .collect();

    // Rust could do reallocs for us, but here we know how many more space we need, so can do better
    result.reserve(missing_ids.len());

    for id in missing_ids {
        let display_name = if id > 0 {
            let idx = (id - 1) as usize;
            config
                .workspace_names
                .get(idx)
                .cloned()
                .unwrap_or_else(|| id.to_string())
        } else {
            id.to_string()
        };
        result.push(Workspace {
            id,
            name: display_name,
            monitor_id: None,
            monitor: "".to_string(),
            displayed: Displayed::Hidden,
            windows: 0,
        });
    }

    result.sort_by_key(|w| w.id);

    result
}

#[derive(Debug, Clone)]
pub enum Message {
    WorkspacesChanged,
    ChangeWorkspace(i32),
    ToggleSpecialWorkspace(i32),
    Scroll(i32),
}

pub struct Workspaces {
    config: WorkspacesModuleConfig,
    workspaces: Vec<Workspace>,
}

impl Workspaces {
    pub fn new(config: WorkspacesModuleConfig) -> Self {
        Self {
            workspaces: get_workspaces(&config),
            config,
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::WorkspacesChanged => {
                self.workspaces = get_workspaces(&self.config);
            }
            Message::ChangeWorkspace(id) => {
                if id > 0 {
                    let already_active = self
                        .workspaces
                        .iter()
                        .any(|w| w.displayed == Displayed::Active && w.id == id);

                    if !already_active {
                        debug!("changing workspace to: {id}");
                        let res = if self.config.enable_virtual_desktops {
                            let id_str = id.to_string();
                            hyprland::dispatch::Dispatch::call(
                                hyprland::dispatch::DispatchType::Custom("vdesk", &id_str),
                            )
                        } else {
                            hyprland::dispatch::Dispatch::call(
                                hyprland::dispatch::DispatchType::Workspace(
                                    hyprland::dispatch::WorkspaceIdentifierWithSpecial::Id(id),
                                ),
                            )
                        };

                        if let Err(e) = res {
                            error!("failed to dispatch workspace change: {e:?}");
                        }
                    }
                }
            }

            Message::ToggleSpecialWorkspace(id) => {
                if let Some(special) = self.workspaces.iter().find(|w| w.id == id && w.id < 0) {
                    debug!("toggle special workspace: {id}");
                    let res = hyprland::dispatch::Dispatch::call(
                        hyprland::dispatch::DispatchType::FocusMonitor(MonitorIdentifier::Id(
                            special.monitor_id.unwrap_or_default(),
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
                        error!("failed to dispatch special workspace toggle: {e:?}");
                    }
                }
            }
            Message::Scroll(direction) => {
                let current_workspace = self
                    .workspaces
                    .iter()
                    .find(|w| w.displayed.eq(&Displayed::Active));
                let Some(current_id) = current_workspace.map(|w| w.id) else {
                    return;
                };

                let next_workspace = if direction > 0 {
                    self.workspaces
                        .iter()
                        .filter(|w| w.id > current_id)
                        .min_by_key(|w| w.id)
                } else {
                    self.workspaces
                        .iter()
                        .filter(|w| w.id < current_id)
                        .max_by_key(|w| w.id)
                };
                let Some(next_workspace) = next_workspace else {
                    return;
                };
                Self::update(self, Message::ChangeWorkspace(next_workspace.id));
            }
        }
    }

    pub fn view<'a>(
        &'a self,
        id: Id,
        theme: &'a AshellTheme,
        outputs: &Outputs,
    ) -> Element<'a, Message> {
        let monitor_name = outputs.get_monitor_name(id);

        Into::<Element<Message>>::into(
            MouseArea::new(
                Row::with_children(
                    self.workspaces
                        .iter()
                        .filter_map(|w| {
                            let show = match self.config.visibility_mode {
                                WorkspaceVisibilityMode::All => true,
                                WorkspaceVisibilityMode::MonitorSpecific => {
                                    w.monitor == monitor_name.unwrap_or_else(|| &w.monitor)
                                        || !outputs.has_name(&w.monitor)
                                }
                                WorkspaceVisibilityMode::MonitorSpecificExclusive => {
                                    w.monitor == monitor_name.unwrap_or_else(|| &w.monitor)
                                }
                            };
                            if show {
                                let empty = w.windows == 0;

                                let color_index = if self.config.enable_virtual_desktops {
                                    // For virtual desktops, we use the workspace ID as the index
                                    Some(w.id as i128)
                                } else {
                                    // For normal workspaces, we use the monitor ID as the index
                                    w.monitor_id
                                };
                                let color = color_index.map(|i| {
                                    if w.id > 0 {
                                        theme.workspace_colors.get(i as usize).copied()
                                    } else {
                                        theme
                                            .special_workspace_colors
                                            .as_ref()
                                            .unwrap_or(&theme.workspace_colors)
                                            .get(i as usize)
                                            .copied()
                                    }
                                });

                                Some(
                                    button(
                                        container(text(w.name.as_str()).size(theme.font_size.xs))
                                            .align_x(alignment::Horizontal::Center)
                                            .align_y(alignment::Vertical::Center),
                                    )
                                    .style(theme.workspace_button_style(empty, color))
                                    .padding(if w.id < 0 {
                                        match w.displayed {
                                            Displayed::Active => [0, theme.space.md],
                                            Displayed::Visible => [0, theme.space.sm],
                                            Displayed::Hidden => [0, theme.space.xs],
                                        }
                                    } else {
                                        [0, 0]
                                    })
                                    .on_press(if w.id > 0 {
                                        Message::ChangeWorkspace(w.id)
                                    } else {
                                        Message::ToggleSpecialWorkspace(w.id)
                                    })
                                    .width(match (w.id < 0, &w.displayed) {
                                        (true, _) => Length::Shrink,
                                        (_, Displayed::Active) => {
                                            Length::Fixed(theme.space.xl as f32)
                                        }
                                        (_, Displayed::Visible) => {
                                            Length::Fixed(theme.space.lg as f32)
                                        }
                                        (_, Displayed::Hidden) => {
                                            Length::Fixed(theme.space.md as f32)
                                        }
                                    })
                                    .height(theme.space.md)
                                    .into(),
                                )
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<Element<'_, _, _>>>(),
                )
                .spacing(theme.space.xxs),
            )
            .on_scroll(move |direction| {
                let delta = match direction {
                    iced::mouse::ScrollDelta::Lines { y, .. } => y,
                    iced::mouse::ScrollDelta::Pixels { y, .. } => y,
                };

                // Scrolling down should increase workspace ID
                if delta < 0.0 {
                    Message::Scroll(1)
                } else {
                    Message::Scroll(-1)
                }
            }),
        )
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let id = TypeId::of::<Self>();
        let enable_workspace_filling = self.config.enable_workspace_filling;

        Subscription::run_with_id(
            (id, enable_workspace_filling),
            channel(10, async move |output| {
                let output = Arc::new(RwLock::new(output));
                loop {
                    let mut event_listener = AsyncEventListener::new();

                    event_listener.add_workspace_added_handler({
                        let output = output.clone();
                        move |e| {
                            debug!("workspace added: {e:?}");
                            let output = output.clone();
                            Box::pin(async move {
                                if let Ok(mut output) = output.write() {
                                    output
                                        .try_send(Message::WorkspacesChanged)
                                        .expect("error getting workspaces: workspace added event");
                                }
                            })
                        }
                    });

                    event_listener.add_workspace_changed_handler({
                        let output = output.clone();
                        move |e| {
                            debug!("workspace changed: {e:?}");
                            let output = output.clone();
                            Box::pin(async move {
                                if let Ok(mut output) = output.write() {
                                    output
                                        .try_send(Message::WorkspacesChanged)
                                        .expect("error getting workspaces: workspace change event");
                                }
                            })
                        }
                    });

                    event_listener.add_workspace_deleted_handler({
                        let output = output.clone();
                        move |e| {
                            debug!("workspace deleted: {e:?}");
                            let output = output.clone();
                            Box::pin(async move {
                                if let Ok(mut output) = output.write() {
                                    output.try_send(Message::WorkspacesChanged).expect(
                                        "error getting workspaces: workspace destroy event",
                                    );
                                }
                            })
                        }
                    });

                    event_listener.add_workspace_moved_handler({
                        let output = output.clone();
                        move |e| {
                            debug!("workspace moved: {e:?}");
                            let output = output.clone();
                            Box::pin(async move {
                                if let Ok(mut output) = output.write() {
                                    output
                                        .try_send(Message::WorkspacesChanged)
                                        .expect("error getting workspaces: workspace moved event");
                                }
                            })
                        }
                    });

                    event_listener.add_changed_special_handler({
                        let output = output.clone();
                        move |e| {
                            debug!("special workspace changed: {e:?}");
                            let output = output.clone();
                            Box::pin(async move {
                                if let Ok(mut output) = output.write() {
                                    output.try_send(Message::WorkspacesChanged).expect(
                                        "error getting workspaces: special workspace change event",
                                    );
                                }
                            })
                        }
                    });

                    event_listener.add_special_removed_handler({
                        let output = output.clone();
                        move |e| {
                            debug!("special workspace removed: {e:?}");
                            let output = output.clone();
                            Box::pin(async move {
                                if let Ok(mut output) = output.write() {
                                    output.try_send(Message::WorkspacesChanged).expect(
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
                                        .try_send(Message::WorkspacesChanged)
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
                                        .try_send(Message::WorkspacesChanged)
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
                                        .try_send(Message::WorkspacesChanged)
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
                                    output.try_send(Message::WorkspacesChanged).expect(
                                        "error getting workspaces: active monitor change event",
                                    );
                                }
                            })
                        }
                    });

                    let res = event_listener.start_listener_async().await;

                    if let Err(e) = res {
                        error!("restarting workspaces listener due to error: {e:?}");
                    }
                }
            }),
        )
    }
}
