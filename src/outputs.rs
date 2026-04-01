use iced_layershell::{
    Anchor, KeyboardInteractivity, Layer, LayerShellSettings, OutputId, SurfaceId, Task,
    destroy_layer_surface, new_layer_surface, set_anchor, set_exclusive_zone,
    set_keyboard_interactivity, set_size,
};
use log::debug;

use crate::{
    HEIGHT,
    config::{self, AppearanceStyle, Position},
    menu::{Menu, MenuType},
    widgets::ButtonUIRef,
};

#[derive(Debug, Clone)]
pub struct ShellInfo {
    pub id: SurfaceId,
    pub position: Position,
    pub layer: config::Layer,
    pub style: AppearanceStyle,
    pub menu: Menu,
    pub scale_factor: f64,
}

#[derive(Debug, Clone)]
pub struct Outputs(Vec<(String, Option<ShellInfo>, Option<OutputId>)>);

pub enum HasOutput<'a> {
    Main,
    Menu(Option<&'a (MenuType, ButtonUIRef)>),
}

impl Outputs {
    pub fn iter(&self) -> std::slice::Iter<'_, (String, Option<ShellInfo>, Option<OutputId>)> {
        self.0.iter()
    }

    pub fn new<Message: 'static>(
        style: AppearanceStyle,
        position: Position,
        layer: config::Layer,
        scale_factor: f64,
    ) -> (Self, Task<Message>) {
        // The MAIN surface is pre-created by the framework via .layer_shell() in main.rs
        // We only need to create the menu overlay surface
        let (menu_id, menu_task) = new_layer_surface(LayerShellSettings {
            anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
            layer: Layer::Background,
            keyboard_interactivity: KeyboardInteractivity::None,
            namespace: "ashell-menu-layer".into(),
            ..Default::default()
        });

        (
            Self(vec![(
                "Fallback".to_string(),
                Some(ShellInfo {
                    id: SurfaceId::MAIN,
                    menu: Menu::new(menu_id),
                    position,
                    layer,
                    style,
                    scale_factor,
                }),
                None,
            )]),
            menu_task,
        )
    }

    pub fn get_height(style: AppearanceStyle, scale_factor: f64) -> f64 {
        (HEIGHT
            - match style {
                AppearanceStyle::Solid | AppearanceStyle::Gradient => 8.,
                AppearanceStyle::Islands => 0.,
            })
            * scale_factor
    }

    pub fn create_output_layers<Message: 'static>(
        style: AppearanceStyle,
        output_id: Option<OutputId>,
        position: Position,
        layer: config::Layer,
        scale_factor: f64,
    ) -> (SurfaceId, SurfaceId, Task<Message>) {
        let height = Self::get_height(style, scale_factor);

        let iced_layer = match layer {
            config::Layer::Top => Layer::Top,
            config::Layer::Bottom => Layer::Bottom,
            config::Layer::Overlay => Layer::Overlay,
        };

        let (id, main_task) = new_layer_surface(LayerShellSettings {
            namespace: "ashell-main-layer".to_string(),
            size: Some((0, height as u32)),
            layer: iced_layer,
            keyboard_interactivity: KeyboardInteractivity::None,
            exclusive_zone: height as i32,
            output: output_id,
            anchor: match position {
                Position::Top => Anchor::TOP,
                Position::Bottom => Anchor::BOTTOM,
            } | Anchor::LEFT
                | Anchor::RIGHT,
            ..Default::default()
        });

        let (menu_id, menu_task) = new_layer_surface(LayerShellSettings {
            namespace: "ashell-menu-layer".to_string(),
            size: None,
            layer: Layer::Background,
            keyboard_interactivity: KeyboardInteractivity::None,
            output: output_id,
            anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
            ..Default::default()
        });

        (id, menu_id, Task::batch(vec![main_task, menu_task]))
    }

    fn name_in_config(name: &str, outputs: &config::Outputs) -> bool {
        match outputs {
            config::Outputs::All => true,
            config::Outputs::Active => false,
            config::Outputs::Targets(request_outputs) => {
                request_outputs.iter().any(|output| name.contains(output))
            }
        }
    }

    pub fn has(&'_ self, id: SurfaceId) -> Option<HasOutput<'_>> {
        self.0.iter().find_map(|(_, info, _)| {
            info.as_ref().and_then(|info| {
                if info.id == id {
                    Some(HasOutput::Main)
                } else if info.menu.id == id {
                    Some(HasOutput::Menu(info.menu.menu_info.as_ref()))
                } else {
                    None
                }
            })
        })
    }

    pub fn get_monitor_name(&self, id: SurfaceId) -> Option<&str> {
        self.0.iter().find_map(|(name, info, _)| {
            info.as_ref().and_then(|info| {
                if info.id == id {
                    Some(name.as_str())
                } else {
                    None
                }
            })
        })
    }

    pub fn has_name(&self, name: &str) -> bool {
        self.0
            .iter()
            .any(|(n, info, _)| info.is_some() && n.as_str().contains(name))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add<Message: 'static>(
        &mut self,
        style: AppearanceStyle,
        request_outputs: &config::Outputs,
        position: Position,
        layer: config::Layer,
        name: &str,
        output_id: OutputId,
        scale_factor: f64,
    ) -> Task<Message> {
        let target = Self::name_in_config(name, request_outputs);

        if target {
            debug!("Found target output, creating a new layer surface");

            let (id, menu_id, task) =
                Self::create_output_layers(style, Some(output_id), position, layer, scale_factor);

            let destroy_task = match self.0.iter().position(|(key, _, _)| key.as_str() == name) {
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
                name.to_owned(),
                Some(ShellInfo {
                    id,
                    menu: Menu::new(menu_id),
                    position,
                    layer,
                    style,
                    scale_factor,
                }),
                Some(output_id),
            ));

            // remove fallback layer surface
            let destroy_fallback_task =
                match self.0.iter().position(|(_, _, output)| output.is_none()) {
                    Some(index) => {
                        let old_output = self.0.swap_remove(index);

                        match old_output.1 {
                            Some(shell_info) => {
                                let destroy_fallback_main_task =
                                    destroy_layer_surface(shell_info.id);
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
            debug!(
                "Output {:?} does not match configured output target {:?}",
                name, request_outputs
            );

            self.0.push((name.to_owned(), None, Some(output_id)));

            Task::none()
        }
    }

    pub fn remove<Message: 'static>(
        &mut self,
        style: AppearanceStyle,
        position: Position,
        layer: config::Layer,
        output_id: OutputId,
        scale_factor: f64,
    ) -> Task<Message> {
        match self.0.iter().position(|(_, _, assigned_output_id)| {
            assigned_output_id
                .as_ref()
                .is_some_and(|assigned| *assigned == output_id)
        }) {
            Some(index_to_remove) => {
                debug!("Removing layer surface for output");

                let (name, shell_info, output_id) = self.0.swap_remove(index_to_remove);

                let destroy_task = if let Some(shell_info) = shell_info {
                    let destroy_main_task = destroy_layer_surface(shell_info.id);
                    let destroy_menu_task = destroy_layer_surface(shell_info.menu.id);

                    Task::batch(vec![destroy_main_task, destroy_menu_task])
                } else {
                    Task::none()
                };

                self.0.push((name, None, output_id));

                if self.0.iter().any(|(_, shell_info, _)| shell_info.is_some()) {
                    Task::batch(vec![destroy_task])
                } else {
                    debug!("No outputs left, creating a fallback layer surface");

                    let (id, menu_id, task) =
                        Self::create_output_layers(style, None, position, layer, scale_factor);

                    self.0.push((
                        "Fallback".to_string(),
                        Some(ShellInfo {
                            id,
                            menu: Menu::new(menu_id),
                            position,
                            layer,
                            style,
                            scale_factor,
                        }),
                        None,
                    ));

                    Task::batch(vec![destroy_task, task])
                }
            }
            _ => Task::none(),
        }
    }

    pub fn sync<Message: 'static>(
        &mut self,
        style: AppearanceStyle,
        request_outputs: &config::Outputs,
        position: Position,
        layer: config::Layer,
        scale_factor: f64,
    ) -> Task<Message> {
        debug!("Syncing outputs: {self:?}, request_outputs: {request_outputs:?}");

        let to_remove = self
            .0
            .iter()
            .filter_map(|(name, shell_info, output_id)| {
                if !Self::name_in_config(name, request_outputs) && shell_info.is_some() {
                    *output_id
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        debug!("Removing outputs: {to_remove:?}");

        let to_add = self
            .0
            .iter()
            .filter_map(|(name, shell_info, output_id)| {
                if Self::name_in_config(name, request_outputs) && shell_info.is_none() {
                    Some((name.clone(), *output_id))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        debug!("Adding outputs: {to_add:?}");

        let mut tasks = Vec::new();

        for (name, output_id) in to_add {
            if let Some(output_id) = output_id {
                tasks.push(self.add(
                    style,
                    request_outputs,
                    position,
                    layer,
                    name.as_str(),
                    output_id,
                    scale_factor,
                ));
            }
        }

        for output_id in to_remove {
            tasks.push(self.remove(style, position, layer, output_id, scale_factor));
        }

        for shell_info in self.0.iter_mut().filter_map(|(_, shell_info, _)| {
            if let Some(shell_info) = shell_info
                && shell_info.position != position
            {
                Some(shell_info)
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

        // Handle layer changes - only recreate surfaces when layer actually changes
        for (_name, shell_info, output_id) in &mut self.0 {
            if let Some(shell_info) = shell_info
                && shell_info.layer != layer
            {
                let destroy_main_task = destroy_layer_surface(shell_info.id);
                let destroy_menu_task = destroy_layer_surface(shell_info.menu.id);

                let (id, menu_id, task) =
                    Self::create_output_layers(style, *output_id, position, layer, scale_factor);

                shell_info.id = id;
                shell_info.menu = Menu::new(menu_id);
                shell_info.position = position;
                shell_info.layer = layer;
                shell_info.style = style;
                shell_info.scale_factor = scale_factor;

                tasks.push(Task::batch(vec![
                    destroy_main_task,
                    destroy_menu_task,
                    task,
                ]));
            }
        }

        for shell_info in self.0.iter_mut().filter_map(|(_, shell_info, _)| {
            if let Some(shell_info) = shell_info
                && (shell_info.style != style || shell_info.scale_factor != scale_factor)
            {
                Some(shell_info)
            } else {
                None
            }
        }) {
            debug!(
                "Change style or scale_factor for output: {:?}, new style {:?}, new scale_factor {:?}",
                shell_info.id, style, scale_factor
            );
            shell_info.style = style;
            shell_info.scale_factor = scale_factor;
            let height = Self::get_height(style, scale_factor);
            tasks.push(Task::batch(vec![
                set_size(shell_info.id, (0, height as u32)),
                set_exclusive_zone(shell_info.id, height as i32),
            ]));
        }

        Task::batch(tasks)
    }

    pub fn menu_is_open(&self) -> bool {
        self.0.iter().any(|(_, shell_info, _)| {
            shell_info
                .as_ref()
                .map(|shell_info| shell_info.menu.menu_info.is_some())
                .unwrap_or_default()
        })
    }

    pub fn toggle_menu<Message: 'static>(
        &mut self,
        id: SurfaceId,
        menu_type: MenuType,
        button_ui_ref: ButtonUIRef,
        request_keyboard: bool,
    ) -> Task<Message> {
        let task = match self.0.iter_mut().find(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell_info| shell_info.id) == Some(id)
                || shell_info.as_ref().map(|shell_info| shell_info.menu.id) == Some(id)
        }) {
            Some((_, Some(shell_info), _)) => {
                let toggle_task =
                    shell_info
                        .menu
                        .toggle(menu_type, button_ui_ref, request_keyboard);
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
        };

        if request_keyboard {
            if self.menu_is_open() {
                Task::batch(vec![
                    task,
                    set_keyboard_interactivity(id, KeyboardInteractivity::OnDemand),
                ])
            } else {
                Task::batch(vec![
                    task,
                    set_keyboard_interactivity(id, KeyboardInteractivity::None),
                ])
            }
        } else {
            task
        }
    }

    pub fn close_menu<Message: 'static>(
        &mut self,
        id: SurfaceId,
        esc_button_enabled: bool,
    ) -> Task<Message> {
        let task = match self.0.iter_mut().find(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell_info| shell_info.id) == Some(id)
                || shell_info.as_ref().map(|shell_info| shell_info.menu.id) == Some(id)
        }) {
            Some((_, Some(shell_info), _)) => shell_info.menu.close(),
            _ => Task::none(),
        };

        if esc_button_enabled && !self.menu_is_open() {
            Task::batch(vec![
                task,
                set_keyboard_interactivity(id, KeyboardInteractivity::None),
            ])
        } else {
            task
        }
    }

    pub fn close_menu_if<Message: 'static>(
        &mut self,
        id: SurfaceId,
        menu_type: MenuType,
        esc_button_enabled: bool,
    ) -> Task<Message> {
        let task = match self.0.iter_mut().find(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell_info| shell_info.id) == Some(id)
                || shell_info.as_ref().map(|shell_info| shell_info.menu.id) == Some(id)
        }) {
            Some((_, Some(shell_info), _)) => shell_info.menu.close_if(menu_type),
            _ => Task::none(),
        };

        if esc_button_enabled && !self.menu_is_open() {
            Task::batch(vec![
                task,
                set_keyboard_interactivity(id, KeyboardInteractivity::None),
            ])
        } else {
            task
        }
    }

    pub fn close_all_menu_if<Message: 'static>(
        &mut self,
        menu_type: MenuType,
        esc_button_enabled: bool,
    ) -> Task<Message> {
        let task = Task::batch(
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
        );

        if esc_button_enabled && !self.menu_is_open() {
            let keyboard_tasks = self
                .0
                .iter()
                .map(|(_, shell_info, _)| {
                    shell_info.as_ref().map_or_else(Task::none, |shell_info| {
                        set_keyboard_interactivity(shell_info.id, KeyboardInteractivity::None)
                    })
                })
                .collect::<Vec<_>>();
            Task::batch(vec![task, Task::batch(keyboard_tasks)])
        } else {
            task
        }
    }

    pub fn close_all_menus<Message: 'static>(&mut self, esc_button_enabled: bool) -> Task<Message> {
        let task = Task::batch(
            self.0
                .iter_mut()
                .map(|(_, shell_info, _)| {
                    if let Some(shell_info) = shell_info {
                        if shell_info.menu.menu_info.is_some() {
                            shell_info.menu.close()
                        } else {
                            Task::none()
                        }
                    } else {
                        Task::none()
                    }
                })
                .collect::<Vec<_>>(),
        );

        if esc_button_enabled && !self.menu_is_open() {
            let keyboard_tasks = self
                .0
                .iter()
                .map(|(_, shell_info, _)| {
                    shell_info.as_ref().map_or_else(Task::none, |shell_info| {
                        set_keyboard_interactivity(shell_info.id, KeyboardInteractivity::None)
                    })
                })
                .collect::<Vec<_>>();
            Task::batch(vec![task, Task::batch(keyboard_tasks)])
        } else {
            task
        }
    }

    pub fn request_keyboard<Message: 'static>(&self, id: SurfaceId) -> Task<Message> {
        match self.0.iter().find(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell_info| shell_info.id) == Some(id)
                || shell_info.as_ref().map(|shell_info| shell_info.menu.id) == Some(id)
        }) {
            Some((_, Some(shell_info), _)) => shell_info.menu.request_keyboard(),
            _ => Task::none(),
        }
    }

    pub fn release_keyboard<Message: 'static>(&self, id: SurfaceId) -> Task<Message> {
        match self.0.iter().find(|(_, shell_info, _)| {
            shell_info.as_ref().map(|shell_info| shell_info.id) == Some(id)
                || shell_info.as_ref().map(|shell_info| shell_info.menu.id) == Some(id)
        }) {
            Some((_, Some(shell_info), _)) => shell_info.menu.release_keyboard(),
            _ => Task::none(),
        }
    }
}
