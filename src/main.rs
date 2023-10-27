use app::{App, LayerOption};
use gtk4_layer_shell::Layer;

mod app;
mod bar;
mod modules;
mod reactive_gtk;
mod utils;

#[tokio::main]
async fn main() {
    let app = App::new("ashell")
        .set_height(34)
        .set_layer_option(LayerOption {
            r#type: Layer::Top,
            exclusive_zone: true,
            top_anchor: true,
            left_anchor: true,
            right_anchor: true,
            bottom_anchor: false,
        });

    app.run(bar::bar);
}
