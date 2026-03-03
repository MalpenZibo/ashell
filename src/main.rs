mod components;
pub mod config;
mod config_watcher;
mod modules;
mod services;

use components::center_box;
use config::ModuleName;
use guido::prelude::*;
use modules::{MenuCtx, MenuType, ModuleData, close_menu_fn, menu_width_for, modules_in_config};
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

const NERD_FONT: &[u8] = include_bytes!("../target/generated/SymbolsNerdFont-Regular-Subset.ttf");

#[tokio::main]
async fn main() {
    env_logger::init();

    let config_path = config_watcher::resolve_config_path(None);
    config_watcher::ensure_config_dir(&config_path);

    loop {
        load_font(NERD_FONT.to_vec());

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
                .then(|| modules::system_info::create());

            let updates = (needed.contains(&ModuleName::Updates) && cfg.updates.is_some())
                .then(|| modules::updates::create());

            let settings = needed
                .contains(&ModuleName::Settings)
                .then(|| modules::settings::create());

            let data = ModuleData {
                compositor_state,
                compositor_svc,
                system_info,
                updates: updates.clone(),
                settings: settings.clone(),
            };

            // Menu state
            let backdrop_ref = create_widget_ref();
            let menu = MenuCtx {
                active_menu: create_signal(None::<MenuType>),
                menu_x: create_signal(0.0_f32),
                menu_sid: create_signal(None::<SurfaceId>),
                backdrop_ref,
            };

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
                        .padding([4.0, 0.0])
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
                    let menu_x = menu.menu_x;
                    let close_outer = close.clone();

                    container()
                        .widget_ref(backdrop_ref)
                        .width(fill())
                        .height(fill())
                        .background(Color::TRANSPARENT)
                        .on_click(close_outer)
                        .child({
                            let updates_inner = updates.clone();
                            let settings_inner = settings.clone();
                            let close_inner = close.clone();
                            move || {
                                let mt = active_menu.get();
                                mt.map(|mt| {
                                    let menu_width = menu_width_for(mt);
                                    let close = close_inner.clone();
                                    container()
                                        .translate(move || menu_x.get(), 0.)
                                        .width(menu_width)
                                        .height(at_most(800.0))
                                        .scrollable(ScrollAxis::Vertical)
                                        .background(theme_colors.background)
                                        .corner_radius(12.0)
                                        .padding(16.0)
                                        .on_click(|| {
                                            // Swallow clicks so they don't close the menu
                                        })
                                        .child({
                                            let updates_inner = updates_inner.clone();
                                            let settings_inner = settings_inner.clone();
                                            let close = close.clone();
                                            move || match active_menu.get() {
                                                Some(MenuType::SystemInfo) => {
                                                    system_info.map(|info| {
                                                        container().child(
                                                            modules::system_info::menu_view(info),
                                                        )
                                                    })
                                                }
                                                Some(MenuType::Updates) => {
                                                    updates_inner.as_ref().map(|(d, svc)| {
                                                        container().child(
                                                            modules::updates::menu_view(
                                                                *d,
                                                                svc.clone(),
                                                                close.clone(),
                                                            ),
                                                        )
                                                    })
                                                }
                                                Some(MenuType::Settings) => {
                                                    settings_inner.as_ref().map(|s| {
                                                        container().child(
                                                            modules::settings::menu_view(
                                                                s.clone(),
                                                                close.clone(),
                                                            ),
                                                        )
                                                    })
                                                }
                                                None => None,
                                            }
                                        })
                                })
                            }
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
