mod components;
pub mod config;
mod config_watcher;
mod modules;
mod services;

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

pub fn truncate_text(value: &str, max_length: u32) -> String {
    let length = value.len();

    if length > max_length as usize {
        let split = max_length as usize / 2;
        let first_part = value.chars().take(split).collect::<String>();
        let last_part = value.chars().skip(length - split).collect::<String>();
        format!("{first_part}...{last_part}")
    } else {
        value.to_string()
    }
}

const NERD_FONT: &[u8] = include_bytes!("../target/generated/SymbolsNerdFont-Regular-Subset.ttf");
const NERD_FONT_MONO: &[u8] =
    include_bytes!("../target/generated/SymbolsNerdFontMono-Regular-Subset.ttf");
const CUSTOM_FONT: &[u8] = include_bytes!("../assets/AshellCustomIcon-Regular.otf");

#[tokio::main]
async fn main() {
    env_logger::init();

    // ASHELL_CONFIG_PATH overrides the default ~/.config/ashell/config.toml
    // (used by the benchmark harness and handy for testing)
    let custom_config = std::env::var("ASHELL_CONFIG_PATH").ok();
    let config_path = config_watcher::resolve_config_path(custom_config.as_deref());
    config_watcher::ensure_config_dir(&config_path);

    loop {
        load_font(NERD_FONT.to_vec());
        load_font(NERD_FONT_MONO.to_vec());
        load_font(CUSTOM_FONT.to_vec());

        let cfg = config::load_config(&config_path);
        let theme_colors = theme::init(&cfg.appearance);

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

            let data = ModuleData {
                compositor_state,
                compositor_svc,
                system_info,
                updates: updates.clone(),
                settings: settings.clone(),
                tray,
            };

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

            // Bar surface
            let position_anchor = match cfg.position {
                config::Position::Top => Anchor::TOP,
                config::Position::Bottom => Anchor::BOTTOM,
            };
            let bar_surface_id = app.add_surface(
                SurfaceConfig::new()
                    .height(34)
                    .anchor(position_anchor | Anchor::LEFT | Anchor::RIGHT)
                    .layer(Layer::Bottom)
                    .exclusive_zone(Some(34))
                    .background_color(Color::TRANSPARENT)
                    .keyboard_interactivity(KeyboardInteractivity::None)
                    .namespace("ashell"),
                move || {
                    container()
                        .child(
                            center_box()
                                .left(modules::build_section(&cfg.modules.left, &data, menu))
                                .center(modules::build_section(&cfg.modules.center, &data, menu))
                                .right(modules::build_section(&cfg.modules.right, &data, menu)),
                        )
                        .padding([4, 0])
                },
            );

            // Menus anchor their popups to the bar surface
            menu.bar_sid.set(Some(bar_surface_id));
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
