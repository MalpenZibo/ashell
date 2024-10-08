use app::App;
use config::{read_config, Position};
use flexi_logger::{
    Age, Cleanup, Criterion, FileSpec, LogSpecBuilder, LogSpecification, Logger, Naming,
};
use iced::Font;
use iced_layershell::{
    reexport::{Anchor, KeyboardInteractivity, Layer},
    settings::{LayerShellSettings, Settings, StartMode},
    MultiApplication,
};
use log::{error, LevelFilter};
use std::panic;

mod app;
// mod centerbox;
mod components;
mod config;
mod menu;
mod modules;
// mod password_dialog;
mod services;
mod style;
mod utils;

const HEIGHT: u32 = 34;

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
async fn main() -> Result<(), iced_layershell::Error> {
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
        error!("Panic: {}", info);
    }));

    let config = read_config().unwrap_or_else(|err| {
        panic!("Failed to parse config file: {}", err);
    });

    logger.set_new_spec(get_log_spec(config.log_level));

    App::run(Settings {
        layer_settings: LayerShellSettings {
            size: Some((0, HEIGHT)),
            anchor: match config.position {
                Position::Top => Anchor::Top,
                Position::Bottom => Anchor::Bottom,
            } | Anchor::Left
                | Anchor::Right,
            exclusive_zone: HEIGHT as i32,
            start_mode: StartMode::Active,
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        },
        flags: (logger, config),
        default_font: Font::with_name("DejaVu Sans"),
        default_text_size: 14.into(),
        id: None,
        fonts: Default::default(),
        antialiasing: false,
        virtual_keyboard_support: None,
    })
}

