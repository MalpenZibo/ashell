mod components;
pub mod config;
mod config_watcher;
mod modules;
mod services;

use components::center_box;
use config::ModuleName;
use guido::prelude::*;
use modules::{
    MENU_WIDTH, MenuCtx, MenuType, ModuleData, close_menu_fn, menu_width_for, modules_in_config,
};
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

    let config_path = config_watcher::resolve_config_path(None);
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

            let system_info = needed
                .contains(&ModuleName::SystemInfo)
                .then(modules::system_info::create);

            let updates = (needed.contains(&ModuleName::Updates) && cfg.updates.is_some())
                .then(modules::updates::create);

            let settings = needed
                .contains(&ModuleName::Settings)
                .then(modules::settings::create);

            let data = ModuleData {
                compositor_state,
                compositor_svc,
                system_info,
                updates: updates.clone(),
                settings: settings.clone(),
            };

            // Menu state
            let backdrop_ref = create_widget_ref();
            let surface_hide_ready = create_signal(false);
            let menu = MenuCtx {
                active_menu: create_signal(None::<MenuType>),
                displayed_menu: create_signal(None::<MenuType>),
                menu_x: create_signal(0.0_f32),
                menu_sid: create_signal(None::<SurfaceId>),
                backdrop_ref,
                surface_hide_writer: surface_hide_ready.writer(),
            };

            // Effect: hide menu surface after close animation completes
            create_effect(move || {
                if surface_hide_ready.get() && menu.active_menu.get().is_none() {
                    if let Some(id) = menu.menu_sid.get() {
                        modules::toggle_menu_surface(id, false);
                    }
                    // Clear displayed_menu so content widgets are destroyed.
                    // This ensures the next open recreates them fresh, avoiding
                    // stale layout state that can cause a brief flash at full height.
                    menu.displayed_menu.set(None);
                    surface_hide_ready.set(false);
                }
            })
            .detach();

            // Bar surface
            app.add_surface(
                SurfaceConfig::new()
                    .height(34)
                    .anchor(Anchor::TOP | Anchor::LEFT | Anchor::RIGHT)
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

            // Menu surface: full-screen overlay, starts hidden on Background layer
            let close = close_menu_fn(menu);
            let menu_surface_id = app.add_surface(
                SurfaceConfig::new()
                    .anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT)
                    .layer(Layer::Background)
                    .exclusive_zone(Some(0))
                    .background_color(Color::TRANSPARENT)
                    .keyboard_interactivity(KeyboardInteractivity::None)
                    .namespace("ashell-menu"),
                move || {
                    let active_menu = menu.active_menu;
                    let displayed_menu = menu.displayed_menu;
                    let menu_x = menu.menu_x;
                    let close_outer = close.clone();

                    container()
                        .widget_ref(backdrop_ref)
                        .width(fill())
                        .height(fill())
                        .background(Color::TRANSPARENT)
                        .on_click(close_outer)
                        // Outer container: position only (translate)
                        .child({
                            let updates_inner = updates.clone();
                            let settings_inner = settings.clone();
                            let close_inner = close.clone();
                            container()
                                .translate(move || menu_x.get(), 0)
                                .width(move || {
                                    displayed_menu
                                        .get()
                                        .map(menu_width_for)
                                        .unwrap_or(MENU_WIDTH)
                                })
                                // Inner container: scaleY animation + content
                                .child(
                                    container()
                                        .width(fill())
                                        .transform(move || {
                                            if active_menu.get().is_some() {
                                                Transform::IDENTITY
                                            } else {
                                                Transform::scale_xy(1.0, 0.0)
                                            }
                                        })
                                        .transform_origin(TransformOrigin::TOP)
                                        .animate_transform(
                                            Transition::spring(SpringConfig::DEFAULT).reverse(
                                                Transition::new(200, TimingFunction::EaseOut),
                                            ),
                                        )
                                        .overflow(Overflow::Hidden)
                                        .background(theme_colors.background)
                                        .corner_radius(12)
                                        .padding(16)
                                        .on_click(|| {})
                                        // Each menu type gets its own child slot to avoid
                                        // key-0 collision in dynamic child reconciliation.
                                        .child(move || {
                                            (displayed_menu.get() == Some(MenuType::SystemInfo))
                                                .then(|| {
                                                    system_info.map(|info| {
                                                        container().child(
                                                            modules::system_info::menu_view(info),
                                                        )
                                                    })
                                                })
                                                .flatten()
                                        })
                                        .child({
                                            let close = close_inner.clone();
                                            move || {
                                                (displayed_menu.get() == Some(MenuType::Updates))
                                                    .then(|| {
                                                        updates_inner.as_ref().map(|(d, svc)| {
                                                            container().child(
                                                                modules::updates::menu_view(
                                                                    *d,
                                                                    svc.clone(),
                                                                    close.clone(),
                                                                ),
                                                            )
                                                        })
                                                    })
                                                    .flatten()
                                            }
                                        })
                                        .child({
                                            let close = close_inner.clone();
                                            move || {
                                                (displayed_menu.get() == Some(MenuType::Settings))
                                                    .then(|| {
                                                        settings_inner.as_ref().map(|s| {
                                                            container().child(
                                                                modules::settings::menu_view(
                                                                    s.clone(),
                                                                    close.clone(),
                                                                ),
                                                            )
                                                        })
                                                    })
                                                    .flatten()
                                            }
                                        }),
                                ) // close inner container
                        })
                },
            );

            // Store the menu surface ID so on_click handlers can access it
            menu.menu_sid.set(Some(menu_surface_id));
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
        }
    }
}
