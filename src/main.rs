use app::App;
use clap::{Parser, command};
use config::{Config, read_config};
use flexi_logger::{
    Age, Cleanup, Criterion, FileSpec, LogSpecBuilder, LogSpecification, Logger, Naming,
};
use iced::Font;
use log::{error, warn};
use std::panic;
use std::{backtrace::Backtrace, borrow::Cow};

mod app;
mod centerbox;
mod components;
mod config;
mod menu;
mod modules;
mod outputs;
mod password_dialog;
mod position_button;
mod services;
mod style;
mod utils;

const ICON_FONT: &[u8] = include_bytes!("../assets/SymbolsNerdFont-Regular.ttf");
const HEIGHT: u32 = 34;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "~/.config/ashell/config.toml")]
    config_path: String,
}

fn get_log_spec(log_level: &str) -> LogSpecification {
    LogSpecification::env_or_parse(log_level).unwrap_or_else(|err| {
        panic!("Failed to parse log level: {err}");
    })
}

#[tokio::main]
async fn main() -> iced::Result {
    let args = Args::parse();

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
        error!("Panic: {info} \n {b}");
    }));

    println!(" args: {args:?}");

    let config = read_config(&args.config_path).unwrap_or_else(|err| {
        error!("Failed to parse config file: {err}");

        warn!("Using default config");
        Config::default()
    });

    logger.set_new_spec(get_log_spec(&config.log_level));

    let font = match config.appearance.font_name {
        Some(ref font_name) => Font::with_name(Box::leak(font_name.clone().into_boxed_str())),
        None => Font::DEFAULT,
    };

    iced::daemon(App::title, App::update, App::view)
        .subscription(App::subscription)
        .theme(App::theme)
        .style(App::style)
        .font(Cow::from(ICON_FONT))
        .default_font(font)
        .run_with(App::new((logger, config, args.config_path)))
}
