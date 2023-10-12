use gtk4_layer_shell::Layer;
use gtk4_wrapper::LayerOption;
use leptos::*;

mod bar;
mod components;
mod gtk4_wrapper;
mod modules;
mod utils;

const APP_ID: &str = "ashell";

#[tokio::main]
async fn main() {
    let _ = create_runtime();

    let app = gtk4_wrapper::App::new(APP_ID)
        .set_height(40)
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
