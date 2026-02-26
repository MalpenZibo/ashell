use crate::{
    config::{WorkspaceIndicatorFormat, WorkspaceVisibilityMode, WorkspacesModuleConfig},
    outputs::Outputs,
    services::{
        ReadOnlyService, Service, ServiceEvent,
        compositor::{CompositorCommand, CompositorService, CompositorState},
        xdg_icons::{self, XdgIcon},
    },
    theme::AshellTheme,
};
use iced::{
    Element, Font, Length, Subscription, alignment,
    widget::{Image, MouseArea, Row, Svg, button, container, text},
    window::Id,
};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Displayed {
    Active,
    Visible,
    Hidden,
}

#[derive(Debug, Clone)]
pub struct UiWorkspace {
    pub id: i32,
    pub index: i32,
    pub name: String,
    pub monitor_id: Option<i128>,
    pub monitor: String,
    pub displayed: Displayed,
    pub windows: u16,
    pub icons: Vec<XdgIcon>,
}

#[derive(Debug, Clone)]
struct VirtualDesktop {
    pub active: bool,
    pub windows: u16,
    pub window_classes: Vec<String>,
}

fn resolve_workspace_icons(
    window_classes: &[String],
    config: &WorkspacesModuleConfig,
) -> Vec<XdgIcon> {
    if config.indicator_format != WorkspaceIndicatorFormat::NameAndIcons {
        return Vec::new();
    }

    let mut icons = Vec::new();
    let mut seen = HashSet::new();

    for class in window_classes {
        let class_lower = class.to_lowercase();
        if !seen.insert(class_lower.clone()) {
            continue;
        }
        icons.push(
            xdg_icons::get_icon_from_name(&class_lower).unwrap_or_else(xdg_icons::fallback_icon),
        );
    }

    icons
}

#[derive(Debug, Clone)]
pub enum Message {
    ServiceEvent(ServiceEvent<CompositorService>),
    ChangeWorkspace(i32),
    ToggleSpecialWorkspace(i32),
    Scroll(i32),
    ConfigReloaded(WorkspacesModuleConfig),
    ScrollAccumulator(f32),
}

pub struct Workspaces {
    config: WorkspacesModuleConfig,
    service: Option<CompositorService>,
    ui_workspaces: Vec<UiWorkspace>,
    scroll_accumulator: f32,
}

fn calculate_ui_workspaces(
    config: &WorkspacesModuleConfig,
    state: &CompositorState,
) -> Vec<UiWorkspace> {
    let active_id = state.active_workspace_id;
    let monitors = &state.monitors;
    let monitor_order = monitors
        .iter()
        .enumerate()
        .map(|(idx, monitor)| (monitor.name.clone(), idx))
        .collect::<HashMap<_, _>>();

    let workspaces = state
        .workspaces
        .clone()
        .into_iter()
        .unique_by(|w| w.id)
        .collect_vec();

    let mut result: Vec<UiWorkspace> = Vec::with_capacity(workspaces.len());
    let (special, normal): (Vec<_>, Vec<_>) = workspaces.into_iter().partition(|w| w.id < 0);

    // map special workspaces
    if !config.disable_special_workspaces {
        for w in special.iter() {
            // Special workspaces are active if they are assigned to any monitor.
            // Currently a special and normal workspace can be active at the same time on the same monitor.
            let active = monitors.iter().any(|m| m.special_workspace_id == w.id);
            result.push(UiWorkspace {
                id: w.id,
                index: w.index,
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
                icons: resolve_workspace_icons(&w.window_classes, config),
            });
        }
    }

    if config.enable_virtual_desktops {
        let monitor_count = monitors.len().max(1);
        let mut virtual_desktops: HashMap<i32, VirtualDesktop> = HashMap::new();

        for w in normal.iter() {
            let vdesk_id = ((w.id - 1) / monitor_count as i32) + 1;
            let is_active = Some(w.id) == active_id;

            if let Some(vdesk) = virtual_desktops.get_mut(&vdesk_id) {
                vdesk.windows += w.windows;
                vdesk.active = vdesk.active || is_active;
                vdesk.window_classes.extend(w.window_classes.clone());
            } else {
                virtual_desktops.insert(
                    vdesk_id,
                    VirtualDesktop {
                        active: is_active,
                        windows: w.windows,
                        window_classes: w.window_classes.clone(),
                    },
                );
            }
        }

        virtual_desktops.into_iter().for_each(|(id, vdesk)| {
            let idx = (id - 1) as usize;
            let display_name = config
                .workspace_names
                .get(idx)
                .cloned()
                .unwrap_or_else(|| id.to_string());

            result.push(UiWorkspace {
                id,
                index: id,
                name: display_name,
                monitor_id: None,
                monitor: "".to_string(),
                displayed: if vdesk.active {
                    Displayed::Active
                } else {
                    Displayed::Hidden
                },
                windows: vdesk.windows,
                icons: resolve_workspace_icons(&vdesk.window_classes, config),
            });
        });
    } else {
        for w in normal.iter() {
            let display_name = if w.id > 0 {
                let idx = (w.id - 1) as usize;
                config
                    .workspace_names
                    .get(idx)
                    .cloned()
                    .or_else(|| Some(w.name.clone()))
                    .unwrap_or_else(|| w.id.to_string())
            } else {
                w.name.clone()
            };

            let is_active = active_id == Some(w.id);
            let is_visible = monitors.iter().any(|m| m.active_workspace_id == w.id);

            result.push(UiWorkspace {
                id: w.id,
                index: w.index,
                name: display_name,
                monitor_id: w.monitor_id,
                monitor: w.monitor.clone(),
                displayed: match (is_active, is_visible) {
                    (true, _) => Displayed::Active,
                    (false, true) => Displayed::Visible,
                    (false, false) => Displayed::Hidden,
                },
                windows: w.windows,
                icons: resolve_workspace_icons(&w.window_classes, config),
            });
        }
    }

    if config.enable_workspace_filling && !result.is_empty() {
        let existing_ids = result.iter().map(|w| w.id).collect_vec();
        let mut max_id = *existing_ids
            .iter()
            .filter(|&&id| id > 0)
            .max()
            .unwrap_or(&0);

        if let Some(max_cfg) = config.max_workspaces
            && max_cfg > max_id as u32
        {
            max_id = max_cfg as i32;
        }

        let missing_ids: Vec<i32> = (1..=max_id)
            .filter(|id| !existing_ids.contains(id))
            .collect();

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

            result.push(UiWorkspace {
                id,
                index: id,
                name: display_name,
                monitor_id: None,
                monitor: "".to_string(),
                displayed: Displayed::Hidden,
                windows: 0,
                icons: Vec::new(),
            });
        }
    }

    if config.group_by_monitor {
        result.sort_by(|a, b| {
            let a_order = monitor_order.get(&a.monitor).copied().unwrap_or(usize::MAX);
            let b_order = monitor_order.get(&b.monitor).copied().unwrap_or(usize::MAX);

            a_order
                .cmp(&b_order)
                .then(a.index.cmp(&b.index))
                .then(a.id.cmp(&b.id))
        });
    } else {
        result.sort_by(|a, b| a.index.cmp(&b.index).then(a.id.cmp(&b.id)));
    }

    result
}

fn needs_window_classes(config: &WorkspacesModuleConfig) -> bool {
    config.indicator_format == WorkspaceIndicatorFormat::NameAndIcons
}

impl Workspaces {
    pub fn new(config: WorkspacesModuleConfig) -> Self {
        crate::services::compositor::set_collect_window_classes(needs_window_classes(&config));
        Self {
            config,
            service: None,
            ui_workspaces: Vec::new(),
            scroll_accumulator: 0.,
        }
    }

    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::ServiceEvent(event) => {
                match event {
                    ServiceEvent::Init(s) => {
                        self.service = Some(s);
                        self.recalculate_ui_workspaces();
                    }
                    ServiceEvent::Update(e) => {
                        if let Some(s) = &mut self.service {
                            s.update(e);
                            self.recalculate_ui_workspaces();
                        }
                    }
                    _ => {}
                }
                iced::Task::none()
            }
            Message::ChangeWorkspace(id) => {
                if let Some(service) = &mut self.service {
                    let already_active = self
                        .ui_workspaces
                        .iter()
                        .any(|w| w.displayed == Displayed::Active && w.id == id);

                    if !already_active {
                        if self.config.enable_virtual_desktops {
                            return service
                                .command(CompositorCommand::CustomDispatch(
                                    "vdesk".to_string(),
                                    id.to_string(),
                                ))
                                .map(Message::ServiceEvent);
                        } else {
                            return service
                                .command(CompositorCommand::FocusWorkspace(id))
                                .map(Message::ServiceEvent);
                        }
                    }
                }
                iced::Task::none()
            }
            Message::ToggleSpecialWorkspace(id) => {
                if let Some(service) = &mut self.service
                    && let Some(special) = service.workspaces.iter().find(|w| w.id == id)
                {
                    return service
                        .command(CompositorCommand::ToggleSpecialWorkspace(
                            special
                                .name
                                .split(":")
                                .last()
                                .map_or_else(|| special.name.clone(), |s| s.to_string()),
                        ))
                        .map(Message::ServiceEvent);
                }
                iced::Task::none()
            }
            Message::Scroll(direction) => {
                self.scroll_accumulator = 0.;

                /* TODO: should we use the native service implementation instead?
                if let Some(service) = &mut self.service {
                    return service
                        .command(CompositorCommand::ScrollWorkspace(direction))
                        .map(Message::ServiceEvent);
                }
                return iced::Task::none();*/
                let current_workspace = self
                    .ui_workspaces
                    .iter()
                    .find(|w| w.displayed == Displayed::Active);

                let Some(current_id) = current_workspace.map(|w| w.id) else {
                    return iced::Task::none();
                };

                let next_workspace = if direction > 0 {
                    self.ui_workspaces
                        .iter()
                        .filter(|w| w.id < current_id)
                        .max_by_key(|w| w.id)
                } else {
                    self.ui_workspaces
                        .iter()
                        .filter(|w| w.id > current_id)
                        .min_by_key(|w| w.id)
                };

                if let Some(next) = next_workspace {
                    return self.update(Message::ChangeWorkspace(next.id));
                }
                iced::Task::none()
            }
            Message::ConfigReloaded(cfg) => {
                crate::services::compositor::set_collect_window_classes(needs_window_classes(&cfg));
                self.config = cfg;
                self.recalculate_ui_workspaces();
                iced::Task::none()
            }
            Message::ScrollAccumulator(value) => {
                if value == 0. {
                    self.scroll_accumulator = 0.;
                } else {
                    self.scroll_accumulator += value;
                }

                iced::Task::none()
            }
        }
    }

    fn recalculate_ui_workspaces(&mut self) {
        if let Some(service) = &self.service {
            self.ui_workspaces = calculate_ui_workspaces(&self.config, service);
        }
    }

    pub fn view<'a>(
        &'a self,
        id: Id,
        theme: &'a AshellTheme,
        outputs: &Outputs,
    ) -> Element<'a, Message> {
        let monitor_name = outputs.get_monitor_name(id);

        MouseArea::new(
            Row::with_children(
                self.ui_workspaces
                    .iter()
                    .filter_map(|w| {
                        let show = match self.config.visibility_mode {
                            WorkspaceVisibilityMode::All => true,
                            WorkspaceVisibilityMode::MonitorSpecific => {
                                monitor_name
                                    .unwrap_or_else(|| &w.monitor)
                                    .contains(&w.monitor)
                                    || !outputs.has_name(&w.monitor)
                            }
                            WorkspaceVisibilityMode::MonitorSpecificExclusive => monitor_name
                                .unwrap_or_else(|| &w.monitor)
                                .contains(&w.monitor),
                        };

                        if show {
                            let empty = w.windows == 0;
                            let color_index = if self.config.enable_virtual_desktops {
                                Some(w.id as i128)
                            } else {
                                w.monitor_id
                            };

                            let is_active = w.displayed == Displayed::Active;

                            let color = color_index.map(|i| {
                                let i = i as usize;
                                let colors = if is_active {
                                    theme
                                        .active_workspace_colors
                                        .as_ref()
                                        .unwrap_or(&theme.workspace_colors)
                                } else if w.id < 0 {
                                    theme
                                        .special_workspace_colors
                                        .as_ref()
                                        .unwrap_or(&theme.workspace_colors)
                                } else {
                                    &theme.workspace_colors
                                };
                                colors.get(i).copied()
                            });

                            let has_icons = !w.icons.is_empty();
                            let dynamic_width = w.id < 0 || has_icons;

                            let content: Element<'a, Message> = if has_icons {
                                let mut children: Vec<Element<'a, Message>> =
                                    vec![text(w.name.as_str()).size(theme.font_size.xs).into()];
                                children.extend(w.icons.iter().map(|i| {
                                    match i {
                                        XdgIcon::Svg(handle) => Svg::new(handle.clone())
                                            .height(Length::Fixed(theme.font_size.xs as f32))
                                            .width(Length::Shrink)
                                            .into(),
                                        XdgIcon::Image(handle) => Image::new(handle.clone())
                                            .height(Length::Fixed(theme.font_size.xs as f32))
                                            .width(Length::Shrink)
                                            .into(),
                                        XdgIcon::NerdFont(glyph) => text(*glyph)
                                            .size(theme.font_size.xs)
                                            .font(Font::with_name("Symbols Nerd Font"))
                                            .into(),
                                    }
                                }));
                                Row::with_children(children)
                                    .spacing(theme.space.xxs)
                                    .align_y(alignment::Vertical::Center)
                                    .into()
                            } else {
                                container(text(w.name.as_str()).size(theme.font_size.xs))
                                    .align_x(alignment::Horizontal::Center)
                                    .align_y(alignment::Vertical::Center)
                                    .into()
                            };

                            Some(
                                button(content)
                                    .style(theme.workspace_button_style(empty, color))
                                    .padding(if dynamic_width {
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
                                    .width(if dynamic_width {
                                        Length::Shrink
                                    } else {
                                        match w.displayed {
                                            Displayed::Active => {
                                                Length::Fixed(theme.space.xl as f32)
                                            }
                                            Displayed::Visible => {
                                                Length::Fixed(theme.space.lg as f32)
                                            }
                                            Displayed::Hidden => {
                                                Length::Fixed(theme.space.md as f32)
                                            }
                                        }
                                    })
                                    .height(theme.space.md)
                                    .into(),
                            )
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>(),
            )
            .spacing(theme.space.xxs),
        )
        .on_scroll(move |direction| match direction {
            iced::mouse::ScrollDelta::Lines { y, .. } => {
                if y < 0. {
                    Message::Scroll(-1)
                } else {
                    Message::Scroll(1)
                }
            }
            iced::mouse::ScrollDelta::Pixels { y, .. } => {
                let sensibility = 3.;

                if self.scroll_accumulator.abs() < sensibility {
                    Message::ScrollAccumulator(y)
                } else if self.scroll_accumulator.is_sign_positive() {
                    Message::Scroll(-1)
                } else {
                    Message::Scroll(1)
                }
            }
        })
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        CompositorService::subscribe().map(Message::ServiceEvent)
    }
}
