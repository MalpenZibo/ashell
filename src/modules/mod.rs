pub mod clock;
pub mod custom_module;
pub mod keyboard_layout;
pub mod keyboard_submap;
pub mod media_player;
pub mod notifications;
pub mod osd;
pub mod privacy;
pub mod settings;
pub mod system_info;
pub mod tempo;
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
    MediaPlayer,
    Tempo,
    Notifications,
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
    pub media_player: Option<media_player::MediaPlayerHandle>,
    pub tempo: Option<tempo::TempoHandle>,
    pub notifications: Option<notifications::NotificationsHandle>,
    pub custom: std::collections::HashMap<String, custom_module::CustomHandle>,
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

// The one open menu popup, its open/collapse animation signal, and the
// owner scope holding the popup's reactive resources.
thread_local! {
    static OPEN_POPUP: std::cell::RefCell<
        Option<(PopupHandle, RwSignal<bool>, guido::reactive::owner::OwnerId)>,
    > = const { std::cell::RefCell::new(None) };
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Clear the menu slot: called at the start of every app run, because a
/// config-reload restart resets guido's arenas but not this module's
/// thread_local — a stale PopupHandle/OwnerId here would act on recycled
/// ids belonging to the new run.
pub fn reset_menu_state() {
    OPEN_POPUP.with(|slot| slot.borrow_mut().take());
}

/// Begin closing the open menu: flip the collapse animation; the reverse
/// transition's on_complete destroys the popup when it settles. Used as
/// the `close` callback handed to menu views (power actions, etc.).
pub fn close_menu_fn(menu: MenuCtx) -> impl Fn() + Clone + 'static {
    move || {
        let open = OPEN_POPUP.with(|slot| slot.borrow().as_ref().map(|(_, open, _)| *open));
        let Some(open_sig) = open else {
            return;
        };
        open_sig.set(false);
        menu.active_menu.set(None);
    }
}

/// Destroy whatever popup is in the slot, now. Called by the collapse
/// animation's on_complete and by menu switches.
fn close_open_popup_now() {
    if let Some((popup, _, owner)) = OPEN_POPUP.with(|slot| slot.borrow_mut().take()) {
        popup.close();
        // Deferred disposal: the popup's widgets stay alive until the Close
        // command is processed; disposing now would leave live closures
        // reading dead signals
        guido::reactive::dispose_owner(owner);
    }
}

pub fn menu_width_for(mt: &MenuType) -> f32 {
    match mt {
        MenuType::Settings => 350.0,
        MenuType::MediaPlayer => 450.0,
        MenuType::Tempo => 650.0,
        MenuType::Notifications => 350.0,
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
    let (menu_opacity, blur) =
        with_context::<Config, _>(|c| (c.appearance.menu.opacity, c.appearance.blur))
            .unwrap_or((1.0, crate::config::BlurMode::Never));
    let bg = theme.background;
    let border_color = with_context::<Config, _>(|c| c.appearance.background_color.weak())
        .flatten()
        .unwrap_or_else(|| theme.background.lighter(0.15));
    let mut shell = container()
        .width(fill())
        .background(Color::rgba(bg.r, bg.g, bg.b, menu_opacity))
        .border(
            1,
            Color::rgba(border_color.r, border_color.g, border_color.b, menu_opacity),
        )
        .corner_radius(16);
    if blur.enabled(menu_opacity) {
        shell = shell.background_blur();
    }
    let collapsed = Transform::scale_xy(1.0, 0.0);
    shell
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
        .animate_transform_from(
            // ENTER plays from collapsed on first layout: `open` starts
            // true, no delay timer. The close is the reverse transition;
            // its on_complete destroys the popup — the animation itself is
            // the source of truth for "the collapse has finished".
            collapsed,
            Transition::spring(SpringConfig::SNAPPY).reverse(
                Transition::new(120, TimingFunction::CubicBezier(0.215, 0.61, 0.355, 1.0))
                    .on_complete(close_open_popup_now),
            ),
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
        // Untracked reads: this callback can be invoked from any context
        // (event dispatch, effects) and must never register subscriptions
        let was_open = menu.active_menu.get_untracked().as_ref() == Some(&mt);
        if was_open {
            // Plain close: play the collapse animation, deferred destroy
            close_menu_fn(menu)();
            return;
        }
        // A NEW popup is about to be created (menu switch, or reopen during
        // the close animation): destroy the previous one IMMEDIATELY, no
        // animation. xdg-shell requires a new grab to nest under the
        // currently grabbed popup — a still-mapped previous menu makes the
        // compositor kill the connection (not_the_topmost_popup). The Close
        // command is processed before the CreatePopup pushed below, so the
        // protocol sees destroy-then-create in order.
        if let Some((popup, _open, owner)) = OPEN_POPUP.with(|slot| slot.borrow_mut().take()) {
            popup.close();
            guido::reactive::dispose_owner(owner);
            menu.active_menu.set(None);
        }
        let Some(bar) = menu.bar_sid.get_untracked() else {
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
        let content = content.clone();
        let mt_effect = mt.clone();
        // Anchor to the bar's edge, not the module content rect: the module
        // stops at the bar padding and the menu would overlap the bar
        let mut anchor_rect = wr.rect().get_untracked();
        anchor_rect.y = 0.0;
        anchor_rect.height = crate::BAR_HEIGHT as f32;

        // Everything reactive the popup needs lives in its own owner scope,
        // disposed when the popup goes away — never in whatever scope this
        // callback happened to run under (an effect re-run would dispose the
        // open signal and leave the menu stuck collapsed and invisible).
        let ((popup, open_sig), owner_id) = guido::reactive::owner::with_owner(move || {
            // `open` starts TRUE: the shell's enter transition plays from
            // collapsed on its own — no delay timer, no flip task
            let open_sig = create_signal(true);
            let mut popup_config = PopupConfig::new(width)
                .anchor_rect(anchor_rect)
                .anchor(direction)
                .gravity(direction)
                .background_color(Color::TRANSPARENT);
            // Debug aid: without the grab the popup survives outside input
            if std::env::var("ASHELL_DEBUG_NO_GRAB").is_err() {
                popup_config = popup_config.grab();
            }
            let popup = spawn_popup(bar, popup_config, move || {
                menu_shell(content(), open_sig, origin)
            });

            // Reset state when the compositor dismisses the popup (outside
            // click) or close_open_menu() runs — but only if this popup is
            // still the current one (the user may have switched menus).
            let popup_id = popup.id();
            create_effect(move || {
                if popup.dismissed() {
                    OPEN_POPUP.with(|slot| {
                        let mut slot = slot.borrow_mut();
                        if let Some((p, _, owner)) = slot.as_ref()
                            && p.id() == popup_id
                        {
                            guido::reactive::dispose_owner(*owner);
                            *slot = None;
                        }
                    });
                    if menu.active_menu.get_untracked().as_ref() == Some(&mt_effect) {
                        menu.active_menu.set(None);
                    }
                }
            })
            .detach();

            (popup, open_sig)
        });

        menu.active_menu.set(Some(mt.clone()));
        OPEN_POPUP.with(|slot| *slot.borrow_mut() = Some((popup, open_sig, owner_id)));
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
                let content = move || {
                    // Submenus always start collapsed when the menu opens
                    s_menu.submenu.set(None);
                    settings::menu_view(s_menu.clone(), close.clone()).into_any()
                };
                // DEBUG: auto-open shortly after startup
                if std::env::var("ASHELL_DEBUG_OPEN_SETTINGS").is_ok() {
                    let trigger = menu_toggle(MenuType::Settings, wr, menu, content.clone());
                    // Value = delay in seconds (default 3) so menu-switch
                    // sequences can be scripted (tempo at 3s, settings later)
                    let delay = std::env::var("ASHELL_DEBUG_OPEN_SETTINGS")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(3u64);
                    let opened = create_signal(false);
                    let w = opened.writer();
                    tokio::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                        w.set(true);
                    });
                    create_effect(move || {
                        if opened.get() {
                            trigger();
                        }
                    })
                    .detach();
                }
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
                let blocklist =
                    with_context::<Config, _>(|c| c.tray.blocklist.clone()).unwrap_or_default();
                // Hidden entirely while no (non-blocklisted) item is registered
                group.child(container().height(fill()).child(move || {
                    let any_visible = items.with(|l| {
                        l.iter()
                            .any(|item| !blocklist.iter().any(|re| re.0.is_match(&item.name)))
                    });
                    any_visible.then(|| tray::view(items, svc.clone(), menu))
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
        ModuleName::MediaPlayer => {
            if let Some(mp) = &data.media_player {
                let (mp_data, mp_svc, mp_bars) = (mp.data, mp.svc.clone(), mp.bars);
                let config =
                    with_context::<Config, _>(|c| c.media_player.clone()).unwrap_or_default();

                // cava runs only while a visualizer can be seen and music
                // actually plays (upstream gates the subscription the same way)
                let wants_bar_viz = config.indicator_visualizer.is_some();
                let menu_viz = config.menu_visualizer;
                if wants_bar_viz || menu_viz {
                    let gate = mp.gate.clone();
                    let is_playing = create_memo(move || {
                        mp_data.with(|s| {
                            s.as_ref().is_some_and(|x| {
                                x.players().iter().any(|p| {
                                    p.state == crate::services::mpris::PlaybackStatus::Playing
                                })
                            })
                        })
                    });
                    let active_menu = menu.active_menu;
                    create_effect(move || {
                        let menu_open = matches!(active_menu.get(), Some(MenuType::MediaPlayer));
                        let wanted = (wants_bar_viz || (menu_viz && menu_open)) && is_playing.get();
                        let _ = gate.send(wanted);
                    })
                    .detach();
                }

                let menu_config = config.clone();
                let wr = create_widget_ref();
                let content = move || {
                    media_player::menu_view(mp_data, mp_svc.clone(), mp_bars, menu_config.clone())
                        .into_any()
                };
                // The whole module (and its click handler) hides while no
                // player is around
                group.child(container().height(fill()).widget_ref(wr).child(move || {
                    let has_players =
                        mp_data.with(|s| s.as_ref().is_some_and(|x| !x.players().is_empty()));
                    let config = config.clone();
                    let content = content.clone();
                    has_players.then(move || {
                        module_item()
                            .on_click(menu_toggle(MenuType::MediaPlayer, wr, menu, content))
                            .child(media_player::view(mp_data, mp_bars, config))
                    })
                }))
            } else {
                group
            }
        }
        ModuleName::KeyboardSubmap => {
            group.child(module_item().child(keyboard_submap::view(data.compositor_state)))
        }
        ModuleName::Tempo => {
            if let Some(t) = data.tempo {
                let wr = create_widget_ref();
                let content = move || tempo::menu_view(t).into_any();
                // DEBUG: auto-open the tempo menu shortly after startup
                // DEBUG: advance the calendar one month shortly after the
                // menu opens (month-change reposition repro)
                if std::env::var("ASHELL_DEBUG_NEXT_MONTH").is_ok() {
                    let step = create_signal(false);
                    let w = step.writer();
                    tokio::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_secs(6)).await;
                        w.set(true);
                    });
                    create_effect(move || {
                        if step.get() {
                            let cur = t
                                .selected_date
                                .get_untracked()
                                .unwrap_or_else(|| t.date.get_untracked().date_naive());
                            eprintln!("[repro] advancing month from {cur}");
                            t.selected_date
                                .set(cur.checked_add_months(chrono::Months::new(1)));
                        }
                    })
                    .detach();
                }
                if std::env::var("ASHELL_DEBUG_OPEN_TEMPO").is_ok() {
                    let trigger = menu_toggle(MenuType::Tempo, wr, menu, content.clone());
                    let step = create_signal(0u32);
                    let w = step.writer();
                    // With ASHELL_DEBUG_REOPEN_TEMPO: open at 3s, close at
                    // 6s, reopen at 8s — cold vs warm open comparison
                    let reopen = std::env::var("ASHELL_DEBUG_REOPEN_TEMPO").is_ok();
                    tokio::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                        w.set(1);
                        if reopen {
                            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                            w.set(2);
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            w.set(3);
                        }
                    });
                    create_effect(move || {
                        if step.get() > 0 {
                            eprintln!(
                                "[open-probe] t={} menu_toggle step {}",
                                crate::utils::debug_wall_ms(),
                                step.get()
                            );
                            trigger();
                        }
                    })
                    .detach();
                }
                group.child(
                    container().height(fill()).widget_ref(wr).child(
                        module_item()
                            .on_click(menu_toggle(MenuType::Tempo, wr, menu, content))
                            .child(tempo::view(t)),
                    ),
                )
            } else {
                group
            }
        }
        ModuleName::Notifications => {
            if let Some(n) = data.notifications {
                let wr = create_widget_ref();
                let content = move || notifications::menu_view(n).into_any();
                group.child(
                    container().height(fill()).widget_ref(wr).child(
                        module_item()
                            .on_click(menu_toggle(MenuType::Notifications, wr, menu, content))
                            .child(notifications::view(n)),
                    ),
                )
            } else {
                group
            }
        }
        ModuleName::Custom(name) => {
            if let Some(handle) = data.custom.get(name) {
                group.child(module_item().child(custom_module::view(handle.clone())))
            } else {
                group
            }
        }
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
