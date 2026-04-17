use crate::config::{Position, get_config};
use crate::outputs::Outputs;
use app::App;
use clap::{Parser, Subcommand};
use flexi_logger::{
    Age, Cleanup, Criterion, FileSpec, LogSpecBuilder, LogSpecification, Logger, Naming,
};
use iced::{Anchor, Font, KeyboardInteractivity, Layer, LayerShellSettings};
use log::{debug, error, warn};
use std::backtrace::Backtrace;
use std::panic;
use std::path::PathBuf;

mod app;
mod components;
mod config;
mod ipc;
mod modules;
mod outputs;
mod services;
mod theme;
mod utils;

const NERD_FONT: &[u8] = include_bytes!("../target/generated/SymbolsNerdFont-Regular-Subset.ttf");
const NERD_FONT_MONO: &[u8] =
    include_bytes!("../target/generated/SymbolsNerdFontMono-Regular-Subset.ttf");
const CUSTOM_FONT: &[u8] = include_bytes!("../assets/AshellCustomIcon-Regular.otf");
const HEIGHT: f64 = 34.;

#[derive(Parser, Debug)]
#[command(
    version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_HASH"), ")"),
    about = env!("CARGO_PKG_DESCRIPTION")
)]
struct Args {
    #[arg(short, long, value_parser = clap::value_parser!(PathBuf))]
    config_path: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Send a message to a running ashell instance
    Msg {
        #[command(subcommand)]
        command: IpcCommand,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum IpcCommand {
    /// Toggle bar visibility
    ToggleVisibility,
}

fn get_log_spec(log_level: &str) -> LogSpecification {
    let new_spec = LogSpecification::env_or_parse(log_level);

    match new_spec {
        Ok(spec) => spec,
        Err(err) => {
            warn!("Failed to parse log level: {err}, use the default");

            LogSpecification::default()
        }
    }
}

fn main() -> iced::Result {
    let args = Args::parse();

    if let Some(Command::Msg { command }) = &args.command {
        if let Err(e) = ipc::run_client(command) {
            eprintln!("Error: {e:#}");
            std::process::exit(1);
        }
        std::process::exit(0);
    }

    debug!("args: {args:?}");

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
    let logger = logger.start().unwrap_or_else(|e| {
        eprintln!("Failed to initialize file logger: {e}, falling back to stderr-only");
        Logger::with(
            LogSpecBuilder::new()
                .default(log::LevelFilter::Info)
                .build(),
        )
        .start()
        .expect("critical: cannot initialize any logger")
    });
    panic::set_hook(Box::new(|info| {
        let b = Backtrace::capture();
        error!("Panic: {info} \n {b}");
    }));

    let (config, config_path) = get_config(args.config_path).unwrap_or_else(|err| {
        error!("Failed to read config: {err}");

        std::process::exit(1);
    });

    logger.set_new_spec(get_log_spec(&config.log_level));

    let font = if let Some(font_name) = &config.appearance.font_name {
        Font::with_name(Box::leak(font_name.clone().into_boxed_str()))
    } else {
        Font::DEFAULT
    };

    let height = Outputs::get_height(config.appearance.style, config.appearance.scale_factor);

    let iced_layer = match config.layer {
        config::Layer::Top => Layer::Top,
        config::Layer::Bottom => Layer::Bottom,
        config::Layer::Overlay => Layer::Overlay,
    };

    iced::application(
        App::new((logger, config.clone(), config_path)),
        App::update,
        App::view,
    )
    .layer_shell(LayerShellSettings {
        anchor: match config.position {
            Position::Top => Anchor::TOP,
            Position::Bottom => Anchor::BOTTOM,
        } | Anchor::LEFT
            | Anchor::RIGHT,
        layer: iced_layer,
        exclusive_zone: height as i32,
        size: Some((0, height as u32)),
        keyboard_interactivity: KeyboardInteractivity::None,
        namespace: "ashell-main-layer".into(),
        ..Default::default()
    })
    .subscription(App::subscription)
    .theme(App::theme)
    .scale_factor(App::scale_factor)
    .font(NERD_FONT)
    .font(NERD_FONT_MONO)
    .font(CUSTOM_FONT)
    .default_font(font)
    .run()
}
