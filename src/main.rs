use app::App;
use config::{read_config, Position};
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
use std::backtrace::Backtrace;
use std::{borrow::Cow, panic};

mod app;
mod centerbox;
mod components;
mod config;
mod menu;
mod modules;
mod password_dialog;
mod services;
mod style;
mod utils;

const ICON_FONT: &[u8] = include_bytes!("../assets/SymbolsNerdFont-Regular.ttf");
const HEIGHT: u32 = 34;

fn get_log_spec(log_level: &str) -> LogSpecification {
    LogSpecification::env_or_parse(log_level).unwrap_or_else(|err| {
        panic!("Failed to parse log level: {}", err);
    })
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
    );
    let logger = if cfg!(debug_assertions) {
        logger.duplicate_to_stdout(flexi_logger::Duplicate::All)
    } else {
        logger
    };
    let logger = logger.start().unwrap();
    panic::set_hook(Box::new(|info| {
        let b = Backtrace::capture();
        error!("Panic: {} \n {}", info, b);
    }));

    let config = read_config().unwrap_or_else(|err| {
        panic!("Failed to parse config file: {}", err);
    });

    logger.set_new_spec(get_log_spec(&config.log_level));

    App::run(Settings {
        antialiasing: true,
        exit_on_close_request: false,
        initial_surface: InitialSurface::LayerSurface(SctkLayerSurfaceSettings {
            id: Id::MAIN,
            keyboard_interactivity: KeyboardInteractivity::None,
            namespace: "ashell".into(),
            layer: Layer::Top,
            size: Some((None, Some(HEIGHT))),
            anchor: match config.position {
                Position::Top => Anchor::TOP,
                Position::Bottom => Anchor::BOTTOM,
            }
            .union(Anchor::LEFT)
            .union(Anchor::RIGHT),
            exclusive_zone: HEIGHT as i32,
            ..Default::default()
        }),
        flags: (logger, config),
        id: None,
        fonts: vec![Cow::from(ICON_FONT)],
        default_font: Font::DEFAULT,
        default_text_size: 14.into(),
    })
    .unwrap();
}
