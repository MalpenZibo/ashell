use iced::{
    Anchor, InputRegionRect, KeyboardInteractivity, Layer, LayerShellSettings, OutputId, SurfaceId,
    Task, destroy_layer_surface, new_layer_surface, set_anchor, set_exclusive_zone,
    set_input_region, set_keyboard_interactivity, set_size,
};
use log::debug;

use crate::{
    HEIGHT,
    components::ButtonUIRef,
    components::menu::{Menu, MenuType, OpenMenu},
    config::{self, AppearanceStyle, Position},
};

#[derive(Debug, Clone)]
pub struct ShellInfo {
    pub id: SurfaceId,
    pub position: Position,
    pub layer: config::Layer,
    pub style: AppearanceStyle,
    pub menu: Menu,
    pub scale_factor: f64,
    /// Optional layer surface used to render toast notifications.
    pub toast_id: Option<SurfaceId>,
    /// Logical height of this output (for computing toast input regions).
    pub output_logical_height: Option<u32>,
}

impl ShellInfo {
    fn destroy_surfaces<Message: 'static>(self) -> Task<Message> {
        let mut tasks = vec![destroy_layer_surface(self.id)];
        if let Some(menu_id) = self.menu.surface_id() {
            tasks.push(destroy_layer_surface(menu_id));
        }
        if let Some(toast_id) = self.toast_id {
            tasks.push(destroy_layer_surface(toast_id));
        }
        Task::batch(tasks)
    }
}

#[derive(Debug, Clone)]
pub struct Outputs(Vec<(String, Option<ShellInfo>, Option<OutputId>)>);

pub enum HasOutput<'a> {
    Main,
    Menu(Option<&'a OpenMenu>),
    Toast,
}

impl Outputs {
    pub fn iter(&self) -> std::slice::Iter<'_, (String, Option<ShellInfo>, Option<OutputId>)> {
        self.0.iter()
    }

    pub fn new(
        style: AppearanceStyle,
        position: Position,
        layer: config::Layer,
        scale_factor: f64,
    ) -> Self {
        // Use the initial surface created by .layer_shell() in main.rs as a
        // fallback until real outputs are detected. Menu surfaces are created
        // on demand when a menu is opened.
        Self(vec![(
            "Fallback".to_string(),
            Some(ShellInfo {
                id: SurfaceId::MAIN,
                menu: Menu::new(),
                toast_id: None,
                position,
                layer,
                style,
                scale_factor,
                output_logical_height: None,
            }),
            None,
        )])
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
    ) -> (SurfaceId, Task<Message>) {
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

        (id, main_task)
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
                } else if info.menu.surface_id() == Some(id) {
                    Some(HasOutput::Menu(info.menu.open.as_ref()))
                } else if let Some(toast_id) = info.toast_id {
                    if toast_id == id {
                        Some(HasOutput::Toast)
                    } else {
                        None
                    }
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

            let (id, task) =
                Self::create_output_layers(style, Some(output_id), position, layer, scale_factor);

            let destroy_task = match self.0.iter().position(|(key, _, _)| key.as_str() == name) {
                Some(index) => {
                    let old_output = self.0.swap_remove(index);

                    match old_output.1 {
                        Some(shell_info) => shell_info.destroy_surfaces(),
                        _ => Task::none(),
                    }
                }
                _ => Task::none(),
            };

            self.0.push((
                name.to_owned(),
                Some(ShellInfo {
                    id,
                    menu: Menu::new(),
                    toast_id: None,
                    position,
                    layer,
                    style,
                    scale_factor,
                    output_logical_height: None,
                }),
                Some(output_id),
            ));

            // remove fallback layer surface
            let destroy_fallback_task =
                match self.0.iter().position(|(_, _, output)| output.is_none()) {
                    Some(index) => {
                        let old_output = self.0.swap_remove(index);

                        match old_output.1 {
                            Some(shell_info) => shell_info.destroy_surfaces(),
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
                    shell_info.destroy_surfaces()
                } else {
                    Task::none()
                };

                self.0.push((name, None, output_id));

                if self.0.iter().any(|(_, shell_info, _)| shell_info.is_some()) {
                    Task::batch(vec![destroy_task])
                } else {
                    debug!("No outputs left, creating a fallback layer surface");

                    let (id, task) =
                        Self::create_output_layers(style, None, position, layer, scale_factor);

                    self.0.push((
                        "Fallback".to_string(),
                        Some(ShellInfo {
                            id,
                            menu: Menu::new(),
                            toast_id: None,
                            position,
                            layer,
                            style,
                            scale_factor,
                            output_logical_height: None,
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
                let old = shell_info.clone();
                let destroy_task = old.destroy_surfaces();

                let (id, create_task) =
                    Self::create_output_layers(style, *output_id, position, layer, scale_factor);

                shell_info.id = id;
                shell_info.menu = Menu::new();
                shell_info.toast_id = None;
                shell_info.style = style;
                shell_info.scale_factor = scale_factor;

                tasks.push(Task::batch(vec![destroy_task, create_task]));
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

    fn find_by_surface_id(
        &self,
        id: SurfaceId,
    ) -> Option<&(String, Option<ShellInfo>, Option<OutputId>)> {
        self.0.iter().find(|(_, shell_info, _)| {
            shell_info
                .as_ref()
                .is_some_and(|si| si.id == id || si.menu.surface_id() == Some(id))
        })
    }

    fn find_by_surface_id_mut(
        &mut self,
        id: SurfaceId,
    ) -> Option<&mut (String, Option<ShellInfo>, Option<OutputId>)> {
        self.0.iter_mut().find(|(_, shell_info, _)| {
            shell_info
                .as_ref()
                .is_some_and(|si| si.id == id || si.menu.surface_id() == Some(id))
        })
    }

    pub fn menu_is_open(&self) -> bool {
        self.0
            .iter()
            .any(|(_, shell_info, _)| shell_info.as_ref().is_some_and(|si| si.menu.is_open()))
    }

    pub fn toggle_menu<Message: 'static>(
        &mut self,
        id: SurfaceId,
        menu_type: MenuType,
        button_ui_ref: ButtonUIRef,
        request_keyboard: bool,
    ) -> Task<Message> {
        let task = match self.find_by_surface_id_mut(id) {
            Some((_, Some(shell_info), output_id)) => {
                let output_id = *output_id;
                let toggle_task =
                    shell_info
                        .menu
                        .toggle(menu_type, button_ui_ref, request_keyboard, output_id);
                let mut tasks = self
                    .0
                    .iter_mut()
                    .filter_map(|(_, shell_info, _)| {
                        if let Some(shell_info) = shell_info {
                            if shell_info.id != id && shell_info.menu.surface_id() != Some(id) {
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

    /// Disable keyboard interactivity on all outputs if no menus remain open.
    fn maybe_release_all_keyboards<Message: 'static>(
        &self,
        task: Task<Message>,
        esc_button_enabled: bool,
    ) -> Task<Message> {
        if esc_button_enabled && !self.menu_is_open() {
            let keyboard_tasks = self
                .0
                .iter()
                .filter_map(|(_, shell_info, _)| {
                    shell_info
                        .as_ref()
                        .map(|si| set_keyboard_interactivity(si.id, KeyboardInteractivity::None))
                })
                .collect::<Vec<_>>();
            Task::batch(vec![task, Task::batch(keyboard_tasks)])
        } else {
            task
        }
    }

    pub fn close_menu<Message: 'static>(
        &mut self,
        id: SurfaceId,
        menu_type: Option<MenuType>,
        esc_button_enabled: bool,
    ) -> Task<Message> {
        let task = match self.find_by_surface_id_mut(id) {
            Some((_, Some(shell_info), _)) => match menu_type {
                Some(mt) => shell_info.menu.close_if(mt),
                None => shell_info.menu.close(),
            },
            _ => Task::none(),
        };

        self.maybe_release_all_keyboards(task, esc_button_enabled)
    }

    pub fn close_all_menu_if<Message: 'static>(
        &mut self,
        menu_type: MenuType,
        esc_button_enabled: bool,
    ) -> Task<Message> {
        let task = Task::batch(
            self.0
                .iter_mut()
                .filter_map(|(_, shell_info, _)| {
                    shell_info
                        .as_mut()
                        .map(|si| si.menu.close_if(menu_type.clone()))
                })
                .collect::<Vec<_>>(),
        );

        self.maybe_release_all_keyboards(task, esc_button_enabled)
    }

    pub fn close_all_menus<Message: 'static>(&mut self, esc_button_enabled: bool) -> Task<Message> {
        let task = Task::batch(
            self.0
                .iter_mut()
                .filter_map(|(_, shell_info, _)| {
                    shell_info
                        .as_mut()
                        .filter(|si| si.menu.is_open())
                        .map(|si| si.menu.close())
                })
                .collect::<Vec<_>>(),
        );

        self.maybe_release_all_keyboards(task, esc_button_enabled)
    }

    pub fn request_keyboard<Message: 'static>(&self, id: SurfaceId) -> Task<Message> {
        match self.find_by_surface_id(id) {
            Some((_, Some(shell_info), _)) => shell_info.menu.request_keyboard(),
            _ => Task::none(),
        }
    }

    pub fn release_keyboard<Message: 'static>(&self, id: SurfaceId) -> Task<Message> {
        match self.find_by_surface_id(id) {
            Some((_, Some(shell_info), _)) => shell_info.menu.release_keyboard(),
            _ => Task::none(),
        }
    }

    /// Show the toast layer(s) for every output.
    ///
    /// Creates a full-height surface anchored at `position`. Height is 0 so
    /// the compositor fills the output height. The surface starts with an
    /// empty input region (fully click-through); `update_toast_input_region`
    /// restricts input to just the rendered toast area after layout.
    pub fn show_toast_layer<Message: 'static>(
        &mut self,
        width: u32,
        position: config::ToastPosition,
    ) -> Task<Message> {
        let mut tasks = vec![];

        for (_, shell_info, _) in &mut self.0 {
            if let Some(shell_info) = shell_info
                && shell_info.toast_id.is_none()
            {
                // Anchor both vertical edges so height 0 → full output height.
                let anchor = match position {
                    config::ToastPosition::TopLeft => Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT,
                    config::ToastPosition::TopRight => Anchor::TOP | Anchor::BOTTOM | Anchor::RIGHT,
                    config::ToastPosition::BottomLeft => {
                        Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT
                    }
                    config::ToastPosition::BottomRight => {
                        Anchor::TOP | Anchor::BOTTOM | Anchor::RIGHT
                    }
                };

                let (toast_id, toast_task) = new_layer_surface(LayerShellSettings {
                    namespace: "ashell-toast-layer".to_string(),
                    size: Some((width, 0)),
                    layer: Layer::Overlay,
                    keyboard_interactivity: KeyboardInteractivity::None,
                    exclusive_zone: 0,
                    anchor,
                    ..Default::default()
                });

                shell_info.toast_id = Some(toast_id);
                tasks.push(toast_task);
                // Start fully click-through until sensor reports content size.
                tasks.push(set_input_region(toast_id, Some(vec![])));
            }
        }

        Task::batch(tasks)
    }

    /// Update the input region of the toast surface(s) so only the rendered
    /// toast content accepts pointer input. Everything else is click-through.
    pub fn update_toast_input_region<Message: 'static>(
        &self,
        content_size: iced::Size,
        position: config::ToastPosition,
    ) -> Task<Message> {
        let content_w = content_size.width.ceil() as i32;
        let content_h = content_size.height.ceil() as i32;
        let mut tasks = vec![];
        for (_, shell_info, _) in &self.0 {
            if let Some(shell_info) = shell_info
                && let Some(toast_id) = shell_info.toast_id
            {
                let y = match position {
                    config::ToastPosition::TopLeft | config::ToastPosition::TopRight => 0,
                    config::ToastPosition::BottomLeft | config::ToastPosition::BottomRight => {
                        shell_info
                            .output_logical_height
                            .map_or(0, |h| (h as i32) - content_h)
                    }
                };
                tasks.push(set_input_region(
                    toast_id,
                    Some(vec![InputRegionRect {
                        x: 0,
                        y,
                        width: content_w,
                        height: content_h,
                    }]),
                ));
            }
        }
        Task::batch(tasks)
    }

    /// Store the logical height for an output (used for bottom-aligned toast input regions).
    pub fn set_output_logical_height(&mut self, output_id: OutputId, height: u32) {
        for (_, shell_info, oid) in &mut self.0 {
            if *oid == Some(output_id)
                && let Some(info) = shell_info
            {
                info.output_logical_height = Some(height);
            }
        }
    }

    pub fn hide_toast_layer<Message: 'static>(&mut self) -> Task<Message> {
        let mut tasks = vec![];
        for (_, shell_info, _) in &mut self.0 {
            if let Some(shell_info) = shell_info
                && let Some(toast_id) = shell_info.toast_id.take()
            {
                tasks.push(destroy_layer_surface(toast_id));
            }
        }
        Task::batch(tasks)
    }
}
