use app::App;
use iced::{
    wayland::{
        actions::layer_surface::SctkLayerSurfaceSettings,
        layer_surface::{Anchor, KeyboardInteractivity, Layer},
        InitialSurface,
    },
    window::Id,
    Application, Settings, Font,
};

mod app;
mod modules;
mod centerbox;
mod utils;
mod components;
mod menu;
mod style;

#[tokio::main]
async fn main() {
    let height = 34;

    let menu_sender = menu::create_menu();

    App::run(Settings {
        antialiasing: true,
        exit_on_close_request: false,
        initial_surface: InitialSurface::LayerSurface(SctkLayerSurfaceSettings {
            id: Id(0),
            keyboard_interactivity: KeyboardInteractivity::None,
            namespace: "ashell2".into(),
            layer: Layer::Top,
            size: Some((None, Some(height))),
            anchor: Anchor::TOP.union(Anchor::LEFT).union(Anchor::RIGHT),
            exclusive_zone: height as i32,
            ..Default::default()
        }),
        flags: menu_sender,
        id: None,
        default_font: Font::default(),
        default_text_size: 14.
    })
    .unwrap();
}
