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
    /// Logical height of this output (for computing toast input regions).
    pub output_logical_height: Option<u32>,
}

impl ShellInfo {
    fn destroy_surfaces<Message: 'static>(self) -> Task<Message> {
        let mut tasks = vec![destroy_layer_surface(self.id)];
        if let Some(menu_id) = self.menu.surface_id() {
            tasks.push(destroy_layer_surface(menu_id));
        }
        Task::batch(tasks)
    }
}

/// A floating overlay surface (toast or OSD) with `output: None`. `output` is
/// filled in by `OutputEvent::SurfaceEnteredOutput` and used to look up the
/// correct logical height for input-region positioning.
#[derive(Debug, Clone, Copy)]
struct OverlaySurface {
    id: SurfaceId,
    output: Option<OutputId>,
}

impl OverlaySurface {
    fn show<Message: 'static>(
        slot: &mut Option<Self>,
        settings: LayerShellSettings,
    ) -> Option<(SurfaceId, Task<Message>)> {
        if slot.is_some() {
            return None;
        }
        let (id, task) = new_layer_surface(settings);
        *slot = Some(Self { id, output: None });
        Some((id, task))
    }

    fn hide<Message: 'static>(slot: &mut Option<Self>) -> Task<Message> {
        match slot.take() {
            Some(s) => destroy_layer_surface(s.id),
            None => Task::none(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Outputs {
    entries: Vec<(String, Option<ShellInfo>, Option<OutputId>)>,
    toast: Option<OverlaySurface>,
    osd: Option<OverlaySurface>,
    /// Last toast input-region request, replayed once the toast's output is
    /// known (the enter event may arrive after the first layout).
    toast_region: Option<(iced::Size, config::ToastPosition)>,
}

pub enum HasOutput<'a> {
    Main,
    Menu(Option<&'a OpenMenu>),
    Toast,
    Osd,
}

impl Outputs {
    pub fn iter(&self) -> std::slice::Iter<'_, (String, Option<ShellInfo>, Option<OutputId>)> {
        self.entries.iter()
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
        Self {
            entries: vec![(
                "Fallback".to_string(),
                Some(ShellInfo {
                    id: SurfaceId::MAIN,
                    menu: Menu::new(),
                    position,
                    layer,
                    style,
                    scale_factor,
                    output_logical_height: None,
                }),
                None,
            )],
            toast: None,
            osd: None,
            toast_region: None,
        }
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
        if self.toast.is_some_and(|t| t.id == id) {
            return Some(HasOutput::Toast);
        }
        if self.osd.is_some_and(|o| o.id == id) {
            return Some(HasOutput::Osd);
        }
        self.entries.iter().find_map(|(_, info, _)| {
            info.as_ref().and_then(|info| {
                if info.id == id {
                    Some(HasOutput::Main)
                } else if info.menu.surface_id() == Some(id) {
                    Some(HasOutput::Menu(info.menu.open.as_ref()))
                } else {
                    None
                }
            })
        })
    }

    pub fn get_monitor_name(&self, id: SurfaceId) -> Option<&str> {
        self.entries.iter().find_map(|(name, info, _)| {
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
        self.entries
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

            let destroy_task = match self
                .entries
                .iter()
                .position(|(key, _, _)| key.as_str() == name)
            {
                Some(index) => {
                    let old_output = self.entries.swap_remove(index);

                    match old_output.1 {
                        Some(shell_info) => shell_info.destroy_surfaces(),
                        _ => Task::none(),
                    }
                }
                _ => Task::none(),
            };

            self.entries.push((
                name.to_owned(),
                Some(ShellInfo {
                    id,
                    menu: Menu::new(),
                    position,
                    layer,
                    style,
                    scale_factor,
                    output_logical_height: None,
                }),
                Some(output_id),
            ));

            // remove fallback layer surface
            let destroy_fallback_task = match self
                .entries
                .iter()
                .position(|(_, _, output)| output.is_none())
            {
                Some(index) => {
                    let old_output = self.entries.swap_remove(index);

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

            self.entries.push((name.to_owned(), None, Some(output_id)));

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
        match self.entries.iter().position(|(_, _, assigned_output_id)| {
            assigned_output_id
                .as_ref()
                .is_some_and(|assigned| *assigned == output_id)
        }) {
            Some(index_to_remove) => {
                debug!("Removing layer surface for output");

                let (_name, shell_info, _output_id) = self.entries.swap_remove(index_to_remove);

                let destroy_task = if let Some(shell_info) = shell_info {
                    shell_info.destroy_surfaces()
                } else {
                    Task::none()
                };

                // Drop the entry entirely instead of keeping a (name, None, stale_id) marker:
                // on resume, sync() would otherwise treat it as "needs re-add" and call add()
                // with the stale OutputId. iced_layershell can't resolve the id, silently
                // falls back to output=None, and the compositor binds the phantom surface
                // to any available monitor — producing a duplicate bar.

                if self
                    .entries
                    .iter()
                    .any(|(_, shell_info, _)| shell_info.is_some())
                {
                    Task::batch(vec![destroy_task])
                } else {
                    debug!("No outputs left, creating a fallback layer surface");

                    let (id, task) =
                        Self::create_output_layers(style, None, position, layer, scale_factor);

                    self.entries.push((
                        "Fallback".to_string(),
                        Some(ShellInfo {
                            id,
                            menu: Menu::new(),
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
            .entries
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
            .entries
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

        for shell_info in self.entries.iter_mut().filter_map(|(_, shell_info, _)| {
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
        for (_name, shell_info, output_id) in &mut self.entries {
            if let Some(shell_info) = shell_info
                && shell_info.layer != layer
            {
                let old = shell_info.clone();
                let destroy_task = old.destroy_surfaces();

                let (id, create_task) =
                    Self::create_output_layers(style, *output_id, position, layer, scale_factor);

                shell_info.id = id;
                shell_info.menu = Menu::new();
                shell_info.style = style;
                shell_info.scale_factor = scale_factor;

                tasks.push(Task::batch(vec![destroy_task, create_task]));
            }
        }

        for shell_info in self.entries.iter_mut().filter_map(|(_, shell_info, _)| {
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
        self.entries.iter().find(|(_, shell_info, _)| {
            shell_info
                .as_ref()
                .is_some_and(|si| si.id == id || si.menu.surface_id() == Some(id))
        })
    }

    fn find_by_surface_id_mut(
        &mut self,
        id: SurfaceId,
    ) -> Option<&mut (String, Option<ShellInfo>, Option<OutputId>)> {
        self.entries.iter_mut().find(|(_, shell_info, _)| {
            shell_info
                .as_ref()
                .is_some_and(|si| si.id == id || si.menu.surface_id() == Some(id))
        })
    }

    pub fn menu_is_open(&self) -> bool {
        self.entries
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
                    .entries
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
                .entries
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
            self.entries
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
            self.entries
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

    /// Create a single full-height overlay surface for toast notifications.
    /// Starts with an empty input region (fully click-through);
    /// `update_toast_input_region` restricts input to the rendered toast area.
    pub fn show_toast_layer<Message: 'static>(
        &mut self,
        width: u32,
        position: config::ToastPosition,
    ) -> Task<Message> {
        // Anchor both vertical edges so height 0 → full output height.
        let anchor = match position {
            config::ToastPosition::TopLeft | config::ToastPosition::BottomLeft => {
                Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT
            }
            config::ToastPosition::TopRight | config::ToastPosition::BottomRight => {
                Anchor::TOP | Anchor::BOTTOM | Anchor::RIGHT
            }
        };

        let Some((toast_id, toast_task)) = OverlaySurface::show(
            &mut self.toast,
            LayerShellSettings {
                namespace: "ashell-toast-layer".to_string(),
                size: Some((width, 0)),
                layer: Layer::Overlay,
                keyboard_interactivity: KeyboardInteractivity::None,
                exclusive_zone: 0,
                anchor,
                output: None,
                ..Default::default()
            },
        ) else {
            return Task::none();
        };

        Task::batch(vec![toast_task, set_input_region(toast_id, Some(vec![]))])
    }

    /// Update the input region of the toast surface so only the rendered
    /// toast content accepts pointer input. Everything else is click-through.
    pub fn update_toast_input_region<Message: 'static>(
        &mut self,
        content_size: iced::Size,
        position: config::ToastPosition,
    ) -> Task<Message> {
        let Some(toast) = self.toast else {
            return Task::none();
        };
        self.toast_region = Some((content_size, position));
        let content_w = content_size.width.ceil() as i32;
        let content_h = content_size.height.ceil() as i32;
        let y = match position {
            config::ToastPosition::TopLeft | config::ToastPosition::TopRight => 0,
            config::ToastPosition::BottomLeft | config::ToastPosition::BottomRight => self
                .logical_height_for_output(toast.output)
                .map_or(0, |h| (h as i32) - content_h),
        };
        set_input_region(
            toast.id,
            Some(vec![InputRegionRect {
                x: 0,
                y,
                width: content_w,
                height: content_h,
            }]),
        )
    }

    fn logical_height_for_output(&self, output_id: Option<OutputId>) -> Option<u32> {
        let target = output_id?;
        self.entries.iter().find_map(|(_, info, oid)| {
            if *oid == Some(target) {
                info.as_ref().and_then(|i| i.output_logical_height)
            } else {
                None
            }
        })
    }

    /// Store the logical height for an output (used for bottom-aligned toast input regions).
    pub fn set_output_logical_height(&mut self, output_id: OutputId, height: u32) {
        for (_, shell_info, oid) in &mut self.entries {
            if *oid == Some(output_id)
                && let Some(info) = shell_info
            {
                info.output_logical_height = Some(height);
            }
        }
    }

    /// Track which output the toast/OSD overlay is mapped on, populated from
    /// `OutputEvent::SurfaceEnteredOutput` after the compositor maps the
    /// surface.
    pub fn surface_entered_output<Message: 'static>(
        &mut self,
        surface_id: SurfaceId,
        output_id: OutputId,
    ) -> Task<Message> {
        let mut toast_output_changed = false;
        if let Some(toast) = self.toast.as_mut()
            && toast.id == surface_id
            && toast.output != Some(output_id)
        {
            toast.output = Some(output_id);
            toast_output_changed = true;
        }
        if let Some(osd) = self.osd.as_mut()
            && osd.id == surface_id
        {
            osd.output = Some(output_id);
        }

        // Replay the input region now the output's logical height is known.
        if toast_output_changed && let Some((content_size, position)) = self.toast_region {
            self.update_toast_input_region(content_size, position)
        } else {
            Task::none()
        }
    }

    pub fn surface_left_output(&mut self, surface_id: SurfaceId, output_id: OutputId) {
        let clear = |slot: &mut Option<OverlaySurface>| {
            if let Some(s) = slot
                && s.id == surface_id
                && s.output == Some(output_id)
            {
                s.output = None;
            }
        };
        clear(&mut self.toast);
        clear(&mut self.osd);
    }

    pub fn hide_toast_layer<Message: 'static>(&mut self) -> Task<Message> {
        self.toast_region = None;
        OverlaySurface::hide(&mut self.toast)
    }

    /// Create a centered bottom-anchored overlay surface for the OSD.
    pub fn show_osd_layer<Message: 'static>(&mut self, width: u32, height: u32) -> Task<Message> {
        OverlaySurface::show(
            &mut self.osd,
            LayerShellSettings {
                namespace: "ashell-osd-layer".to_string(),
                size: Some((width, height)),
                layer: Layer::Overlay,
                keyboard_interactivity: KeyboardInteractivity::None,
                exclusive_zone: 0,
                anchor: Anchor::BOTTOM,
                margin: (0, 0, 48, 0),
                output: None,
            },
        )
        .map_or(Task::none(), |(_, task)| task)
    }

    pub fn hide_osd_layer<Message: 'static>(&mut self) -> Task<Message> {
        OverlaySurface::hide(&mut self.osd)
    }
}
