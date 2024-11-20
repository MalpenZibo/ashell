use app::App;
use config::{read_config, Position};
use flexi_logger::{
    Age, Cleanup, Criterion, FileSpec, LogSpecBuilder, LogSpecification, Logger, Naming,
};
use iced::Font;
use iced_layershell::{
    reexport::{Anchor, KeyboardInteractivity},
    settings::{LayerShellSettings, Settings, StartMode},
    MultiApplication,
};
use log::error;
use std::{backtrace::Backtrace, borrow::Cow, panic};

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
        let b = Backtrace::capture();
        error!("Panic: {} \n {}", info, b);
    }));

    let config = read_config().unwrap_or_else(|err| {
        panic!("Failed to parse config file: {}", err);
    });

    logger.set_new_spec(get_log_spec(&config.log_level));

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
        fonts: vec![Cow::from(ICON_FONT)],
        default_font: Font::DEFAULT,
        default_text_size: 14.into(),
        id: None,
        antialiasing: false,
        virtual_keyboard_support: None,
    })
}
