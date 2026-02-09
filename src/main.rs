mod components;
mod config_watcher;
mod modules;
mod services;

use components::{center_box, module_group};
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

const NERD_FONT: &[u8] =
    include_bytes!("../target/generated/SymbolsNerdFont-Regular-Subset.ttf");

const BAR_HEIGHT: f32 = 34.0;
const MENU_WIDTH: f32 = 300.0;
// TODO: read from compositor monitor info
const SCREEN_WIDTH: f32 = 1920.0;

fn toggle_menu_surface(id: SurfaceId, open: bool) {
    let handle = surface_handle(id);
    if open {
        handle.set_layer(Layer::Overlay);
        handle.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
    } else {
        handle.set_layer(Layer::Background);
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

    let compositor_state = CompositorStateSignals::new(CompositorState::default());
    let compositor_svc = start_compositor_service(compositor_state.writers());

    // Shared system info signals (used by both bar and menu)
    let system_info = modules::system_info::create();

    // Menu state
    let menu_open = create_signal(false);
    let pointer_x = create_signal(0.0f32);
    let menu_x = create_signal(0.0f32);
    // Signal to share menu surface ID between bar and menu surface closures
    let menu_sid = create_signal(None::<SurfaceId>);

    // Build app with bar surface + menu surface
    let (app, _bar_id) = App::new().add_surface(
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
                .on_pointer_move(move |x, _y| pointer_x.set(x))
                .child(
                    center_box()
                        .left(module_group().child(modules::workspaces::view(
                            compositor_state,
                            compositor_svc.clone(),
                        )))
                        .center(module_group().child(modules::window_title::view(compositor_state)))
                        .right(
                            module_group()
                                .child(modules::system_info::view(system_info, move || {
                                    let open = !menu_open.get();
                                    if open {
                                        menu_x.set(pointer_x.get());
                                    }
                                    menu_open.set(open);
                                    if let Some(id) = menu_sid.get() {
                                        toggle_menu_surface(id, open);
                                    }
                                }))
                                .child(modules::clock::view()),
                        ),
                )
                .padding_xy(0., 4.)
        },
    );

    // Menu surface: full-screen overlay, starts hidden on Background layer
    let (app, menu_surface_id) = app.add_surface(
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
                .width(fill())
                .height(fill())
                .background(Color::TRANSPARENT)
                .on_click(move || {
                    menu_open.set(false);
                    if let Some(id) = menu_sid.get() {
                        toggle_menu_surface(id, false);
                    }
                })
                .child(move || {
                    if menu_open.get() {
                        Some(
                            // Menu content panel
                            container()
                                .translate(
                                    move || {
                                        let x = menu_x.get() - MENU_WIDTH / 2.0;
                                        x.max(8.0).min(SCREEN_WIDTH - MENU_WIDTH - 8.0)
                                    },
                                    BAR_HEIGHT,
                                )
                                .width(MENU_WIDTH)
                                .background(theme::SURFACE)
                                .corner_radius(12.0)
                                .padding(16.0)
                                .on_click(|| {
                                    // Swallow clicks so they don't close the menu
                                })
                                .child(modules::system_info::menu_view(system_info)),
                        )
                    } else {
                        None
                    }
                })
        },
    );

    // Store the menu surface ID so on_click handlers can access it
    menu_sid.set(Some(menu_surface_id));

    app.run();
}
