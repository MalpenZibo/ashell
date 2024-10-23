use app::App;
use config::read_config;
use flexi_logger::{
    Age, Cleanup, Criterion, FileSpec, LogSpecBuilder, LogSpecification, Logger, Naming,
};
use log::{error, LevelFilter};
use std::panic;

mod app;
mod centerbox;
mod components;
mod config;
mod menu;
mod modules;
mod outputs;
mod password_dialog;
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
async fn main() -> iced::Result {
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

    iced::daemon(App::title, App::update, App::view)
        .subscription(App::subscription)
        .theme(App::theme)
        .style(App::style)
        .run_with(App::new((logger, config)))
}
