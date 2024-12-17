use iced::{
    platform_specific::shell::commands::layer_surface::{
        destroy_layer_surface, get_layer_surface, set_anchor, Anchor, KeyboardInteractivity, Layer,
    },
    runtime::platform_specific::wayland::layer_surface::{IcedOutput, SctkLayerSurfaceSettings},
    window::Id,
    Task,
};
use log::debug;
use wayland_client::protocol::wl_output::WlOutput;

use crate::{
    config::Position,
    menu::{Menu, MenuType},
    HEIGHT,
};

static FALLBACK_LAYER: &str = "fallback";

#[derive(Debug, Clone)]
struct ShellInfo {
    id: Id,
    position: Position,
    menu: Menu,
}

#[derive(Debug, Clone)]
pub struct Outputs(Vec<(String, Option<ShellInfo>, Option<WlOutput>)>);

pub enum HasOutput<'a> {
    Main,
    Menu(&'a Option<MenuType>),
}

impl Outputs {
    pub fn new<Message: 'static>(position: Position) -> (Self, Task<Message>) {
        let (id, menu_id, task) = Self::create_output_layers(None, position);

        (
            Self(vec![(
                FALLBACK_LAYER.to_owned(),
                Some(ShellInfo {
                    id,
                    menu: Menu::new(menu_id),
                    position,
                }),
                None,
            )]),
            task,
        )
    }

    fn create_output_layers<Message: 'static>(
        wl_output: Option<WlOutput>,
        position: Position,
    ) -> (Id, Id, Task<Message>) {
        let id = Id::unique();
        let task = get_layer_surface(SctkLayerSurfaceSettings {
            id,
            size: Some((None, Some(HEIGHT))),
            layer: Layer::Bottom,
            pointer_interactivity: true,
            keyboard_interactivity: KeyboardInteractivity::None,
            exclusive_zone: HEIGHT as i32,
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

    pub fn has(&self, id: Id) -> Option<HasOutput> {
        self.0.iter().find_map(|(_, info, _)| {
            if let Some(info) = info {
                if info.id == id {
                    Some(HasOutput::Main)
                } else if info.menu.id == id {
                    Some(HasOutput::Menu(&info.menu.menu_type))
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    pub fn add<Message: 'static>(
        &mut self,
        request_outputs: &[String],
        position: Position,
        name: &str,
        wl_output: WlOutput,
    ) -> Task<Message> {
        let target = request_outputs.iter().any(|output| output.as_str() == name);

        if target {
            debug!("Found target output, creating a new layer surface");

            let (id, menu_id, task) = Self::create_output_layers(Some(wl_output.clone()), position);

            let destroy_task =
                if let Some(index) = self.0.iter().position(|(key, _, _)| key == name) {
                    let old_output = self.0.swap_remove(index);

                    if let Some(shell_info) = old_output.1 {
                        let destroy_main_task = destroy_layer_surface(shell_info.id);
                        let destroy_menu_task = destroy_layer_surface(shell_info.menu.id);

                        Task::batch(vec![destroy_main_task, destroy_menu_task])
                    } else {
                        Task::none()
                    }
                } else {
                    Task::none()
                };

            self.0.push((
                name.to_owned(),
                Some(ShellInfo {
                    id,
                    menu: Menu::new(menu_id),
                    position,
                }),
                Some(wl_output),
            ));

            // remove fallback layer surface
            let destroy_fallback_task =
                if let Some(index) = self.0.iter().position(|(key, _, _)| key == FALLBACK_LAYER) {
                    let old_output = self.0.swap_remove(index);

                    if let Some(shell_info) = old_output.1 {
                        let destroy_fallback_main_task = destroy_layer_surface(shell_info.id);
                        let destroy_fallback_menu_task = destroy_layer_surface(shell_info.menu.id);

                        Task::batch(vec![destroy_fallback_main_task, destroy_fallback_menu_task])
                    } else {
                        Task::none()
                    }
                } else {
                    Task::none()
                };

            Task::batch(vec![destroy_task, destroy_fallback_task, task])
        } else {
            self.0.push((name.to_owned(), None, Some(wl_output)));

            Task::none()
        }
    }

    pub fn remove<Message: 'static>(
        &mut self,
        position: Position,
        wl_output: WlOutput,
    ) -> Task<Message> {
        if let Some(index_to_remove) = self.0.iter().position(|(_, _, assigned_wl_output)| {
            assigned_wl_output
                .as_ref()
                .map(|assigned_wl_output| *assigned_wl_output == wl_output)
                .unwrap_or_default()
        }) {
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

                let (id, menu_id, task) = Self::create_output_layers(None, position);

                self.0.push((
                    FALLBACK_LAYER.to_owned(),
                    Some(ShellInfo {
                        id,
                        menu: Menu::new(menu_id),
                        position,
                    }),
                    None,
                ));

                Task::batch(vec![destroy_task, task])
            } else {
                Task::batch(vec![destroy_task])
            }
        } else {
            Task::none()
        }
    }

    pub fn sync<Message: 'static>(
        &mut self,
        request_outputs: &[String],
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
                if !request_outputs.iter().any(|output| output.as_str() == name)
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
                if request_outputs.iter().any(|output| output.as_str() == name)
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
                tasks.push(self.add(request_outputs, position, &name, wl_output));
            }
        }

        for wl_output in to_remove {
            tasks.push(self.remove(position, wl_output));
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

        Task::batch(tasks)
    }

    pub fn toggle_menu<Message: 'static>(&mut self, id: Id, menu_type: MenuType) -> Task<Message> {
        if let Some((_, Some(shell_info), _)) = self.0.iter_mut().find(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell_info| shell_info.id) == Some(id)
                || shell_info.as_ref().map(|shell_info| shell_info.menu.id) == Some(id)
        }) {
            let toggle_task = shell_info.menu.toggle(menu_type);
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
        } else {
            Task::none()
        }
    }

    pub fn close_menu<Message: 'static>(&mut self, id: Id) -> Task<Message> {
        if let Some((_, Some(shell_info), _)) = self.0.iter_mut().find(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell_info| shell_info.id) == Some(id)
                || shell_info.as_ref().map(|shell_info| shell_info.menu.id) == Some(id)
        }) {
            shell_info.menu.close()
        } else {
            Task::none()
        }
    }

    pub fn close_menu_if<Message: 'static>(
        &mut self,
        id: Id,
        menu_type: MenuType,
    ) -> Task<Message> {
        if let Some((_, Some(shell_info), _)) = self.0.iter_mut().find(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell_info| shell_info.id) == Some(id)
                || shell_info.as_ref().map(|shell_info| shell_info.menu.id) == Some(id)
        }) {
            shell_info.menu.close_if(menu_type)
        } else {
            Task::none()
        }
    }

    pub fn request_keyboard<Message: 'static>(&self, id: Id) -> Task<Message> {
        if let Some((_, Some(shell_info), _)) = self.0.iter().find(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell_info| shell_info.id) == Some(id)
                || shell_info.as_ref().map(|shell_info| shell_info.menu.id) == Some(id)
        }) {
            shell_info.menu.request_keyboard()
        } else {
            Task::none()
        }
    }

    pub fn release_keyboard<Message: 'static>(&self, id: Id) -> Task<Message> {
        if let Some((_, Some(shell_info), _)) = self.0.iter().find(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell_info| shell_info.id) == Some(id)
                || shell_info.as_ref().map(|shell_info| shell_info.menu.id) == Some(id)
        }) {
            shell_info.menu.release_keyboard()
        } else {
            Task::none()
        }
    }
}
