pub mod clock;
pub mod settings;
pub mod system_info;
pub mod updates;
pub mod window_title;
pub mod workspaces;

use std::collections::HashSet;

use guido::prelude::*;

use crate::components::module_group::ModuleGroup;
use crate::components::{module_group, module_item};
use crate::config::{Config, ModuleDef, ModuleName, Modules, Position};
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
///
/// Menus are xdg popups anchored to the bar: the compositor positions
/// them, keeps them on screen, and dismisses them on outside click (grab).
/// No persistent fullscreen overlay surface is involved.
#[derive(Clone, Copy)]
pub struct MenuCtx {
    pub active_menu: RwSignal<Option<MenuType>>,
    /// The bar surface popups anchor to (set once after add_surface).
    pub bar_sid: RwSignal<Option<SurfaceId>>,
}

// The one open menu popup (only one menu can be open at a time).
thread_local! {
    static OPEN_POPUP: std::cell::RefCell<Option<PopupHandle>> =
        const { std::cell::RefCell::new(None) };
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Close the currently open menu popup, if any. Used as the `close`
/// callback handed to menu views (power actions, etc.); the popup's
/// dismissal effect resets `active_menu`.
pub fn close_open_menu() {
    let popup = OPEN_POPUP.with(|slot| slot.borrow_mut().take());
    if let Some(popup) = popup {
        popup.close();
    }
}

pub fn menu_width_for(mt: MenuType) -> f32 {
    match mt {
        MenuType::Settings => 350.0,
        _ => MENU_WIDTH,
    }
}

/// Popup size per menu (xdg positioners need an explicit size).
fn menu_size_for(mt: MenuType) -> (u32, u32) {
    let width = menu_width_for(mt) as u32;
    let height = match mt {
        MenuType::Settings => 620,
        MenuType::SystemInfo => 460,
        MenuType::Updates => 280,
    };
    (width, height)
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

/// Common popup chrome around a menu view.
fn menu_shell(content: AnyWidget) -> Container {
    let theme = expect_context::<crate::theme::ThemeColors>();
    container()
        .width(fill())
        .height(fill())
        .background(theme.background)
        .corner_radius(12)
        .padding(16)
        .overflow(Overflow::Hidden)
        .child(content)
}

fn menu_toggle(
    mt: MenuType,
    wr: WidgetRef,
    menu: MenuCtx,
    content: impl Fn() -> AnyWidget + Clone + 'static,
) -> impl Fn() + 'static {
    move || {
        let was_open = menu.active_menu.get() == Some(mt);
        // Whatever happens, the previous popup goes away
        close_open_menu();
        if was_open {
            menu.active_menu.set(None);
            return;
        }
        let Some(bar) = menu.bar_sid.get() else {
            return;
        };

        // Open downward from a top bar, upward from a bottom bar; the
        // compositor's flip adjustment covers the rest.
        let direction = match with_context::<Config, _>(|c| c.position).unwrap_or_default() {
            Position::Top => PopupAnchor::Bottom,
            Position::Bottom => PopupAnchor::Top,
        };

        let (width, height) = menu_size_for(mt);
        let content = content.clone();
        let popup = spawn_popup(
            bar,
            PopupConfig::new(width, height)
                .anchor_rect(wr.rect().get())
                .anchor(direction)
                .gravity(direction)
                .grab()
                .background_color(Color::TRANSPARENT),
            move || menu_shell(content()),
        );
        menu.active_menu.set(Some(mt));

        // Reset state when the compositor dismisses the popup (outside
        // click) or close_open_menu() runs — but only if this popup is
        // still the current one (the user may have switched menus).
        let popup_id = popup.id();
        create_effect(move || {
            if popup.dismissed() {
                OPEN_POPUP.with(|slot| {
                    let mut slot = slot.borrow_mut();
                    if slot.as_ref().is_some_and(|p| p.id() == popup_id) {
                        *slot = None;
                    }
                });
                if menu.active_menu.get_untracked() == Some(mt) {
                    menu.active_menu.set(None);
                }
            }
        })
        .detach();

        OPEN_POPUP.with(|slot| *slot.borrow_mut() = Some(popup));
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
                let content = move || system_info::menu_view(info).into_any();
                group.child(
                    container().widget_ref(wr).child(
                        module_item()
                            .on_click(menu_toggle(MenuType::SystemInfo, wr, menu, content))
                            .child(system_info::view(info)),
                    ),
                )
            } else {
                group
            }
        }
        ModuleName::Updates => {
            if let Some((d, svc)) = &data.updates {
                let wr = create_widget_ref();
                let (d, svc) = (*d, svc.clone());
                let content =
                    move || updates::menu_view(d, svc.clone(), close_open_menu).into_any();
                group.child(
                    container().widget_ref(wr).child(
                        module_item()
                            .on_click(menu_toggle(MenuType::Updates, wr, menu, content))
                            .child(updates::view(d)),
                    ),
                )
            } else {
                group
            }
        }
        ModuleName::Settings => {
            if let Some(s) = &data.settings {
                let wr = create_widget_ref();
                let s_menu = s.clone();
                let content =
                    move || settings::menu_view(s_menu.clone(), close_open_menu).into_any();
                group.child(
                    container().widget_ref(wr).child(
                        module_item()
                            .on_click(menu_toggle(MenuType::Settings, wr, menu, content))
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
