use app::App;
use config::{read_config, Orientation};
use flexi_logger::{
    Age, Cleanup, Criterion, FileSpec, LogSpecBuilder, LogSpecification, Logger, Naming,
};
use iced_sctk::{
    command::platform_specific::wayland::layer_surface::SctkLayerSurfaceSettings,
    commands::layer_surface::{Anchor, KeyboardInteractivity, Layer},
    core::{window::Id, Font},
    multi_window::{settings::Settings, Application},
    settings::InitialSurface,
};
use log::{error, LevelFilter};
use utils::get_anchor;
use std::panic;

mod app;
mod centerbox;
mod components;
mod config;
mod menu;
mod modules;
mod password_dialog;
mod style;
mod utils;

fn get_log_spec(log_level: LevelFilter) -> LogSpecification {
    LogSpecBuilder::new()
        .default(log::LevelFilter::Warn)
        .module(
            "ashell",
            if cfg!(debug_assertions) {
                log::LevelFilter::Debug
            } else {
                log_level
            },
        )
        .build()
}

#[tokio::main]
async fn main() {
    let logger = Logger::with(
        LogSpecBuilder::new()
            .default(log::LevelFilter::Info)
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
    let config = read_config().unwrap_or_else(|err| {
        panic!("Failed to parse config file: {}", err);
    });

    logger.set_new_spec(get_log_spec(config.log_level));

    App::run(Settings {
        antialiasing: true,
        exit_on_close_request: false,
        initial_surface: InitialSurface::LayerSurface(SctkLayerSurfaceSettings {
            id: Id::MAIN,
            keyboard_interactivity: KeyboardInteractivity::None,
            namespace: "ashell".into(),
            layer: Layer::Top,
            size: match config.orientation {
                Orientation::Horizontal => Some((None, Some(config.size))),
                Orientation::Vertical => Some((Some(config.size), None)),
            },
            anchor: get_anchor(config.orientation),
            exclusive_zone: config.size as i32,
            ..Default::default()
        }),
        flags: (logger, config),
        id: None,
        fonts: Default::default(),
        default_font: Font::with_name("DejaVu Sans"),
        default_text_size: 14.into(),
    })
    .unwrap();
}
