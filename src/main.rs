use std::fs::File;

use app::App;
use flexi_logger::{Age, Cleanup, Criterion, FileSpec, LogSpecification, Logger, Naming};
use iced::{
    wayland::{
        actions::layer_surface::SctkLayerSurfaceSettings,
        layer_surface::{Anchor, KeyboardInteractivity, Layer},
        InitialSurface,
    },
    window::Id,
    Application, Font, Pixels, Settings,
};
use serde::Deserialize;

mod app;
mod centerbox;
mod components;
mod menu;
mod modules;
mod style;
mod utils;

#[derive(Deserialize, Debug)]
struct Config {
    log_level: log::LevelFilter,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_level: log::LevelFilter::Warn,
        }
    }
}

#[tokio::main]
async fn main() {
    let config_file = File::open("~/ashell.yaml");
    let config = if let Ok(config_file) = config_file {
        let config = serde_yaml::from_reader(config_file);
        if let Ok(config) = config {
            config
        }
        Logger::with(LogSpecification::default()).start().unwrap();
        log_panics::init();

        panic!("Failed to parse config file");
    } else {
        Config::default()
    };

    Logger::with(match config.log_level {
        log::LevelFilter::Off => LogSpecification::off(),
        log::LevelFilter::Error => LogSpecification::error(),
        log::LevelFilter::Warn => LogSpecification::warn(),
        log::LevelFilter::Info => LogSpecification::info(),
        log::LevelFilter::Debug => LogSpecification::debug(),
        log::LevelFilter::Trace => LogSpecification::trace(),
    })
    .log_to_stdout()
    .log_to_file(FileSpec::default().directory("/tmp/ashell"))
    .rotate(
        Criterion::Age(Age::Day),
        Naming::Timestamps,
        Cleanup::KeepLogFiles(7),
    )
    .start()
    .unwrap();
    log_panics::init();

    let height = 34;

    App::run(Settings {
        antialiasing: true,
        exit_on_close_request: false,
        initial_surface: InitialSurface::LayerSurface(SctkLayerSurfaceSettings {
            id: Id::MAIN,
            keyboard_interactivity: KeyboardInteractivity::None,
            namespace: "ashell".into(),
            layer: Layer::Top,
            size: None,
            anchor: Anchor::TOP.union(Anchor::LEFT).union(Anchor::RIGHT),
            exclusive_zone: height as i32,
            ..Default::default()
        }),
        flags: (),
        id: None,
        default_font: Font::default(),
        fonts: Default::default(),
        default_text_size: Pixels::from(14.),
    })
    .unwrap();
}
