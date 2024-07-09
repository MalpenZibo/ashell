use crate::style::header_pills;
use hex_color::HexColor;
use hyprland::{
    event_listener::EventListener,
    shared::{HyprData, HyprDataActive, HyprDataVec},
};
use iced::{
    alignment, subscription, widget::{container, mouse_area, text, Row}, Background, Border, Color, Element, Length, Subscription, Theme
};
use std::cell::RefCell;

#[derive(Debug, Clone)]
pub struct Workspace {
    pub id: i32,
    pub monitor: Option<usize>,
    pub active: bool,
    pub windows: u16,
}

const MONITOR: [&str; 3] = ["eDP-1", "DP-1", "DP-2"];

fn get_workspaces() -> Vec<Workspace> {
    let active = hyprland::data::Workspace::get_active().unwrap();
    let mut workspaces = hyprland::data::Workspaces::get()
        .map(|w| w.to_vec())
        .unwrap_or_default();

    workspaces.sort_by_key(|w| w.id);

    let mut current: usize = 1;
    let s = workspaces
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
                monitor: MONITOR.iter().position(|m| w.monitor == *m),
                active: w.id == active.id,
                windows: w.windows,
            });

            res
        })
        .collect::<Vec<Workspace>>();

    s
}

pub struct Workspaces {
    workspaces: Vec<Workspace>,
}

#[derive(Debug, Clone)]
pub enum Message {
    WorkspacesChanged(Vec<Workspace>),
    ChangeWorkspace(i32),
}

impl Workspaces {
    pub fn new() -> Self {
        Self {
            workspaces: get_workspaces(),
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::WorkspacesChanged(workspaces) => {
                self.workspaces = workspaces;
            }
            Message::ChangeWorkspace(id) => {
                hyprland::dispatch::Dispatch::call(hyprland::dispatch::DispatchType::Workspace(
                    hyprland::dispatch::WorkspaceIdentifierWithSpecial::Id(id),
                ))
                .expect("failed to dispatch workspace change");
            }
        }
    }

    pub fn view(&self, workspace_colors: &[HexColor]) -> Element<Message> {
        container(
            Row::with_children(
                self.workspaces
                    .iter()
                    .map(|w| {
                        let empty = w.windows == 0;
                        let monitor = w.monitor;
                        mouse_area(
                            container(text(w.id).size(10))
                                .style({
                                    let workspace_colors = workspace_colors.to_vec();
                                    move |theme: &Theme| {
                                        let fg_color = if empty {
                                            theme.palette().text
                                        } else {
                                            theme.palette().background
                                        };
                                        let bg_color = monitor.map_or(
                                            theme.extended_palette().background.weak.color,
                                            |m| {
                                                workspace_colors
                                                    .get(m)
                                                    .map(|c| Color::from_rgb8(c.r, c.g, c.b))
                                                    .unwrap_or(theme.palette().primary)
                                            },
                                        );
                                        container::Appearance {
                                            background: Some(Background::Color(if empty {
                                                theme.extended_palette().background.weak.color
                                            } else {
                                                bg_color
                                            })),
                                            border: Border {
                                                width: if empty { 1.0 } else { 0.0 },
                                                color: bg_color,
                                                radius: 16.0.into(),
                                            },
                                            text_color: Some(fg_color),
                                            ..container::Appearance::default()
                                        }
                                    }
                                })
                                .align_x(alignment::Horizontal::Center)
                                .align_y(alignment::Vertical::Center)
                                .width(if w.active { 32 } else { 16 })
                                .height(16),
                        )
                        .on_release(Message::ChangeWorkspace(w.id))
                        .into()
                    })
                    .collect::<Vec<Element<'_, _, _>>>(),
            )
            .spacing(4),
        )
        .padding([4, 8])
        .align_y(alignment::Vertical::Center)
        .height(Length::Shrink)
        .style(header_pills)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        subscription::channel("workspaces-listener", 10, |output| async move {
            let output = RefCell::new(output);
            let mut event_listener = EventListener::new();

            event_listener.add_workspace_added_handler({
                let output = output.clone();
                move |_| {
                    let mut output = output.borrow_mut();
                    output
                        .try_send(Message::WorkspacesChanged(get_workspaces()))
                        .expect("error getting workspaces: workspace added event");
                }
            });

            event_listener.add_workspace_change_handler({
                let output = output.clone();
                move |_| {
                    let mut output = output.borrow_mut();
                    output
                        .try_send(Message::WorkspacesChanged(get_workspaces()))
                        .expect("error getting workspaces: workspace change event");
                }
            });

            event_listener.add_workspace_destroy_handler({
                let output = output.clone();
                move |_| {
                    let mut output = output.borrow_mut();
                    output
                        .try_send(Message::WorkspacesChanged(get_workspaces()))
                        .expect("error getting workspaces: workspace destroy event");
                }
            });

            event_listener.add_workspace_moved_handler({
                let output = output.clone();
                move |_| {
                    let mut output = output.borrow_mut();
                    output
                        .try_send(Message::WorkspacesChanged(get_workspaces()))
                        .expect("error getting workspaces: workspace moved event");
                }
            });

            event_listener.add_window_close_handler({
                let output = output.clone();
                move |_| {
                    let mut output = output.borrow_mut();
                    output
                        .try_send(Message::WorkspacesChanged(get_workspaces()))
                        .expect("error getting workspaces: window close event");
                }
            });

            event_listener.add_window_open_handler({
                let output = output.clone();
                move |_| {
                    let mut output = output.borrow_mut();
                    output
                        .try_send(Message::WorkspacesChanged(get_workspaces()))
                        .expect("error getting workspaces: window open event");
                }
            });

            event_listener.add_window_moved_handler({
                let output = output.clone();
                move |_| {
                    let mut output = output.borrow_mut();
                    output
                        .try_send(Message::WorkspacesChanged(get_workspaces()))
                        .expect("error getting workspaces: window moved event");
                }
            });

            event_listener.add_active_monitor_change_handler({
                let output = output.clone();
                move |_| {
                    let mut output = output.borrow_mut();
                    output
                        .try_send(Message::WorkspacesChanged(get_workspaces()))
                        .expect("error getting workspaces: active monitor change event");
                }
            });

            event_listener
                .start_listener_async()
                .await
                .expect("failed to start workspaces listener");

            panic!("Exiting hyprland event listener");
        })
    }
}
