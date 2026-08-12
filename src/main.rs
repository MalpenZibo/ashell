mod components;
pub mod config;
mod config_watcher;
pub mod i18n;
pub mod ipc;
mod modules;
mod services;
mod utils;
pub mod xdg;

use components::center_box;
use config::ModuleName;
use guido::prelude::*;
use modules::{MenuCtx, MenuType, ModuleData, modules_in_config};
use services::compositor::{CompositorState, CompositorStateSignals, start_compositor_service};

pub mod theme {
    use guido::prelude::Color;

    use crate::config::Appearance;

    const DEFAULT_YELLOW: Color = Color::rgb(249.0 / 255.0, 226.0 / 255.0, 175.0 / 255.0);

    #[derive(Clone, Copy)]
    pub struct ThemeColors {
        pub text: Color,
        pub background: Color,
        pub primary: Color,
        pub success: Color,
        pub warning: Color,
        pub danger: Color,
    }

    pub fn init(appearance: &Appearance) -> ThemeColors {
        ThemeColors {
            text: appearance.text_color.base(),
            background: appearance.background_color.base(),
            primary: appearance.primary_color.base(),
            success: appearance.success_color.base(),
            danger: appearance.danger_color.base(),
            warning: appearance.danger_color.weak().unwrap_or(DEFAULT_YELLOW),
        }
    }
}

use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub enum IndicatorState {
    Normal,
    Success,
    Warning,
    Danger,
}

pub fn format_duration(duration: &Duration) -> String {
    let h = duration.as_secs() / 60 / 60;
    let m = duration.as_secs() / 60 % 60;
    if h > 0 {
        format!("{h}h {m:>2}m")
    } else {
        format!("{m:>2}m")
    }
}

// Upstream ashell's grapheme-aware middle-ellipsis truncation
pub fn truncate_text(value: &str, max_length: u32) -> String {
    use unicode_segmentation::UnicodeSegmentation;

    let graphemes = value.graphemes(true).collect::<Vec<&str>>();
    let length = graphemes.len();

    if length > max_length as usize {
        let split = max_length as usize / 2;
        let first_part = graphemes[..split].concat();
        let last_part = graphemes[length - split..].concat();
        format!("{first_part}...{last_part}")
    } else {
        value.to_string()
    }
}

/// Execute an IPC command through the settings services, then flash the OSD
/// (mirrors upstream's osd_info_for: values are computed optimistically from
/// the cached service state, mute/airplane toggles invert the current value).
const NERD_FONT: &[u8] = include_bytes!("../target/generated/SymbolsNerdFont-Regular-Subset.ttf");
const NERD_FONT_MONO: &[u8] =
    include_bytes!("../target/generated/SymbolsNerdFontMono-Regular-Subset.ttf");
const CUSTOM_FONT: &[u8] = include_bytes!("../assets/AshellCustomIcon-Regular.otf");

#[derive(clap::Parser, Debug)]
#[command(version, about = "ashell, ported to guido")]
struct Args {
    /// Path to the config file (default ~/.config/ashell/config.toml)
    #[arg(short, long)]
    config_path: Option<String>,
    #[command(subcommand)]
    command: Option<CliCommand>,
}

#[derive(clap::Subcommand, Debug)]
enum CliCommand {
    /// Send a message to a running ashell instance
    Msg {
        #[command(subcommand)]
        command: ipc::IpcCommand,
    },
}

/// Bar height in logical pixels — the single source for the surface
/// size, its exclusive zone, and the popup anchor rect in modules.
pub const BAR_HEIGHT: u32 = 34;

#[tokio::main]
async fn main() {
    use clap::Parser;
    let args = Args::parse();

    // Client mode: send the command to the running instance and exit
    if let Some(CliCommand::Msg { command }) = args.command {
        match ipc::run_client(&command) {
            Ok(()) => std::process::exit(0),
            Err(e) => {
                eprintln!("Error: {e:#}");
                std::process::exit(1);
            }
        }
    }

    env_logger::init();

    // -c/--config-path, then ASHELL_CONFIG_PATH (bench harness), then default
    let custom_config = args
        .config_path
        .or_else(|| std::env::var("ASHELL_CONFIG_PATH").ok());
    let config_path = config_watcher::resolve_config_path(custom_config.as_deref());
    config_watcher::ensure_config_dir(&config_path);

    loop {
        // A restart resets guido's arenas but not this crate's statics:
        // clear the menu slot so a menu left open across a config reload
        // can't act on recycled ids of the new run
        modules::reset_menu_state();
        load_font(NERD_FONT.to_vec());
        load_font(NERD_FONT_MONO.to_vec());
        load_font(CUSTOM_FONT.to_vec());

        let cfg = config::load_config(&config_path);

        // The embedded nerd fonts are subsets covering only the glyphs in
        // icons.rs; config-defined icons (custom modules, custom buttons)
        // can use any glyph, so pull in the full system font for those
        let needs_full_nerd = cfg
            .custom_modules
            .iter()
            .any(|m| m.icon.is_some() || m.icons.is_some())
            || !cfg.settings.custom_buttons.is_empty();
        if needs_full_nerd {
            match std::process::Command::new("fc-match")
                .args(["--format=%{file}", "Symbols Nerd Font"])
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .filter(|p| !p.is_empty())
                .and_then(|p| std::fs::read(p).ok())
            {
                Some(bytes) => load_font(bytes),
                None => log::warn!(
                    "config uses custom nerd-font glyphs but no system \"Symbols Nerd Font\" was found; they may render blank"
                ),
            }
        }
        let theme_colors = theme::init(&cfg.appearance);
        i18n::init_localizer(i18n::Localizer::resolve(
            cfg.language.as_deref(),
            cfg.region.as_deref(),
        ));

        let watcher_handle = config_watcher::spawn_config_watcher(config_path.clone());

        let reason = App::new().run(|app| {
            provide_context(cfg.clone());
            provide_context(theme_colors);

            let compositor_state = CompositorStateSignals::new(CompositorState::default());
            let compositor_svc = start_compositor_service(compositor_state.writers());

            // Only create expensive services for modules actually in the config
            let needed = modules_in_config(&cfg.modules);

            // Menu state — menus are xdg popups anchored to the bar; the
            // compositor positions and dismisses them (no overlay surface)
            let menu = MenuCtx {
                active_menu: create_signal(None::<MenuType>),
                bar_sid: create_signal(None::<SurfaceId>),
            };

            // While the system-info menu is open the sysinfo sampling widens
            // to all domains (disks, network); closed, only the configured
            // bar indicators are refreshed. The service watches the memo, so
            // it wakes the moment the menu opens instead of polling a flag.
            let sysinfo_wide =
                create_memo(move || matches!(menu.active_menu.get(), Some(MenuType::SystemInfo)));

            let system_info = needed
                .contains(&ModuleName::SystemInfo)
                .then(|| modules::system_info::create(sysinfo_wide.watch()));

            let updates = (needed.contains(&ModuleName::Updates) && cfg.updates.is_some())
                .then(modules::updates::create);

            let settings = needed
                .contains(&ModuleName::Settings)
                .then(modules::settings::create);

            let tray = needed
                .contains(&ModuleName::Tray)
                .then(modules::tray::create);

            let privacy = needed
                .contains(&ModuleName::Privacy)
                .then(modules::privacy::create);

            let media_player = needed
                .contains(&ModuleName::MediaPlayer)
                .then(modules::media_player::create);

            let tempo = needed
                .contains(&ModuleName::Tempo)
                .then(modules::tempo::create);

            let notifications = needed
                .contains(&ModuleName::Notifications)
                .then(modules::notifications::create);

            // One instance per custom module referenced in the layout
            let custom: std::collections::HashMap<_, _> = cfg
                .custom_modules
                .iter()
                .filter(|def| needed.contains(&ModuleName::Custom(def.name.clone())))
                .map(|def| {
                    (
                        def.name.clone(),
                        modules::custom_module::create(def.clone()),
                    )
                })
                .collect();

            let data = ModuleData {
                compositor_state,
                compositor_svc,
                system_info,
                updates,
                settings,
                tray,
                privacy,
                media_player,
                tempo,
                notifications,
                custom,
            };

            // OSD + IPC: commands arrive on the unix socket into a queue
            // (an event stream must not lose emissions); a pulse signal
            // wakes the main-thread dispatcher, which drains the WHOLE
            // queue — a burst of volume-ups is processed command by command
            let osd = modules::osd::create();
            let ipc_queue = ipc::IpcQueue::default();
            let ipc_pulse = create_signal(());
            let pulse_w = ipc_pulse.writer();
            let _ipc = create_service::<(), _, _>({
                let queue = ipc_queue.clone();
                move |_rx, _ctx| async move {
                    ipc::serve(queue, pulse_w).await;
                }
            });
            let bar_visible = create_signal(true);
            {
                let settings = settings;
                let osd = osd.clone();
                let volume_step = cfg.settings.volume_step;
                let max_volume = cfg.settings.max_volume;
                create_effect(move || {
                    ipc_pulse.get();
                    loop {
                        let cmd = ipc_queue.lock().unwrap().pop_front();
                        let Some(cmd) = cmd else { break };
                        if matches!(cmd, ipc::IpcCommand::ToggleVisibility) {
                            bar_visible.update(|v| *v = !*v);
                            continue;
                        }
                        modules::settings::handle_ipc_command(
                            &cmd,
                            settings.as_ref(),
                            &osd,
                            volume_step,
                            max_volume,
                        );
                    }
                })
                .detach();
            }

            // Bar surfaces: one per configured output (hotplug-aware), or a
            // single compositor-placed bar for Outputs::Active
            let position_anchor = match cfg.position {
                config::Position::Top => Anchor::TOP,
                config::Position::Bottom => Anchor::BOTTOM,
            };
            let _ = app; // surfaces are spawned dynamically below
            let bar_layer = match cfg.layer {
                config::Layer::Bottom => Layer::Bottom,
                config::Layer::Top => Layer::Top,
                config::Layer::Overlay => Layer::Overlay,
            };
            // Solid surface: the bar itself carries the (possibly translucent,
            // possibly blurred) background; Transparent leaves it to the islands
            let bar_bg = match cfg.appearance.bar.surface {
                config::BarSurface::Solid => {
                    let c = cfg.appearance.background_color.base();
                    Color::rgba(c.r, c.g, c.b, cfg.appearance.bar.opacity)
                }
                config::BarSurface::Transparent => Color::TRANSPARENT,
            };
            let data = std::rc::Rc::new(data);
            let active_menu = menu.active_menu;
            let bar_ids: std::rc::Rc<std::cell::RefCell<Vec<SurfaceId>>> = Default::default();

            let make_bar = {
                let data = data.clone();
                let cfg = cfg.clone();
                let bar_ids = bar_ids.clone();
                move |output: Option<OutputId>| -> SurfaceHandle {
                    // Menus anchor their popups to the bar that was clicked
                    let bar_sid = create_signal(None::<SurfaceId>);
                    let bar_menu = MenuCtx {
                        active_menu,
                        bar_sid,
                    };
                    let mut sc = SurfaceConfig::new()
                        .height(BAR_HEIGHT)
                        .anchor(position_anchor | Anchor::LEFT | Anchor::RIGHT)
                        .layer(bar_layer)
                        .exclusive_zone(BAR_HEIGHT)
                        .background_color(bar_bg)
                        .keyboard_interactivity(KeyboardInteractivity::None)
                        .namespace("ashell");
                    if let Some(o) = output {
                        sc = sc.output(o);
                    }
                    let cfg = cfg.clone();
                    let data = data.clone();
                    let handle = spawn_surface(sc, move || {
                        // toggle-visibility (IPC) empties the bar
                        container().child(move || {
                            bar_visible.get().then(|| {
                                container()
                                    .child(
                                        center_box()
                                            .left(modules::build_section(
                                                &cfg.modules.left,
                                                &data,
                                                bar_menu,
                                            ))
                                            .center(modules::build_section(
                                                &cfg.modules.center,
                                                &data,
                                                bar_menu,
                                            ))
                                            .right(modules::build_section(
                                                &cfg.modules.right,
                                                &data,
                                                bar_menu,
                                            )),
                                    )
                                    .padding([4, 0])
                            })
                        })
                    });
                    bar_sid.set(Some(handle.id()));
                    bar_ids.borrow_mut().push(handle.id());
                    handle
                }
            };

            match cfg.outputs.clone() {
                config::Outputs::Active => {
                    make_bar(None);
                }
                mode => {
                    // One bar per matching output; a single compositor-placed
                    // bar until output info arrives (or nothing matches)
                    let bars: std::rc::Rc<
                        std::cell::RefCell<
                            std::collections::HashMap<Option<OutputId>, SurfaceHandle>,
                        >,
                    > = Default::default();
                    let make_bar = make_bar.clone();
                    let bar_ids = bar_ids.clone();
                    create_effect(move || {
                        let outs = outputs().get();
                        // No output info yet (or none matches below): a
                        // compositor-placed fallback bar keeps the app usable;
                        // it hands over to per-output bars once outputs arrive
                        let desired: Vec<Option<OutputId>> = if outs.is_empty() {
                            vec![None]
                        } else {
                            match &mode {
                                config::Outputs::All => outs.iter().map(|o| Some(o.id)).collect(),
                                config::Outputs::Targets(targets) => outs
                                    .iter()
                                    .filter(|o| {
                                        let name = o.name.as_deref().unwrap_or("");
                                        let description =
                                            format!("{} {} {}", name, o.make, o.model);
                                        // Exact connector match or EDID substring
                                        targets
                                            .iter()
                                            .any(|t| name == t.as_str() || description.contains(t))
                                    })
                                    .map(|o| Some(o.id))
                                    .collect(),
                                config::Outputs::Active => unreachable!(),
                            }
                        };
                        let mut bars = bars.borrow_mut();
                        // Spawn before closing: dropping to zero surfaces
                        // (fallback -> pinned handover) would exit the app
                        for key in &desired {
                            bars.entry(*key).or_insert_with(|| make_bar(*key));
                        }
                        bars.retain(|key, handle| {
                            if desired.contains(key) {
                                true
                            } else {
                                bar_ids.borrow_mut().retain(|id| *id != handle.id());
                                handle.close();
                                false
                            }
                        });
                    })
                    .detach();
                }
            }

            // Hidden bars give up their exclusive zone
            create_effect(move || {
                let visible = bar_visible.get();
                for id in bar_ids.borrow().iter() {
                    surface_handle(*id).set_exclusive_zone(if visible {
                        BAR_HEIGHT as i32
                    } else {
                        0
                    });
                }
            })
            .detach();
        });

        // App is dropped here, cleaning up all state

        watcher_handle.abort();

        match reason {
            ExitReason::Quit => break,
            ExitReason::Restart => {
                log::info!("Restarting application...");
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                continue;
            }
            ExitReason::Error(e) => {
                log::error!("Platform error: {e}");
                std::process::exit(1);
            }
        }
    }
}
