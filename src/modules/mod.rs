pub mod clock;
pub mod settings;
pub mod system_info;
pub mod updates;
pub mod window_title;
pub mod workspaces;

use std::collections::HashSet;
use std::time::Duration;

use guido::prelude::*;

use crate::components::module_group::ModuleGroup;
use crate::components::{module_group, module_item};
use crate::config::{ModuleDef, ModuleName, Modules};
use crate::services::compositor::{CompositorCommand, CompositorStateSignals};
use crate::services::system_info::SystemInfoDataSignals;
use crate::services::updates::{UpdatesCmd, UpdatesDataSignals};

pub use self::settings::SettingsSignals;

// ── Constants & types ────────────────────────────────────────────────────────

pub const MENU_WIDTH: f32 = 300.0;

#[derive(Clone, Copy, PartialEq)]
pub enum MenuType {
    SystemInfo,
    Updates,
    Settings,
}

/// All module data — mirrors what main.rs used to hold inline.
pub struct ModuleData {
    pub compositor_state: CompositorStateSignals,
    pub compositor_svc: Service<CompositorCommand>,
    pub system_info: Option<SystemInfoDataSignals>,
    pub updates: Option<(UpdatesDataSignals, Service<UpdatesCmd>)>,
    pub settings: Option<SettingsSignals>,
}

/// Menu infrastructure signals (all Copy).
#[derive(Clone, Copy)]
pub struct MenuCtx {
    pub active_menu: Signal<Option<MenuType>>,
    pub displayed_menu: Signal<Option<MenuType>>,
    pub menu_x: Signal<f32>,
    pub menu_sid: Signal<Option<SurfaceId>>,
    pub backdrop_ref: WidgetRef,
    /// Written from a delayed tokio task to trigger surface hide after close animation.
    pub surface_hide_writer: WriteSignal<bool>,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

pub fn toggle_menu_surface(id: SurfaceId, open: bool) {
    let handle = surface_handle(id);
    if open {
        handle.set_layer(Layer::Overlay);
        handle.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
    } else {
        handle.set_layer(Layer::Background);
        handle.set_keyboard_interactivity(KeyboardInteractivity::None);
    }
}

pub fn menu_width_for(mt: MenuType) -> f32 {
    match mt {
        MenuType::Settings => 350.0,
        _ => MENU_WIDTH,
    }
}

/// Duration to keep the surface visible while the close animation plays.
const MENU_CLOSE_DELAY: Duration = Duration::from_millis(500);

pub fn close_menu_fn(menu: MenuCtx) -> impl Fn() + Clone + 'static {
    move || {
        menu.active_menu.set(None);
        let writer = menu.surface_hide_writer;
        tokio::spawn(async move {
            tokio::time::sleep(MENU_CLOSE_DELAY).await;
            writer.set(true);
        });
    }
}

/// Collect all module names referenced by a config's module layout.
pub fn modules_in_config(modules: &Modules) -> HashSet<ModuleName> {
    let mut set = HashSet::new();
    for defs in [&modules.left, &modules.center, &modules.right] {
        for def in defs {
            match def {
                ModuleDef::Single(name) => {
                    set.insert(name.clone());
                }
                ModuleDef::Group(names) => {
                    for name in names {
                        set.insert(name.clone());
                    }
                }
            }
        }
    }
    set
}

// ── Menu toggle callback ─────────────────────────────────────────────────────

fn menu_toggle(mt: MenuType, wr: WidgetRef, menu: MenuCtx) -> impl Fn() + 'static {
    move || {
        let new = match menu.active_menu.get() {
            Some(m) if m == mt => None,
            _ => Some(mt),
        };
        if let Some(mt) = new {
            // Opening
            let r = wr.rect().get();
            let screen_w = menu.backdrop_ref.rect().get().width;
            let w = menu_width_for(mt);
            let center = r.x + r.width / 2.0;
            menu.menu_x
                .set((center - w / 2.0).max(8.0).min(screen_w - w - 8.0));
            menu.displayed_menu.set(Some(mt));
            menu.active_menu.set(Some(mt));
            if let Some(id) = menu.menu_sid.get() {
                toggle_menu_surface(id, true);
            }
        } else {
            // Closing — delay surface hide so animation plays
            menu.active_menu.set(None);
            let writer = menu.surface_hide_writer;
            tokio::spawn(async move {
                tokio::time::sleep(MENU_CLOSE_DELAY).await;
                writer.set(true);
            });
        }
    }
}

// ── Module dispatch ──────────────────────────────────────────────────────────

/// Add a module's bar view to a group. Returns the group unchanged for
/// unimplemented or unavailable modules.
fn add_module(
    group: ModuleGroup,
    name: &ModuleName,
    data: &ModuleData,
    menu: MenuCtx,
) -> ModuleGroup {
    match name {
        ModuleName::Clock => group.child(module_item().child(clock::view())),
        ModuleName::Workspaces => group.child(module_item().child(workspaces::view(
            data.compositor_state,
            data.compositor_svc.clone(),
        ))),
        ModuleName::WindowTitle => {
            let state = data.compositor_state;
            group.child(container().child(move || {
                state
                    .active_window
                    .with(|w| w.is_some())
                    .then(|| module_item().child(window_title::view(state)))
            }))
        }
        ModuleName::SystemInfo => {
            if let Some(info) = data.system_info {
                let wr = create_widget_ref();
                group.child(
                    container().widget_ref(wr).child(
                        module_item()
                            .on_click(menu_toggle(MenuType::SystemInfo, wr, menu))
                            .child(system_info::view(info)),
                    ),
                )
            } else {
                group
            }
        }
        ModuleName::Updates => {
            if let Some((d, _)) = &data.updates {
                let wr = create_widget_ref();
                group.child(
                    container().widget_ref(wr).child(
                        module_item()
                            .on_click(menu_toggle(MenuType::Updates, wr, menu))
                            .child(updates::view(*d)),
                    ),
                )
            } else {
                group
            }
        }
        ModuleName::Settings => {
            if let Some(s) = &data.settings {
                let wr = create_widget_ref();
                group.child(
                    container().widget_ref(wr).child(
                        module_item()
                            .on_click(menu_toggle(MenuType::Settings, wr, menu))
                            .child(settings::view(s.clone())),
                    ),
                )
            } else {
                group
            }
        }
        // Unimplemented modules are silently skipped.
        _ => group,
    }
}

// ── Section builder ──────────────────────────────────────────────────────────

/// Build a left/center/right section from config definitions.
pub fn build_section(defs: &[ModuleDef], data: &ModuleData, menu: MenuCtx) -> impl Widget + use<> {
    let mut section = container()
        .layout(
            Flex::row()
                .spacing(4)
                .cross_alignment(CrossAlignment::Center),
        )
        .height(fill());

    for def in defs {
        match def {
            ModuleDef::Single(name) => {
                section = section.child(add_module(module_group(), name, data, menu));
            }
            ModuleDef::Group(names) => {
                let mut group = module_group();
                for name in names {
                    group = add_module(group, name, data, menu);
                }
                section = section.child(group);
            }
        }
    }
    section
}
