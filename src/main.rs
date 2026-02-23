mod components;
mod config_watcher;
mod modules;
mod services;

use components::{center_box, module_group, module_item};
use guido::prelude::*;
use services::compositor::{CompositorState, CompositorStateSignals, start_compositor_service};

#[allow(dead_code)]
mod theme {
    use guido::prelude::Color;

    // Catppuccin Mocha
    pub const BASE: Color = Color::rgb(30.0 / 255.0, 30.0 / 255.0, 46.0 / 255.0);
    pub const SURFACE: Color = Color::rgb(49.0 / 255.0, 50.0 / 255.0, 68.0 / 255.0);
    pub const TEXT: Color = Color::rgb(205.0 / 255.0, 214.0 / 255.0, 244.0 / 255.0);
    pub const PEACH: Color = Color::rgb(250.0 / 255.0, 179.0 / 255.0, 135.0 / 255.0);
    pub const LAVENDER: Color = Color::rgb(180.0 / 255.0, 190.0 / 255.0, 254.0 / 255.0);
    pub const MAUVE: Color = Color::rgb(203.0 / 255.0, 166.0 / 255.0, 247.0 / 255.0);
    pub const RED: Color = Color::rgb(243.0 / 255.0, 139.0 / 255.0, 168.0 / 255.0);
    pub const YELLOW: Color = Color::rgb(249.0 / 255.0, 226.0 / 255.0, 175.0 / 255.0);
}

const NERD_FONT: &[u8] = include_bytes!("../target/generated/SymbolsNerdFont-Regular-Subset.ttf");

const MENU_WIDTH: f32 = 300.0;

#[derive(Clone, Copy, PartialEq)]
enum MenuType {
    SystemInfo,
    Updates,
    Settings,
}

fn toggle_menu_surface(id: SurfaceId, open: bool) {
    let handle = surface_handle(id);
    if open {
        handle.set_layer(Layer::Overlay);
        handle.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
    } else {
        handle.set_layer(Layer::Overlay);
        handle.set_keyboard_interactivity(KeyboardInteractivity::None);
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    load_font(NERD_FONT.to_vec());

    let config_path = config_watcher::resolve_config_path(None);
    config_watcher::ensure_config_dir(&config_path);
    config_watcher::spawn_config_watcher(config_path);

    App::new().run(|app| {
        let compositor_state = CompositorStateSignals::new(CompositorState::default());
        let compositor_svc = start_compositor_service(compositor_state.writers());

        // Shared module signals
        let system_info = modules::system_info::create();
        let (updates_data, updates_svc) = modules::updates::create();
        let settings = modules::settings::create();

        // Menu state
        let active_menu = create_signal(None::<MenuType>);
        // Signal to share menu surface ID between bar and menu surface closures
        let menu_sid = create_signal(None::<SurfaceId>);
        // Widget refs for positioning the menu under the triggering module
        let sysinfo_ref = create_widget_ref();
        let updates_ref = create_widget_ref();
        let settings_ref = create_widget_ref();
        // Widget ref for the menu backdrop to get screen width for clamping
        let backdrop_ref = create_widget_ref();

        // Bar surface
        let settings_bar = settings.clone();
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
                            .left(module_group().child(module_item().child(
                                modules::workspaces::view(compositor_state, compositor_svc.clone()),
                            )))
                            .center(container().child(move || {
                                compositor_state
                                    .active_window
                                    .with(|w| w.is_some())
                                    .then(|| {
                                        module_group().child(
                                            module_item().child(modules::window_title::view(
                                                compositor_state,
                                            )),
                                        )
                                    })
                            }))
                            .right(
                                module_group()
                                    .child(
                                        container().widget_ref(updates_ref).child(
                                            module_item()
                                                .on_click(move || {
                                                    let new = match active_menu.get() {
                                                        Some(MenuType::Updates) => None,
                                                        _ => Some(MenuType::Updates),
                                                    };
                                                    active_menu.set(new);
                                                    if let Some(id) = menu_sid.get() {
                                                        toggle_menu_surface(id, new.is_some());
                                                    }
                                                })
                                                .child(modules::updates::view(updates_data)),
                                        ),
                                    )
                                    .child({
                                        let settings = settings_bar.clone();
                                        container().widget_ref(settings_ref).child(
                                            module_item()
                                                .on_click(move || {
                                                    let new = match active_menu.get() {
                                                        Some(MenuType::Settings) => None,
                                                        _ => Some(MenuType::Settings),
                                                    };
                                                    active_menu.set(new);
                                                    if let Some(id) = menu_sid.get() {
                                                        toggle_menu_surface(id, new.is_some());
                                                    }
                                                })
                                                .child(modules::settings::view(settings)),
                                        )
                                    })
                                    .child(
                                        container().widget_ref(sysinfo_ref).child(
                                            module_item()
                                                .on_click(move || {
                                                    let new = match active_menu.get() {
                                                        Some(MenuType::SystemInfo) => None,
                                                        _ => Some(MenuType::SystemInfo),
                                                    };
                                                    active_menu.set(new);
                                                    if let Some(id) = menu_sid.get() {
                                                        toggle_menu_surface(id, new.is_some());
                                                    }
                                                })
                                                .child(modules::system_info::view(system_info)),
                                        ),
                                    )
                                    .child(module_item().child(modules::clock::view())),
                            ),
                    )
                    .padding_xy(0., 4.)
            },
        );

        // Menu surface: full-screen overlay, starts hidden on Background layer
        let updates_svc_menu = updates_svc.clone();
        let settings_menu = settings.clone();
        let menu_surface_id = app.add_surface(
            SurfaceConfig::new()
                .anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT)
                .layer(Layer::Background)
                .exclusive_zone(Some(0))
                .background_color(Color::TRANSPARENT)
                .keyboard_interactivity(KeyboardInteractivity::None)
                .namespace("ashell-menu"),
            move || {
                // Backdrop: full-screen, click-outside closes menu
                container()
                    .widget_ref(backdrop_ref)
                    .width(fill())
                    .height(fill())
                    .background(Color::TRANSPARENT)
                    .on_click(move || {
                        active_menu.set(None);
                        if let Some(id) = menu_sid.get() {
                            toggle_menu_surface(id, false);
                        }
                    })
                    .child({
                        let updates_svc_inner = updates_svc_menu.clone();
                        let settings_inner = settings_menu.clone();
                        move || {
                            let menu = active_menu.get();
                            if menu.is_some() {
                                let anchor_ref = match menu {
                                    Some(MenuType::SystemInfo) => sysinfo_ref,
                                    Some(MenuType::Updates) => updates_ref,
                                    Some(MenuType::Settings) => settings_ref,
                                    None => unreachable!(),
                                };
                                let menu_width = match menu {
                                    Some(MenuType::Settings) => 350.0,
                                    _ => MENU_WIDTH,
                                };
                                Some(
                                    // Menu content panel
                                    container()
                                        .translate(
                                            move || {
                                                let r = anchor_ref.rect().get();
                                                let center = r.x + r.width / 2.0;
                                                let screen_w = backdrop_ref.rect().get().width;
                                                (center - menu_width / 2.0)
                                                    .max(8.0)
                                                    .min(screen_w - menu_width - 8.0)
                                            },
                                            0.,
                                        )
                                        .width(menu_width)
                                        .height(at_most(800.0))
                                        .scrollable(ScrollAxis::Vertical)
                                        .background(theme::SURFACE)
                                        .corner_radius(12.0)
                                        .padding(16.0)
                                        .on_click(|| {
                                            // Swallow clicks so they don't close the menu
                                        })
                                        .child({
                                            let svc = updates_svc_inner.clone();
                                            let settings = settings_inner.clone();
                                            move || {
                                                let menu = active_menu.get();
                                                match menu {
                                                    Some(MenuType::SystemInfo) => Some(
                                                        container().child(
                                                            modules::system_info::menu_view(
                                                                system_info,
                                                            ),
                                                        ),
                                                    ),
                                                    Some(MenuType::Updates) => {
                                                        Some(container().child(
                                                            modules::updates::menu_view(
                                                                updates_data,
                                                                svc.clone(),
                                                                move || {
                                                                    active_menu.set(None);
                                                                    if let Some(id) =
                                                                        menu_sid.get()
                                                                    {
                                                                        toggle_menu_surface(
                                                                            id, false,
                                                                        );
                                                                    }
                                                                },
                                                            ),
                                                        ))
                                                    }
                                                    Some(MenuType::Settings) => {
                                                        Some(container().child(
                                                            modules::settings::menu_view(
                                                                settings.clone(),
                                                                move || {
                                                                    active_menu.set(None);
                                                                    if let Some(id) =
                                                                        menu_sid.get()
                                                                    {
                                                                        toggle_menu_surface(
                                                                            id, false,
                                                                        );
                                                                    }
                                                                },
                                                            ),
                                                        ))
                                                    }
                                                    None => None,
                                                }
                                            }
                                        }),
                                )
                            } else {
                                None
                            }
                        }
                    })
            },
        );

        // Store the menu surface ID so on_click handlers can access it
        menu_sid.set(Some(menu_surface_id));
    });
}
