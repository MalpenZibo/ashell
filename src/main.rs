mod config_watcher;
mod layout;
mod modules;
mod services;

use guido::prelude::*;
use services::{CompositorState, start_compositor_service};

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

    let compositor_state = create_signal(CompositorState::default());
    let compositor_svc = start_compositor_service(compositor_state);

    let (app, _) = App::new().add_surface(
        SurfaceConfig::new()
            .height(34)
            .anchor(Anchor::TOP | Anchor::LEFT | Anchor::RIGHT)
            .layer(Layer::Bottom)
            .exclusive_zone(Some(34))
            .background_color(theme::BASE)
            .namespace("ashell"),
        move || {
            container()
                .width(fill())
                .height(fill())
                .layout(layout::CenterBox::new())
                .padding_xy(8.0, 0.0)
                .child(modules::workspaces::view(
                    compositor_state,
                    compositor_svc.clone(),
                ))
                .child(modules::window_title::view(compositor_state))
                .child(
                    container()
                        .layout(
                            Flex::row()
                                .spacing(16.0)
                                .cross_axis_alignment(CrossAxisAlignment::Center),
                        )
                        .child(modules::system_info::view())
                        .child(modules::clock::view()),
                )
        },
    );
    app.run();
}
