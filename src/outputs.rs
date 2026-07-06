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

/// Pair of strings identifying an output. `name` is the canonical
/// short name reported by the compositor (e.g. `eDP-1`) — used for
/// strict equality checks against workspace events (`has_name`) and
/// as the layer-shell surface key. `description` includes the EDID
/// (e.g. `eDP-1 Make Model Serial`) — used only for the fuzzy
/// substring matching in `name_in_config`, so users can alias
/// outputs by any string that appears in the EDID (the behaviour
/// added by #312).
#[derive(Debug, Clone)]
pub struct OutputKey {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct Outputs {
    entries: Vec<(OutputKey, Option<ShellInfo>, Option<OutputId>)>,
    toast: Option<OverlaySurface>,
    osd: Option<OverlaySurface>,
    /// Last toast input-region request, replayed once the toast's output is
    /// known (the enter event may arrive after the first layout).
    toast_region: Option<(iced::Size, config::ToastPosition)>,
    toast_width: u32,
    animations_enabled: bool,
}

pub enum HasOutput<'a> {
    Main,
    Menu(Option<&'a OpenMenu>),
    Toast,
    Osd,
}

impl Outputs {
    pub fn iter(&self) -> std::slice::Iter<'_, (OutputKey, Option<ShellInfo>, Option<OutputId>)> {
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
                OutputKey {
                    name: "Fallback".to_string(),
                    description: String::new(),
                },
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
            toast_width: 0,
            animations_enabled: false,
        }
    }

    fn make_menu(&self) -> Menu {
        Menu::with_animations(self.animations_enabled)
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

    /// Match a user-supplied output spec against an output's name +
    /// description pair. Returns true when:
    ///   - the spec equals the canonical name (`info.name`), OR
    ///   - the spec appears anywhere in the description (`info.name +
    ///     info.make + info.model`), preserving #312's fuzzy alias
    ///     matching by EDID substring.
    fn matches_spec(name: &str, description: &str, spec: &str) -> bool {
        name == spec || description.contains(spec)
    }

    fn name_in_config(name: &str, description: &str, outputs: &config::Outputs) -> bool {
        match outputs {
            config::Outputs::All => true,
            config::Outputs::Active => false,
            config::Outputs::Targets(request_outputs) => request_outputs
                .iter()
                .any(|spec| Self::matches_spec(name, description, spec)),
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

    /// Returns the canonical short name (e.g. `eDP-1`) — used by the
    /// workspace visibility filter which compares against `w.monitor`.
    pub fn get_monitor_name(&self, id: SurfaceId) -> Option<&str> {
        self.entries.iter().find_map(|(key, info, _)| {
            info.as_ref().and_then(|info| {
                if info.id == id {
                    Some(key.name.as_str())
                } else {
                    None
                }
            })
        })
    }

    /// Strict canonical-name equality. Called by the workspace
    /// visibility filter with `w.monitor` (a compositor-canonical
    /// output name), where the substring fuzz used by `name_in_config`
    /// would re-introduce the very false-match bug this PR fixed
    /// (e.g. an event for `DP-1` matching an output whose description
    /// contains `"DP-1"` as a substring).
    pub fn has_name(&self, name: &str) -> bool {
        self.entries
            .iter()
            .any(|(key, info, _)| info.is_some() && key.name == name)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add<Message: 'static>(
        &mut self,
        style: AppearanceStyle,
        request_outputs: &config::Outputs,
        position: Position,
        layer: config::Layer,
        name: &str,
        description: &str,
        output_id: OutputId,
        scale_factor: f64,
    ) -> Task<Message> {
        let target = Self::name_in_config(name, description, request_outputs);

        if target {
            debug!("Found target output, creating a new layer surface");

            let (id, task) =
                Self::create_output_layers(style, Some(output_id), position, layer, scale_factor);

            // Replace an existing entry with the same canonical name
            // (e.g. monitor was reconnected). Description-match is
            // intentionally NOT used here — replacement is identity-
            // based, not alias-based.
            let destroy_task = match self
                .entries
                .iter()
                .position(|(key, _, _)| key.name.as_str() == name)
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

            let menu = self.make_menu();
            self.entries.push((
                OutputKey {
                    name: name.to_owned(),
                    description: description.to_owned(),
                },
                Some(ShellInfo {
                    id,
                    menu,
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

            self.entries.push((
                OutputKey {
                    name: name.to_owned(),
                    description: description.to_owned(),
                },
                None,
                Some(output_id),
            ));

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

                    let menu = self.make_menu();
                    self.entries.push((
                        OutputKey {
                            name: "Fallback".to_string(),
                            description: String::new(),
                        },
                        Some(ShellInfo {
                            id,
                            menu,
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
            .filter_map(|(key, shell_info, output_id)| {
                if !Self::name_in_config(&key.name, &key.description, request_outputs)
                    && shell_info.is_some()
                {
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
            .filter_map(|(key, shell_info, output_id)| {
                if Self::name_in_config(&key.name, &key.description, request_outputs)
                    && shell_info.is_none()
                {
                    Some((key.clone(), *output_id))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        debug!("Adding outputs: {to_add:?}");

        let mut tasks = Vec::new();

        for (key, output_id) in to_add {
            if let Some(output_id) = output_id {
                tasks.push(self.add(
                    style,
                    request_outputs,
                    position,
                    layer,
                    key.name.as_str(),
                    key.description.as_str(),
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
        let animations_enabled = self.animations_enabled;
        for (_name, shell_info, output_id) in &mut self.entries {
            if let Some(shell_info) = shell_info
                && shell_info.layer != layer
            {
                let old = shell_info.clone();
                let destroy_task = old.destroy_surfaces();

                let (id, create_task) =
                    Self::create_output_layers(style, *output_id, position, layer, scale_factor);

                shell_info.id = id;
                shell_info.menu = Menu::with_animations(animations_enabled);
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
    ) -> Option<&(OutputKey, Option<ShellInfo>, Option<OutputId>)> {
        self.entries.iter().find(|(_, shell_info, _)| {
            shell_info
                .as_ref()
                .is_some_and(|si| si.id == id || si.menu.surface_id() == Some(id))
        })
    }

    fn find_by_surface_id_mut(
        &mut self,
        id: SurfaceId,
    ) -> Option<&mut (OutputKey, Option<ShellInfo>, Option<OutputId>)> {
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

    pub fn menu_is_closing(&self, id: SurfaceId) -> bool {
        self.entries.iter().any(|(_, shell_info, _)| {
            shell_info
                .as_ref()
                .is_some_and(|si| si.menu.surface_id() == Some(id) && si.menu.is_closing())
        })
    }

    pub fn set_animations_enabled(&mut self, enabled: bool) {
        self.animations_enabled = enabled;
        for (_, shell_info, _) in self.entries.iter_mut() {
            if let Some(si) = shell_info.as_mut() {
                si.menu.set_animations_enabled(enabled);
            }
        }
    }

    pub fn finish_close_menu(&mut self, id: SurfaceId) -> Task<crate::app::Message> {
        if let Some((_, Some(shell_info), _)) = self.find_by_surface_id_mut(id) {
            shell_info.menu.finish_close()
        } else {
            Task::none()
        }
    }

    pub fn toggle_menu(
        &mut self,
        id: SurfaceId,
        menu_type: MenuType,
        button_ui_ref: ButtonUIRef,
        request_keyboard: bool,
    ) -> Task<crate::app::Message> {
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
    fn maybe_release_all_keyboards(
        &self,
        task: Task<crate::app::Message>,
        esc_button_enabled: bool,
    ) -> Task<crate::app::Message> {
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

    pub fn close_menu(
        &mut self,
        id: SurfaceId,
        menu_type: Option<MenuType>,
        esc_button_enabled: bool,
    ) -> Task<crate::app::Message> {
        let task = match self.find_by_surface_id_mut(id) {
            Some((_, Some(shell_info), _)) => match menu_type {
                Some(mt) => shell_info.menu.close_if(mt),
                None => shell_info.menu.close(),
            },
            _ => Task::none(),
        };

        self.maybe_release_all_keyboards(task, esc_button_enabled)
    }

    pub fn close_all_menu_if(
        &mut self,
        menu_type: MenuType,
        esc_button_enabled: bool,
    ) -> Task<crate::app::Message> {
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

    pub fn close_all_menus(&mut self, esc_button_enabled: bool) -> Task<crate::app::Message> {
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
        self.toast_width = width;

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
        // The surface is wider than a card to leave slide runway, and content is
        // aligned to the toast's horizontal edge, so the input region must follow.
        let x = match position {
            config::ToastPosition::TopLeft | config::ToastPosition::BottomLeft => 0,
            config::ToastPosition::TopRight | config::ToastPosition::BottomRight => {
                (self.toast_width as i32 - content_w).max(0)
            }
        };
        let y = match position {
            config::ToastPosition::TopLeft | config::ToastPosition::TopRight => 0,
            config::ToastPosition::BottomLeft | config::ToastPosition::BottomRight => self
                .toast_usable_height(toast.output)
                .map_or(0, |h| (h as i32) - content_h),
        };
        set_input_region(
            toast.id,
            Some(vec![InputRegionRect {
                x,
                y,
                width: content_w,
                height: content_h,
            }]),
        )
    }

    /// Output height minus the bar's exclusive zone, which the toast surface
    /// respects — so its usable height (for bottom-aligning) is shorter.
    fn toast_usable_height(&self, output_id: Option<OutputId>) -> Option<u32> {
        let target = output_id?;
        self.entries.iter().find_map(|(_, info, oid)| {
            if *oid == Some(target) {
                info.as_ref().and_then(|i| {
                    i.output_logical_height.map(|h| {
                        let bar = Self::get_height(i.style, i.scale_factor) as u32;
                        h.saturating_sub(bar)
                    })
                })
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
        for slot in [&mut self.toast, &mut self.osd].into_iter().flatten() {
            if slot.id == surface_id && slot.output == Some(output_id) {
                slot.output = None;
            }
        }
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
