use crate::config::{Position, get_config};
use crate::outputs::Outputs;
use app::App;
use clap::Parser;
use flexi_logger::{
    Age, Cleanup, Criterion, FileSpec, LogSpecBuilder, LogSpecification, Logger, Naming,
};
use iced::{
    Anchor, Font, KeyboardInteractivity, Layer, LayerShellSettings,
    font::{Style as FontStyle, Weight as FontWeight},
};
use log::{debug, error, warn};
use std::backtrace::Backtrace;
use std::env;
use std::panic;
use std::path::PathBuf;

mod app;
mod components;
mod config;
mod i18n;
mod ipc;
mod modules;
mod osd;
mod outputs;
mod services;
mod theme;
mod utils;
mod xdg;

const NERD_FONT: &[u8] = include_bytes!("../target/generated/SymbolsNerdFont-Regular-Subset.ttf");
const NERD_FONT_MONO: &[u8] =
    include_bytes!("../target/generated/SymbolsNerdFontMono-Regular-Subset.ttf");
const CUSTOM_FONT: &[u8] = include_bytes!("../assets/AshellCustomIcon-Regular.otf");
const HEIGHT: f64 = 34.;
const TMP_FILE_SIZE: u64 = 10 * 1024 * 1024;

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

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Send a message to a running ashell instance
    Msg {
        #[command(subcommand)]
        command: ipc::IpcCommand,
    },
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

fn fontdb_weight_to_iced(weight: fontdb::Weight) -> FontWeight {
    // Map fontdb::Weight to iced::font::Weight.
    // fontdb uses numeric values (100..=1000); iced uses named variants.
    let numeric_weight = weight.0;
    if numeric_weight <= 150 {
        FontWeight::Thin
    } else if numeric_weight <= 250 {
        FontWeight::ExtraLight
    } else if numeric_weight <= 350 {
        FontWeight::Light
    } else if numeric_weight <= 450 {
        FontWeight::Normal
    } else if numeric_weight <= 550 {
        FontWeight::Medium
    } else if numeric_weight <= 650 {
        FontWeight::Semibold
    } else if numeric_weight <= 750 {
        FontWeight::Bold
    } else if numeric_weight <= 850 {
        FontWeight::ExtraBold
    } else {
        FontWeight::Black
    }
}

fn resolve_font(name: &str) -> Font {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    // Find the best matching face for the requested Normal/Normal/Normal.
    // cosmic-text's font matcher re-scores all faces and prefers a face whose
    // weight/stretch/style are closest to the requested ones. So if the user
    // asks for `Weight::Normal` (400) but the font only has a face with
    // weight=500 (Medium), the matcher will prefer a different font with
    // weight=400 — a silent fallback. To work around this, we detect the
    // closest available face in the requested family and pass its actual
    // weight to iced so the matcher's scoring keeps us on the right font.
    let best_face = db
        .faces()
        .filter(|f| f.families.iter().any(|(fam, _)| fam == name))
        .min_by_key(|f| {
            // Score: prefer Normal style/stretch, then closest weight to 400.
            let style_penalty = if f.style == fontdb::Style::Normal {
                0
            } else {
                100
            };
            let stretch_penalty = if f.stretch == fontdb::Stretch::Normal {
                0
            } else {
                100
            };
            let weight_penalty = f.weight.0.abs_diff(400) / 10;
            style_penalty + stretch_penalty + weight_penalty
        });

    if best_face.is_none() {
        warn!("Font '{}' was not found in the system font database.", name);
        warn!("  Use `fc-list` to list all available fonts and their exact family names.");
        return Font::with_name(Box::leak(name.to_string().into_boxed_str()));
    }

    let face = best_face.unwrap();
    let weight = face.weight;
    let iced_weight = fontdb_weight_to_iced(weight);

    if weight != fontdb::Weight::NORMAL {
        warn!(
            "Font '{name}' has no face with weight=Normal(400) style=Normal. \
             Using the closest available face (weight={}, style={:?}). \
             Note: text rendered in a different weight (e.g. Bold) may look \
             the same as regular text, since this font has no separate faces \
             for those weights. Run `fc-list | grep -i {short_name}` to see \
             all available faces of this font.",
            weight.0,
            face.style,
            short_name = name.split_whitespace().next().unwrap_or(name),
        );
    }

    Font {
        family: iced::font::Family::Name(Box::leak(name.to_string().into_boxed_str())),
        weight: iced_weight,
        stretch: iced::font::Stretch::Normal,
        style: if face.style == fontdb::Style::Italic {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        },
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

    let logdir = xdg::get_runtime_dir()
        .unwrap_or_else(|| [env::temp_dir(), PathBuf::from("ashell")].iter().collect());
    let logger = Logger::with(
        LogSpecBuilder::new()
            .default(log::LevelFilter::Info)
            .build(),
    )
    .log_to_file(FileSpec::default().directory(logdir))
    .rotate(
        Criterion::AgeOrSize(Age::Day, TMP_FILE_SIZE),
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
        resolve_font(font_name)
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
