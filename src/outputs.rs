use iced::{
    Task,
    platform_specific::shell::commands::layer_surface::{
        Anchor, KeyboardInteractivity, Layer, destroy_layer_surface, get_layer_surface, set_anchor,
        set_exclusive_zone, set_size,
    },
    runtime::platform_specific::wayland::layer_surface::{IcedOutput, SctkLayerSurfaceSettings},
    window::Id,
};
use log::debug;
use wayland_client::protocol::wl_output::WlOutput;

use crate::{
    HEIGHT,
    config::{self, Position},
    menu::{Menu, MenuType},
    position_button::ButtonUIRef,
};

#[derive(Debug, Clone)]
struct ShellInfo {
    id: Id,
    position: Position,
    solid_style: bool,
    menu: Menu,
}

#[derive(Debug, Clone)]
pub struct Outputs(Vec<(Option<String>, Option<ShellInfo>, Option<WlOutput>)>);

pub enum HasOutput<'a> {
    Main,
    Menu(Option<&'a (MenuType, ButtonUIRef)>),
}

impl Outputs {
    pub fn new<Message: 'static>(solid_style: bool, position: Position) -> (Self, Task<Message>) {
        let (id, menu_id, task) = Self::create_output_layers(solid_style, None, position);

        (
            Self(vec![(
                None,
                Some(ShellInfo {
                    id,
                    menu: Menu::new(menu_id),
                    position,
                    solid_style,
                }),
                None,
            )]),
            task,
        )
    }

    fn get_height(solid_style: bool) -> u32 {
        HEIGHT - if solid_style { 8 } else { 0 }
    }

    fn create_output_layers<Message: 'static>(
        solid_style: bool,
        wl_output: Option<WlOutput>,
        position: Position,
    ) -> (Id, Id, Task<Message>) {
        let id = Id::unique();
        let height = Self::get_height(solid_style);

        let task = get_layer_surface(SctkLayerSurfaceSettings {
            id,
            namespace: "ashell".to_string(),
            size: Some((None, Some(height))),
            layer: Layer::Bottom,
            pointer_interactivity: true,
            keyboard_interactivity: KeyboardInteractivity::None,
            exclusive_zone: height as i32,
            output: wl_output.clone().map_or(IcedOutput::Active, |wl_output| {
                IcedOutput::Output(wl_output)
            }),
            anchor: match position {
                Position::Top => Anchor::TOP,
                Position::Bottom => Anchor::BOTTOM,
            } | Anchor::LEFT
                | Anchor::RIGHT,
            ..Default::default()
        });

        let menu_id = Id::unique();
        let menu_task = get_layer_surface(SctkLayerSurfaceSettings {
            id: menu_id,
            namespace: "ashell".to_string(),
            size: Some((None, None)),
            layer: Layer::Background,
            pointer_interactivity: true,
            keyboard_interactivity: KeyboardInteractivity::None,
            output: wl_output.map_or(IcedOutput::Active, |wl_output| {
                IcedOutput::Output(wl_output)
            }),
            anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
            ..Default::default()
        });

        (id, menu_id, Task::batch(vec![task, menu_task]))
    }

    fn name_in_config(name: Option<&str>, outputs: &config::Outputs) -> bool {
        match outputs {
            config::Outputs::All => true,
            config::Outputs::Active => false,
            config::Outputs::Targets(request_outputs) => request_outputs
                .iter()
                .any(|output| Some(output.as_str()) == name),
        }
    }

    pub fn has(&self, id: Id) -> Option<HasOutput> {
        self.0.iter().find_map(|(_, info, _)| {
            if let Some(info) = info {
                if info.id == id {
                    Some(HasOutput::Main)
                } else if info.menu.id == id {
                    Some(HasOutput::Menu(info.menu.menu_info.as_ref()))
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    pub fn get_monitor_name(&self, id: Id) -> Option<&str> {
        self.0.iter().find_map(|(name, info, _)| {
            if let Some(info) = info {
                if info.id == id {
                    name.as_ref().map(|n| n.as_str())
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    pub fn has_name(&self, name: &str) -> bool {
        self.0
            .iter()
            .any(|(n, info, _)| info.is_some() && n.as_ref().map(|n| n.as_str()) == Some(name))
    }

    pub fn add<Message: 'static>(
        &mut self,
        solid_style: bool,
        request_outputs: &config::Outputs,
        position: Position,
        name: &str,
        wl_output: WlOutput,
    ) -> Task<Message> {
        let target = Self::name_in_config(Some(name), request_outputs);

        if target {
            debug!("Found target output, creating a new layer surface");

            let (id, menu_id, task) =
                Self::create_output_layers(solid_style, Some(wl_output.clone()), position);

            let destroy_task = match self
                .0
                .iter()
                .position(|(key, _, _)| key.as_ref().map(|k| k.as_str()) == Some(name))
            {
                Some(index) => {
                    let old_output = self.0.swap_remove(index);

                    match old_output.1 {
                        Some(shell_info) => {
                            let destroy_main_task = destroy_layer_surface(shell_info.id);
                            let destroy_menu_task = destroy_layer_surface(shell_info.menu.id);

                            Task::batch(vec![destroy_main_task, destroy_menu_task])
                        }
                        _ => Task::none(),
                    }
                }
                _ => Task::none(),
            };

            self.0.push((
                Some(name.to_owned()),
                Some(ShellInfo {
                    id,
                    menu: Menu::new(menu_id),
                    position,
                    solid_style,
                }),
                Some(wl_output),
            ));

            // remove fallback layer surface
            let destroy_fallback_task = match self.0.iter().position(|(key, _, _)| key.is_none()) {
                Some(index) => {
                    let old_output = self.0.swap_remove(index);

                    match old_output.1 {
                        Some(shell_info) => {
                            let destroy_fallback_main_task = destroy_layer_surface(shell_info.id);
                            let destroy_fallback_menu_task =
                                destroy_layer_surface(shell_info.menu.id);

                            Task::batch(vec![
                                destroy_fallback_main_task,
                                destroy_fallback_menu_task,
                            ])
                        }
                        _ => Task::none(),
                    }
                }
                _ => Task::none(),
            };

            Task::batch(vec![destroy_task, destroy_fallback_task, task])
        } else {
            self.0.push((Some(name.to_owned()), None, Some(wl_output)));

            Task::none()
        }
    }

    pub fn remove<Message: 'static>(
        &mut self,
        solid_style: bool,
        position: Position,
        wl_output: WlOutput,
    ) -> Task<Message> {
        match self.0.iter().position(|(_, _, assigned_wl_output)| {
            assigned_wl_output
                .as_ref()
                .map(|assigned_wl_output| *assigned_wl_output == wl_output)
                .unwrap_or_default()
        }) {
            Some(index_to_remove) => {
                debug!("Removing layer surface for output");

                let (name, shell_info, wl_output) = self.0.swap_remove(index_to_remove);

                let destroy_task = if let Some(shell_info) = shell_info {
                    let destroy_main_task = destroy_layer_surface(shell_info.id);
                    let destroy_menu_task = destroy_layer_surface(shell_info.menu.id);

                    Task::batch(vec![destroy_main_task, destroy_menu_task])
                } else {
                    Task::none()
                };

                self.0.push((name.to_owned(), None, wl_output));

                if !self.0.iter().any(|(_, shell_info, _)| shell_info.is_some()) {
                    debug!("No outputs left, creating a fallback layer surface");

                    let (id, menu_id, task) =
                        Self::create_output_layers(solid_style, None, position);

                    self.0.push((
                        None,
                        Some(ShellInfo {
                            id,
                            menu: Menu::new(menu_id),
                            position,
                            solid_style,
                        }),
                        None,
                    ));

                    Task::batch(vec![destroy_task, task])
                } else {
                    Task::batch(vec![destroy_task])
                }
            }
            _ => Task::none(),
        }
    }

    pub fn sync<Message: 'static>(
        &mut self,
        solid_style: bool,
        request_outputs: &config::Outputs,
        position: Position,
    ) -> Task<Message> {
        debug!(
            "Syncing outputs: {:?}, request_outputs: {:?}",
            self, request_outputs
        );

        let to_remove = self
            .0
            .iter()
            .filter_map(|(name, shell_info, wl_output)| {
                if !Self::name_in_config(name.as_ref().map(|n| n.as_str()), request_outputs)
                    && shell_info.is_some()
                {
                    Some(wl_output.clone())
                } else {
                    None
                }
            })
            .flatten()
            .collect::<Vec<_>>();
        debug!("Removing outputs: {:?}", to_remove);

        let to_add = self
            .0
            .iter()
            .filter_map(|(name, shell_info, wl_output)| {
                if Self::name_in_config(name.as_ref().map(|n| n.as_str()), request_outputs)
                    && shell_info.is_none()
                {
                    Some((name.clone(), wl_output.clone()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        debug!("Adding outputs: {:?}", to_add);

        let mut tasks = Vec::new();

        for (name, wl_output) in to_add {
            if let Some(wl_output) = wl_output {
                if let Some(name) = name {
                    tasks.push(self.add(
                        solid_style,
                        request_outputs,
                        position,
                        name.as_str(),
                        wl_output,
                    ));
                }
            }
        }

        for wl_output in to_remove {
            tasks.push(self.remove(solid_style, position, wl_output));
        }

        for shell_info in self.0.iter_mut().filter_map(|(_, shell_info, _)| {
            if let Some(shell_info) = shell_info {
                if shell_info.position != position {
                    Some(shell_info)
                } else {
                    None
                }
            } else {
                None
            }
        }) {
            debug!(
                "Repositioning output: {:?}, new position {:?}",
                shell_info.id, position
            );
            shell_info.position = position;
            tasks.push(set_anchor(
                shell_info.id,
                match position {
                    Position::Top => Anchor::TOP,
                    Position::Bottom => Anchor::BOTTOM,
                } | Anchor::LEFT
                    | Anchor::RIGHT,
            ));
        }

        for shell_info in self.0.iter_mut().filter_map(|(_, shell_info, _)| {
            if let Some(shell_info) = shell_info {
                if shell_info.solid_style != solid_style {
                    Some(shell_info)
                } else {
                    None
                }
            } else {
                None
            }
        }) {
            debug!(
                "Change style for output: {:?}, new style {:?}",
                shell_info.id, solid_style
            );
            shell_info.solid_style = solid_style;
            let height = Self::get_height(solid_style);
            tasks.push(Task::batch(vec![
                set_size(shell_info.id, None, Some(height)),
                set_exclusive_zone(shell_info.id, height as i32),
            ]));
        }

        Task::batch(tasks)
    }

    pub fn toggle_menu<Message: 'static>(
        &mut self,
        id: Id,
        menu_type: MenuType,
        button_ui_ref: ButtonUIRef,
    ) -> Task<Message> {
        match self.0.iter_mut().find(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell_info| shell_info.id) == Some(id)
                || shell_info.as_ref().map(|shell_info| shell_info.menu.id) == Some(id)
        }) {
            Some((_, Some(shell_info), _)) => {
                let toggle_task = shell_info.menu.toggle(menu_type, button_ui_ref);
                let mut tasks = self
                    .0
                    .iter_mut()
                    .filter_map(|(_, shell_info, _)| {
                        if let Some(shell_info) = shell_info {
                            if shell_info.id != id && shell_info.menu.id != id {
                                Some(shell_info.menu.close())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                tasks.push(toggle_task);
                Task::batch(tasks)
            }
            _ => Task::none(),
        }
    }

    pub fn close_menu<Message: 'static>(&mut self, id: Id) -> Task<Message> {
        match self.0.iter_mut().find(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell_info| shell_info.id) == Some(id)
                || shell_info.as_ref().map(|shell_info| shell_info.menu.id) == Some(id)
        }) {
            Some((_, Some(shell_info), _)) => shell_info.menu.close(),
            _ => Task::none(),
        }
    }

    pub fn close_menu_if<Message: 'static>(
        &mut self,
        id: Id,
        menu_type: MenuType,
    ) -> Task<Message> {
        match self.0.iter_mut().find(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell_info| shell_info.id) == Some(id)
                || shell_info.as_ref().map(|shell_info| shell_info.menu.id) == Some(id)
        }) {
            Some((_, Some(shell_info), _)) => shell_info.menu.close_if(menu_type),
            _ => Task::none(),
        }
    }

    pub fn close_all_menu_if<Message: 'static>(&mut self, menu_type: MenuType) -> Task<Message> {
        Task::batch(
            self.0
                .iter_mut()
                .map(|(_, shell_info, _)| {
                    if let Some(shell_info) = shell_info {
                        shell_info.menu.close_if(menu_type.clone())
                    } else {
                        Task::none()
                    }
                })
                .collect::<Vec<_>>(),
        )
    }

    pub fn request_keyboard<Message: 'static>(&self, id: Id) -> Task<Message> {
        match self.0.iter().find(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell_info| shell_info.id) == Some(id)
                || shell_info.as_ref().map(|shell_info| shell_info.menu.id) == Some(id)
        }) {
            Some((_, Some(shell_info), _)) => shell_info.menu.request_keyboard(),
            _ => Task::none(),
        }
    }

    pub fn release_keyboard<Message: 'static>(&self, id: Id) -> Task<Message> {
        match self.0.iter().find(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell_info| shell_info.id) == Some(id)
                || shell_info.as_ref().map(|shell_info| shell_info.menu.id) == Some(id)
        }) {
            Some((_, Some(shell_info), _)) => shell_info.menu.release_keyboard(),
            _ => Task::none(),
        }
    }
}
