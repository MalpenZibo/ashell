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

#[tokio::main]
async fn main() {
    env_logger::init();

    let config_path = config_watcher::resolve_config_path(None);
    config_watcher::ensure_config_dir(&config_path);
    config_watcher::spawn_config_watcher(config_path);

    let compositor_state = CompositorStateSignals::new(CompositorState::default());
    let compositor_svc = start_compositor_service(compositor_state.writers());

    let (app, _) = App::new().add_surface(
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
                        .left(module_group().child(modules::workspaces::view(
                            compositor_state,
                            compositor_svc.clone(),
                        )))
                        .center(module_group().child(modules::window_title::view(compositor_state)))
                        .right(
                            module_group()
                                .child(modules::system_info::view())
                                .child(modules::clock::view()),
                        ),
                )
                .padding_xy(0., 4.)
        },
    );
    app.run();
}
