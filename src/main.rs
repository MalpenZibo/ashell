use app::App;
use flexi_logger::{
    Age, Cleanup, Criterion, FileSpec, LogSpecBuilder, LogSpecification, Logger, Naming,
};
use iced::{
    wayland::{
        actions::layer_surface::SctkLayerSurfaceSettings,
        layer_surface::{Anchor, KeyboardInteractivity, Layer},
        InitialSurface,
    },
    window::Id,
    Application, Font, Settings,
};
use log::error;
use serde::Deserialize;
use std::{fs::File, panic, thread};

mod app;
mod centerbox;
mod components;
mod menu;
mod modules;
mod password_dialog;
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

const HEIGHT: u32 = 34;

#[tokio::main]
async fn main() {
    let config_file = File::open("~/ashell.yaml");
    let config = if let Ok(config_file) = config_file {
        let config = serde_yaml::from_reader(config_file);
        if let Ok(config) = config {
            config
        }
        Logger::with(LogSpecification::default()).start().unwrap();
        panic::set_hook(Box::new(|info| {
            error!("Panic: {}", info);
        }));

        panic!("Failed to parse config file");
    } else {
        Config::default()
    };

    Logger::with(
        LogSpecBuilder::new()
            .module(
                "ashell",
                if cfg!(debug_assertions) {
                    log::LevelFilter::Debug
                } else {
                    config.log_level
                },
            )
            .build(),
    )
    .log_to_file(FileSpec::default().directory("/tmp/ashell"))
    .duplicate_to_stdout(flexi_logger::Duplicate::All)
    .rotate(
        Criterion::Age(Age::Day),
        Naming::Timestamps,
        Cleanup::KeepLogFiles(7),
    )
    .start()
    .unwrap();
    panic::set_hook(Box::new(|info| {
        error!("Panic: {}", info);
    }));

    App::run(Settings {
        antialiasing: true,
        exit_on_close_request: false,
        initial_surface: InitialSurface::LayerSurface(SctkLayerSurfaceSettings {
            id: Id::MAIN,
            keyboard_interactivity: KeyboardInteractivity::None,
            namespace: "ashell".into(),
            layer: Layer::Top,
            size: Some((None, Some(HEIGHT))),
            anchor: Anchor::TOP.union(Anchor::LEFT).union(Anchor::RIGHT),
            exclusive_zone: HEIGHT as i32,
            ..Default::default()
        }),
        flags: (),
        id: None,
        fonts: Default::default(),
        default_font: Font::default(),
        default_text_size: 14.into(),
    })
    .unwrap();
}
