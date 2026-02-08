use std::ffi::OsStr;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use inotify::{Inotify, WatchMask};

const DEBOUNCE_MS: u64 = 500;

pub fn resolve_config_path(custom: Option<&str>) -> PathBuf {
    if let Some(p) = custom {
        if let Some(rest) = p.strip_prefix("~/") {
            if let Ok(home) = std::env::var("HOME") {
                return PathBuf::from(home).join(rest);
            }
        }
        return PathBuf::from(p);
    }

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home)
        .join(".config")
        .join("ashell")
        .join("config.toml")
}

pub fn ensure_config_dir(path: &Path) {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                log::warn!("Failed to create config directory {:?}: {}", parent, e);
            }
        }
    }
}

pub fn spawn_config_watcher(path: PathBuf) {
    thread::Builder::new()
        .name("config-watcher".into())
        .spawn(move || {
            config_watch_loop(&path);
        })
        .expect("Failed to spawn config watcher thread");
}

fn config_watch_loop(path: &Path) {
    let parent = match path.parent() {
        Some(p) => p,
        None => {
            log::error!("Config path has no parent directory: {:?}", path);
            return;
        }
    };

    let file_name = match path.file_name() {
        Some(n) => n.to_owned(),
        None => {
            log::error!("Config path has no file name: {:?}", path);
            return;
        }
    };

    let mut inotify = match Inotify::init() {
        Ok(i) => i,
        Err(e) => {
            log::error!("Failed to init inotify: {}", e);
            return;
        }
    };

    let mask = WatchMask::CREATE
        | WatchMask::MODIFY
        | WatchMask::CLOSE_WRITE
        | WatchMask::DELETE
        | WatchMask::MOVED_FROM
        | WatchMask::MOVED_TO;

    if let Err(e) = inotify.watches().add(parent, mask) {
        log::error!("Failed to add inotify watch on {:?}: {}", parent, e);
        return;
    }

    log::info!("Watching {:?} for config changes", parent);

    let mut buf = [0u8; 4096];

    loop {
        let mut events = match inotify.read_events_blocking(&mut buf) {
            Ok(evts) => evts,
            Err(e) => {
                log::error!("inotify read error: {}", e);
                return;
            }
        };

        let relevant =
            events.any(|ev| ev.name.map_or(false, |n| n == file_name.as_os_str()));

        if relevant {
            log::info!("Config change detected, debouncing...");
            debounce_and_exec(&file_name);
        }
    }
}

fn debounce_and_exec(file_name: &OsStr) {
    let deadline = Instant::now() + Duration::from_millis(DEBOUNCE_MS);

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        // Drain any queued events during the debounce window
        thread::sleep(remaining);
        // Try a non-blocking drain — if there are more events, extend the deadline
        // by just consuming them (we already decided to restart)
    }

    log::info!(
        "Config change debounced. Restarting via exec()... (watched: {:?})",
        file_name
    );

    do_exec();
}

fn do_exec() -> ! {
    let args: Vec<String> = std::env::args().collect();
    let exe = &args[0];

    let err = std::process::Command::new(exe).args(&args[1..]).exec();

    // exec() only returns on error
    log::error!("exec() failed: {}", err);
    std::process::exit(1);
}
