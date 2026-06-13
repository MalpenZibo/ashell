use crate::{
    config::{
        AppearanceColor, InvertScrollDirection, WorkspaceVisibilityMode, WorkspacesModuleConfig,
    },
    outputs::Outputs,
    services::{
        ReadOnlyService, Service, ServiceEvent,
        compositor::{CompositorCommand, CompositorService, CompositorState},
    },
    theme::{AshellTheme, use_theme},
};
use iced::{
    Element, Length, Subscription, SurfaceId, alignment,
    widget::{MouseArea, Row, button, container, text},
};
use iced_anim::{AnimationBuilder, transition::Easing};
use itertools::Itertools;
use std::collections::HashMap;

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
    pub has_urgent: bool,
}

#[derive(Debug, Clone)]
struct VirtualDesktop {
    pub active: bool,
    pub windows: u16,
    pub has_urgent: bool,
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
                has_urgent: w.has_urgent,
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
                vdesk.has_urgent = vdesk.has_urgent || w.has_urgent;
            } else {
                virtual_desktops.insert(
                    vdesk_id,
                    VirtualDesktop {
                        active: is_active,
                        windows: w.windows,
                        has_urgent: w.has_urgent,
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
                has_urgent: vdesk.has_urgent,
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
                has_urgent: w.has_urgent,
            });
        }
    }

    if config.enable_workspace_filling && !result.is_empty() {
        let existing_indices = result.iter().map(|w| w.index).collect_vec();
        let mut max_index = *existing_indices
            .iter()
            .filter(|&&idx| idx > 0)
            .max()
            .unwrap_or(&0);

        if let Some(max_cfg) = config.max_workspaces
            && max_cfg > max_index as u32
        {
            max_index = max_cfg as i32;
        }

        let missing_indices: Vec<i32> = (1..=max_index)
            .filter(|idx| !existing_indices.contains(idx))
            .collect();

        for index in missing_indices {
            let display_name = if index > 0 {
                let name_idx = (index - 1) as usize;
                config
                    .workspace_names
                    .get(name_idx)
                    .cloned()
                    .unwrap_or_else(|| index.to_string())
            } else {
                index.to_string()
            };

            result.push(UiWorkspace {
                id: index,
                index,
                name: display_name,
                monitor_id: None,
                monitor: "".to_string(),
                displayed: Displayed::Hidden,
                windows: 0,
                has_urgent: false,
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

#[allow(clippy::too_many_arguments)]
fn workspace_button<'a>(
    theme: &AshellTheme,
    name: String,
    font_size: f32,
    empty: bool,
    urgent: bool,
    color: Option<Option<AppearanceColor>>,
    width: Length,
    padding: f32,
    height: f32,
    on_press: Message,
) -> Element<'a, Message> {
    button(
        container(text(name).size(font_size))
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center),
    )
    .style(theme.workspace_button_style(empty, urgent, color))
    .padding([0.0, padding])
    .on_press(on_press)
    .width(width)
    .height(height)
    .into()
}

impl Workspaces {
    pub fn new(config: WorkspacesModuleConfig) -> Self {
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

                // TODO: consider using the compositor service for scroll instead
                let Some(pos) = self
                    .ui_workspaces
                    .iter()
                    .position(|w| w.displayed == Displayed::Active)
                else {
                    return iced::Task::none();
                };

                let current_monitor = self.ui_workspaces[pos].monitor.clone();
                let current_monitor_id = self.ui_workspaces[pos].monitor_id;

                let restrict_to_monitor = matches!(
                    self.config.visibility_mode,
                    WorkspaceVisibilityMode::MonitorSpecific
                        | WorkspaceVisibilityMode::MonitorSpecificExclusive
                );

                let in_current_group = |w: &&UiWorkspace| -> bool {
                    if !restrict_to_monitor {
                        return true;
                    }

                    if let Some(w_monitor_id) = w.monitor_id
                        && let Some(active_monitor_id) = current_monitor_id
                    {
                        return w_monitor_id == active_monitor_id;
                    }

                    if !w.monitor.is_empty() && !current_monitor.is_empty() {
                        return w.monitor == current_monitor;
                    }

                    // monitor doesn't seem to contain any useful info, so assume it's part of the group
                    true
                };

                // Navigate by position in the already-sorted ui_workspaces
                // vector, which represents exact visual order regardless of
                // group_by_monitor or visibility_mode configuration.
                let next_workspace = if direction > 0 {
                    self.ui_workspaces[..pos]
                        .iter()
                        .rev()
                        .find(|w| in_current_group(w))
                } else {
                    self.ui_workspaces[pos + 1..]
                        .iter()
                        .find(|w| in_current_group(w))
                };

                if let Some(next) = next_workspace {
                    return self.update(Message::ChangeWorkspace(next.id));
                }
                iced::Task::none()
            }
            Message::ConfigReloaded(cfg) => {
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

    pub fn view<'a>(&'a self, id: SurfaceId, outputs: &Outputs) -> Element<'a, Message> {
        let monitor_name = outputs.get_monitor_name(id);

        let row = use_theme(|theme| {
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
                            let urgent = w.has_urgent;
                            let color_index = if self.config.enable_virtual_desktops {
                                Some(w.id as i128)
                            } else {
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

                            {
                                let name = w.name.clone();
                                let on_press = if w.id > 0 {
                                    Message::ChangeWorkspace(w.id)
                                } else {
                                    Message::ToggleSpecialWorkspace(w.id)
                                };
                                let font_size = theme.font_size.xs;
                                let height = theme.space.md;

                                let numbered = w.id > 0
                                    && !w.name.is_empty()
                                    && w.name.chars().all(|c| c.is_ascii_digit());

                                Some(if numbered {
                                    let target_width = match (&w.displayed, urgent) {
                                        (Displayed::Active, _) => theme.space.xl,
                                        (Displayed::Visible, _) | (Displayed::Hidden, true) => {
                                            theme.space.lg
                                        }
                                        (Displayed::Hidden, false) => theme.space.md,
                                    };

                                    if theme.animations_enabled {
                                        AnimationBuilder::new(target_width, move |width| {
                                            use_theme(|theme| {
                                                workspace_button(
                                                    theme,
                                                    name.clone(),
                                                    font_size,
                                                    empty,
                                                    urgent,
                                                    color,
                                                    Length::Fixed(width),
                                                    0.0,
                                                    height,
                                                    on_press.clone(),
                                                )
                                            })
                                        })
                                        .animates_layout(true)
                                        .animation(Easing::EASE.very_quick())
                                        .into()
                                    } else {
                                        workspace_button(
                                            theme,
                                            name,
                                            font_size,
                                            empty,
                                            urgent,
                                            color,
                                            Length::Fixed(target_width),
                                            0.0,
                                            height,
                                            on_press,
                                        )
                                    }
                                } else {
                                    let target_padding = match (&w.displayed, urgent) {
                                        (Displayed::Active, _) => theme.space.md,
                                        (Displayed::Visible, _) | (Displayed::Hidden, true) => {
                                            theme.space.sm
                                        }
                                        (Displayed::Hidden, false) => theme.space.xs,
                                    };

                                    if theme.animations_enabled {
                                        AnimationBuilder::new(target_padding, move |padding| {
                                            use_theme(|theme| {
                                                workspace_button(
                                                    theme,
                                                    name.clone(),
                                                    font_size,
                                                    empty,
                                                    urgent,
                                                    color,
                                                    Length::Shrink,
                                                    padding,
                                                    height,
                                                    on_press.clone(),
                                                )
                                            })
                                        })
                                        .animates_layout(true)
                                        .animation(Easing::EASE.very_quick())
                                        .into()
                                    } else {
                                        workspace_button(
                                            theme,
                                            name,
                                            font_size,
                                            empty,
                                            urgent,
                                            color,
                                            Length::Shrink,
                                            target_padding,
                                            height,
                                            on_press,
                                        )
                                    }
                                })
                            }
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>(),
            )
            .spacing(theme.space.xxs)
        });

        MouseArea::new(row)
            .on_scroll(move |direction| match direction {
                iced::mouse::ScrollDelta::Lines { y, .. } => {
                    if y.is_sign_positive() {
                        match self.config.invert_scroll_direction {
                            Some(InvertScrollDirection::All | InvertScrollDirection::Mouse) => {
                                Message::Scroll(-1)
                            }
                            Some(InvertScrollDirection::Trackpad) => Message::Scroll(1),
                            None => Message::Scroll(1),
                        }
                    } else {
                        match self.config.invert_scroll_direction {
                            Some(InvertScrollDirection::All | InvertScrollDirection::Mouse) => {
                                Message::Scroll(1)
                            }
                            Some(InvertScrollDirection::Trackpad) => Message::Scroll(-1),
                            None => Message::Scroll(-1),
                        }
                    }
                }
                iced::mouse::ScrollDelta::Pixels { y, .. } => {
                    let sensibility = 3.;

                    if self.scroll_accumulator.abs() < sensibility {
                        Message::ScrollAccumulator(y)
                    } else if self.scroll_accumulator.is_sign_positive() {
                        match self.config.invert_scroll_direction {
                            Some(InvertScrollDirection::All | InvertScrollDirection::Trackpad) => {
                                Message::Scroll(-1)
                            }
                            Some(InvertScrollDirection::Mouse) => Message::Scroll(1),
                            None => Message::Scroll(1),
                        }
                    } else {
                        match self.config.invert_scroll_direction {
                            Some(InvertScrollDirection::All | InvertScrollDirection::Trackpad) => {
                                Message::Scroll(1)
                            }
                            Some(InvertScrollDirection::Mouse) => Message::Scroll(-1),
                            None => Message::Scroll(-1),
                        }
                    }
                }
            })
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        CompositorService::subscribe().map(Message::ServiceEvent)
    }
}
