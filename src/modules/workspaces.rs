use hyprland::{
    event_listener::EventListener,
    shared::{HyprData, HyprDataActive},
};
use iced::{
    widget::{container, horizontal_space, mouse_area, row, space, text, Row},
    BorderRadius, Color, Element, Length, Theme,
};
use std::cell::RefCell;

use crate::style::{header_pills, BASE, LAVENDER, MAUVE, PEACH, SURFACE_0, TEXT};

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

    pub fn view(&self) -> Element<Message> {
        container(
            Row::with_children(
                self.workspaces
                    .iter()
                    .map(|w| {
                        let bg_color = w.monitor.map_or(SURFACE_0, |m| match m {
                            0 => PEACH,
                            1 => LAVENDER,
                            2 => MAUVE,
                            _ => PEACH,
                        });
                        let empty = w.windows == 0;
                        let fg_color = if empty { TEXT } else { BASE };
                        mouse_area(
                            container(text(w.id).size(12))
                                .style(move |_theme: &Theme| iced::widget::container::Appearance {
                                    background: Some(iced::Background::Color(if empty {
                                        SURFACE_0
                                    } else {
                                        bg_color
                                    })),
                                    border_color: bg_color,
                                    border_width: if empty { 1.0 } else { 0.0 },
                                    border_radius: BorderRadius::from(16.0),
                                    text_color: Some(fg_color),
                                    ..iced::widget::container::Appearance::default()
                                })
                                .align_x(iced::alignment::Horizontal::Center)
                                .align_y(iced::alignment::Vertical::Center)
                                .width(if w.active { 32 } else { 18 })
                                .height(18),
                        )
                        .on_release(Message::ChangeWorkspace(w.id))
                        .into()
                    })
                    .collect::<Vec<Element<'_, _, _>>>(),
            )
            .spacing(4),
        )
        .padding([4, 8])
        .align_y(iced::alignment::Vertical::Center)
        .height(Length::Fill)
        .style(header_pills)
        .into()
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        iced::subscription::channel("workspaces-listener", 10, |output| async move {
            let output = RefCell::new(output);
            let mut event_listener = EventListener::new();

            event_listener.add_workspace_added_handler({
                let output = output.clone();
                move |_| {
                    let mut output = output.borrow_mut();
                    output
                        .try_send(Message::WorkspacesChanged(get_workspaces()))
                        .unwrap();
                }
            });

            event_listener.add_workspace_change_handler({
                let output = output.clone();
                move |_| {
                    let mut output = output.borrow_mut();
                    output
                        .try_send(Message::WorkspacesChanged(get_workspaces()))
                        .unwrap();
                }
            });

            event_listener.add_workspace_destroy_handler({
                let output = output.clone();
                move |_| {
                    let mut output = output.borrow_mut();
                    output
                        .try_send(Message::WorkspacesChanged(get_workspaces()))
                        .unwrap();
                }
            });

            event_listener.add_workspace_moved_handler({
                let output = output.clone();
                move |_| {
                    let mut output = output.borrow_mut();
                    output
                        .try_send(Message::WorkspacesChanged(get_workspaces()))
                        .unwrap();
                }
            });

            event_listener.add_window_close_handler({
                let output = output.clone();
                move |_| {
                    let mut output = output.borrow_mut();
                    output
                        .try_send(Message::WorkspacesChanged(get_workspaces()))
                        .unwrap();
                }
            });

            event_listener.add_window_open_handler({
                let output = output.clone();
                move |_| {
                    let mut output = output.borrow_mut();
                    output
                        .try_send(Message::WorkspacesChanged(get_workspaces()))
                        .unwrap();
                }
            });

            event_listener.add_window_moved_handler({
                let output = output.clone();
                move |_| {
                    let mut output = output.borrow_mut();
                    output
                        .try_send(Message::WorkspacesChanged(get_workspaces()))
                        .unwrap();
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
