pub mod clock;
pub mod keyboard_layout;
pub mod keyboard_submap;
pub mod privacy;
pub mod settings;
pub mod system_info;
pub mod tray;
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

#[derive(Clone, PartialEq)]
pub enum MenuType {
    SystemInfo,
    Updates,
    Settings,
    /// Tray item menu, keyed by the item's service name
    Tray(String),
}

/// All module data — mirrors what main.rs used to hold inline.
pub struct ModuleData {
    pub compositor_state: CompositorStateSignals,
    pub compositor_svc: Service<CompositorCommand>,
    pub system_info: Option<SystemInfoDataSignals>,
    pub updates: Option<(UpdatesDataSignals, Service<UpdatesCmd>)>,
    pub settings: Option<SettingsSignals>,
    pub tray: Option<tray::TrayHandle>,
    pub privacy:
        Option<crate::services::compat::ServiceSignal<crate::services::privacy::PrivacyService>>,
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
    /// Written from a delayed tokio task so the popup closes only after
    /// the collapse animation has played (see finish_menu_close).
    pub pending_close_writer: WriteSignal<bool>,
}

// The one open menu popup + its open/collapse animation signal.
thread_local! {
    static OPEN_POPUP: std::cell::RefCell<Option<(PopupHandle, RwSignal<bool>)>> =
        const { std::cell::RefCell::new(None) };
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Time the collapse animation gets before the popup surface is closed.
const MENU_CLOSE_ANIM: std::time::Duration = std::time::Duration::from_millis(220);
/// Delay before flipping the open signal so the first frame renders
/// collapsed and the expand animation actually plays.
const MENU_OPEN_DELAY: std::time::Duration = std::time::Duration::from_millis(30);

/// Begin closing the open menu: play the collapse animation, then let the
/// deferred pending_close effect destroy the popup. Used as the `close`
/// callback handed to menu views (power actions, etc.).
pub fn close_menu_fn(menu: MenuCtx) -> impl Fn() + Clone + 'static {
    move || {
        let open_sig = OPEN_POPUP.with(|slot| slot.borrow().as_ref().map(|(_, open)| *open));
        let Some(open_sig) = open_sig else {
            return;
        };
        open_sig.set(false);
        menu.active_menu.set(None);
        let writer = menu.pending_close_writer;
        tokio::spawn(async move {
            tokio::time::sleep(MENU_CLOSE_ANIM).await;
            writer.set(true);
        });
    }
}

/// Destroy the open menu popup (after the collapse animation). Called by
/// the pending_close effect in main.rs.
pub fn finish_menu_close() {
    let popup = OPEN_POPUP.with(|slot| slot.borrow_mut().take());
    if let Some((popup, _)) = popup {
        popup.close();
    }
}

pub fn menu_width_for(mt: &MenuType) -> f32 {
    match mt {
        MenuType::Settings => 350.0,
        _ => MENU_WIDTH,
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

/// Common popup chrome around a menu view: background, radius, padding and
/// the expand/collapse animation (spring open, ease-out close) scaling from
/// the bar edge. No fill-height — auto-height popups size to the content.
fn menu_shell(content: AnyWidget, open: RwSignal<bool>, origin: TransformOrigin) -> Container {
    let theme = expect_context::<crate::theme::ThemeColors>();
    container()
        .width(fill())
        .background(theme.background)
        .corner_radius(12)
        .padding(16)
        .overflow(Overflow::Hidden)
        .transform(move || {
            if open.get() {
                Transform::IDENTITY
            } else {
                Transform::scale_xy(1.0, 0.0)
            }
        })
        .transform_origin(origin)
        .animate_transform(
            Transition::spring(SpringConfig::DEFAULT)
                .reverse(Transition::new(200, TimingFunction::EaseOut)),
        )
        .child(content)
}

fn menu_toggle(
    mt: MenuType,
    wr: WidgetRef,
    menu: MenuCtx,
    content: impl Fn() -> AnyWidget + Clone + 'static,
) -> impl Fn() + 'static {
    move || {
        let was_open = menu.active_menu.get().as_ref() == Some(&mt);
        // Whatever happens, the previous popup goes away (animated)
        close_menu_fn(menu)();
        if was_open {
            return;
        }
        let Some(bar) = menu.bar_sid.get() else {
            return;
        };

        // Open downward from a top bar, upward from a bottom bar; the
        // animation scales from the bar edge accordingly.
        let (direction, origin) =
            match with_context::<Config, _>(|c| c.position).unwrap_or_default() {
                Position::Top => (PopupAnchor::Bottom, TransformOrigin::TOP),
                Position::Bottom => (PopupAnchor::Top, TransformOrigin::BOTTOM),
            };

        let width = menu_width_for(&mt) as u32;
        // Starts collapsed; flipped just after mapping so the expand plays
        let open_sig = create_signal(false);
        let content = content.clone();
        let popup = spawn_popup(
            bar,
            PopupConfig::new(width)
                .anchor_rect(wr.rect().get())
                .anchor(direction)
                .gravity(direction)
                .grab()
                .background_color(Color::TRANSPARENT),
            move || menu_shell(content(), open_sig, origin),
        );
        menu.active_menu.set(Some(mt.clone()));
        let open_writer = open_sig.writer();
        tokio::spawn(async move {
            tokio::time::sleep(MENU_OPEN_DELAY).await;
            open_writer.set(true);
        });

        // Reset state when the compositor dismisses the popup (outside
        // click) or close_open_menu() runs — but only if this popup is
        // still the current one (the user may have switched menus).
        let popup_id = popup.id();
        let mt_effect = mt.clone();
        create_effect(move || {
            if popup.dismissed() {
                OPEN_POPUP.with(|slot| {
                    let mut slot = slot.borrow_mut();
                    if slot.as_ref().is_some_and(|(p, _)| p.id() == popup_id) {
                        *slot = None;
                    }
                });
                if menu.active_menu.get_untracked().as_ref() == Some(&mt_effect) {
                    menu.active_menu.set(None);
                }
            }
        })
        .detach();

        OPEN_POPUP.with(|slot| *slot.borrow_mut() = Some((popup, open_sig)));
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
                let close = close_menu_fn(menu);
                let content = move || updates::menu_view(d, svc.clone(), close.clone()).into_any();
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
                let close = close_menu_fn(menu);
                let content = move || settings::menu_view(s_menu.clone(), close.clone()).into_any();
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
        ModuleName::Tray => {
            if let Some((items, svc)) = &data.tray {
                let (items, svc) = (*items, svc.clone());
                // Hidden entirely while no tray item is registered
                group.child(container().height(fill()).child(move || {
                    (!items.with(|l| l.is_empty())).then(|| tray::view(items, svc.clone(), menu))
                }))
            } else {
                group
            }
        }
        ModuleName::Privacy => {
            if let Some(data) = data.privacy {
                group.child(module_item().child(privacy::view(data)))
            } else {
                group
            }
        }
        ModuleName::KeyboardLayout => group.child(module_item().child(keyboard_layout::view(
            data.compositor_state,
            data.compositor_svc.clone(),
        ))),
        ModuleName::KeyboardSubmap => {
            group.child(module_item().child(keyboard_submap::view(data.compositor_state)))
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
