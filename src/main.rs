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
fn handle_ipc_command(
    cmd: &ipc::IpcCommand,
    settings: Option<&modules::SettingsSignals>,
    osd: &modules::osd::OsdTrigger,
    volume_step: u8,
    max_volume: u8,
    bar_visible: RwSignal<bool>,
) {
    use ipc::IpcCommand;
    use modules::osd::OsdKind;

    const NORMAL_VOLUME: u32 = libpulse_binding::volume::Volume::NORMAL.0;

    if matches!(cmd, IpcCommand::ToggleVisibility) {
        bar_visible.update(|v| *v = !*v);
        return;
    }

    let Some(s) = settings else {
        log::warn!("IPC command {cmd} ignored: settings services not running (no Settings module)");
        return;
    };
    let no_osd = cmd.no_osd();
    let show = |kind, value, scale, flag| {
        if !no_osd {
            osd.show(kind, value, scale, flag);
        }
    };

    let vol_max = NORMAL_VOLUME * u32::from(max_volume.clamp(1, 200)) / 100;
    let vol_scale = (vol_max as f32 / NORMAL_VOLUME as f32).max(1.0);
    let step = u32::from(volume_step.clamp(1, 50)) * (NORMAL_VOLUME / 100);
    let mic_step = 5 * (NORMAL_VOLUME / 100);

    match cmd {
        IpcCommand::VolumeUp { .. } | IpcCommand::VolumeDown { .. } => {
            let Some((cur, muted)) = s.audio_data.with(|a| {
                a.as_ref().map(|x| {
                    (
                        x.sink_slider.value(),
                        x.active_sink().map(|d| d.is_mute).unwrap_or(false),
                    )
                })
            }) else {
                return;
            };
            let new = if matches!(cmd, IpcCommand::VolumeUp { .. }) {
                (cur + step).min(vol_max)
            } else {
                cur.saturating_sub(step)
            };
            s.audio_svc
                .send(services::audio::AudioCommand::SinkVolume(new));
            show(
                OsdKind::Volume,
                new as f32 / NORMAL_VOLUME as f32,
                vol_scale,
                muted,
            );
        }
        IpcCommand::VolumeToggleMute { .. } => {
            let Some((cur, muted)) = s.audio_data.with(|a| {
                a.as_ref().map(|x| {
                    (
                        x.sink_slider.value(),
                        x.active_sink().map(|d| d.is_mute).unwrap_or(false),
                    )
                })
            }) else {
                return;
            };
            s.audio_svc
                .send(services::audio::AudioCommand::ToggleSinkMute);
            show(
                OsdKind::Volume,
                cur as f32 / NORMAL_VOLUME as f32,
                vol_scale,
                !muted,
            );
        }
        IpcCommand::MicrophoneUp { .. } | IpcCommand::MicrophoneDown { .. } => {
            let Some((cur, muted)) = s.audio_data.with(|a| {
                a.as_ref().map(|x| {
                    (
                        x.source_slider.value(),
                        x.active_source().map(|d| d.is_mute).unwrap_or(false),
                    )
                })
            }) else {
                return;
            };
            let new = if matches!(cmd, IpcCommand::MicrophoneUp { .. }) {
                (cur + mic_step).min(NORMAL_VOLUME)
            } else {
                cur.saturating_sub(mic_step)
            };
            s.audio_svc
                .send(services::audio::AudioCommand::SourceVolume(new));
            show(
                OsdKind::Microphone,
                new as f32 / NORMAL_VOLUME as f32,
                1.0,
                muted,
            );
        }
        IpcCommand::MicrophoneToggleMute { .. } => {
            let Some((cur, muted)) = s.audio_data.with(|a| {
                a.as_ref().map(|x| {
                    (
                        x.source_slider.value(),
                        x.active_source().map(|d| d.is_mute).unwrap_or(false),
                    )
                })
            }) else {
                return;
            };
            s.audio_svc
                .send(services::audio::AudioCommand::ToggleSourceMute);
            show(
                OsdKind::Microphone,
                cur as f32 / NORMAL_VOLUME as f32,
                1.0,
                !muted,
            );
        }
        IpcCommand::BrightnessUp { .. } | IpcCommand::BrightnessDown { .. } => {
            let Some((cur, max)) = s
                .brightness_data
                .with(|b| b.as_ref().map(|x| (x.current.value(), x.max)))
            else {
                return;
            };
            if max == 0 {
                return;
            }
            let step = (5 * max / 100).max(1);
            let new = if matches!(cmd, IpcCommand::BrightnessUp { .. }) {
                (cur + step).min(max)
            } else {
                cur.saturating_sub(step)
            };
            s.brightness_svc
                .send(services::brightness::BrightnessCommand(new));
            show(OsdKind::Brightness, new as f32 / max as f32, 1.0, false);
        }
        IpcCommand::ToggleAirplaneMode { .. } => {
            let airplane = s
                .network_data
                .with(|n| n.as_ref().is_some_and(|x| x.airplane_mode));
            s.network_svc
                .send(services::network::NetworkCommand::ToggleAirplaneMode);
            show(OsdKind::Airplane, 0.0, 1.0, !airplane);
        }
        IpcCommand::ToggleIdleInhibitor { .. } => {
            let inhibited = s.idle_inhibitor_data.inhibited.get_untracked();
            s.idle_inhibitor_svc
                .send(services::idle_inhibitor::IdleInhibitorCmd::Toggle);
            show(OsdKind::IdleInhibitor, 0.0, 1.0, !inhibited);
        }
        IpcCommand::ToggleVisibility => unreachable!(),
    }
}

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
        load_font(NERD_FONT.to_vec());
        load_font(NERD_FONT_MONO.to_vec());
        load_font(CUSTOM_FONT.to_vec());

        let cfg = config::load_config(&config_path);
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

            // While the system-info menu is open the sysinfo sampling widens
            // to all domains (disks, network); closed, only the configured
            // bar indicators are refreshed. Synced by an effect further down
            // once the menu state exists.
            let sysinfo_menu_open = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

            let system_info = needed
                .contains(&ModuleName::SystemInfo)
                .then(|| modules::system_info::create(sysinfo_menu_open.clone()));

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
                updates: updates.clone(),
                settings: settings.clone(),
                tray,
                privacy,
                media_player,
                tempo,
                notifications,
                custom,
            };

            // OSD + IPC: commands arrive on the unix socket, execute through
            // the settings services and (optionally) flash the OSD overlay
            let osd = modules::osd::create();
            let ipc_cmd = create_signal(None::<ipc::VersionedCmd>);
            let ipc_writer = ipc_cmd.writer();
            let _ipc = create_service::<(), _, _>(move |_rx, _ctx| async move {
                ipc::serve(ipc_writer).await;
            });
            let bar_visible = create_signal(true);
            {
                let settings = settings.clone();
                let osd = osd.clone();
                let volume_step = cfg.settings.volume_step;
                let max_volume = cfg.settings.max_volume;
                create_effect(move || {
                    let Some(vc) = ipc_cmd.get() else { return };
                    handle_ipc_command(
                        &vc.cmd,
                        settings.as_ref(),
                        &osd,
                        volume_step,
                        max_volume,
                        bar_visible,
                    );
                })
                .detach();
            }

            // Menu state — menus are xdg popups anchored to the bar; the
            // compositor positions and dismisses them (no overlay surface)
            let pending_close = create_signal(false);
            let menu = MenuCtx {
                active_menu: create_signal(None::<MenuType>),
                bar_sid: create_signal(None::<SurfaceId>),
                pending_close_writer: pending_close.writer(),
            };

            // Destroy the menu popup once its collapse animation played
            create_effect(move || {
                if pending_close.get() {
                    modules::finish_menu_close();
                    pending_close.set(false);
                }
            })
            .detach();

            // Sync the sysinfo wide-refresh flag with the open menu
            create_effect({
                let flag = sysinfo_menu_open.clone();
                move || {
                    flag.store(
                        matches!(menu.active_menu.get(), Some(MenuType::SystemInfo)),
                        std::sync::atomic::Ordering::Relaxed,
                    );
                }
            })
            .detach();

            // Bar surfaces: one per configured output (hotplug-aware), or a
            // single compositor-placed bar for Outputs::Active
            let position_anchor = match cfg.position {
                config::Position::Top => Anchor::TOP,
                config::Position::Bottom => Anchor::BOTTOM,
            };
            let _ = app; // surfaces are spawned dynamically below
            let data = std::rc::Rc::new(data);
            let active_menu = menu.active_menu;
            let pending_close_writer = menu.pending_close_writer;
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
                        pending_close_writer,
                    };
                    let mut sc = SurfaceConfig::new()
                        .height(34)
                        .anchor(position_anchor | Anchor::LEFT | Anchor::RIGHT)
                        .layer(Layer::Bottom)
                        .exclusive_zone(Some(34))
                        .background_color(Color::TRANSPARENT)
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
                        bars.retain(|key, handle| {
                            if desired.contains(key) {
                                true
                            } else {
                                bar_ids.borrow_mut().retain(|id| *id != handle.id());
                                handle.close();
                                false
                            }
                        });
                        for key in desired {
                            bars.entry(key).or_insert_with(|| make_bar(key));
                        }
                    })
                    .detach();
                }
            }

            // Hidden bars give up their exclusive zone
            create_effect(move || {
                let visible = bar_visible.get();
                for id in bar_ids.borrow().iter() {
                    surface_handle(*id).set_exclusive_zone(if visible { 34 } else { 0 });
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
