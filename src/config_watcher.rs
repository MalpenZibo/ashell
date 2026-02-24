use std::path::{Path, PathBuf};
use std::time::Duration;

use futures::StreamExt;
use inotify::{Inotify, WatchMask};
use tokio::task::JoinHandle;

const DEBOUNCE_MS: u64 = 500;

pub fn resolve_config_path(custom: Option<&str>) -> PathBuf {
    if let Some(p) = custom {
        if let Some(rest) = p.strip_prefix("~/")
            && let Ok(home) = std::env::var("HOME")
        {
            return PathBuf::from(home).join(rest);
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
    if let Some(parent) = path.parent()
        && !parent.exists()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        log::warn!("Failed to create config directory {:?}: {}", parent, e);
    }
}

pub fn spawn_config_watcher(path: PathBuf) -> JoinHandle<()> {
    tokio::spawn(config_watch_loop(path))
}

async fn config_watch_loop(path: PathBuf) {
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

    let inotify = match Inotify::init() {
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

    let mut stream = inotify
        .into_event_stream([0u8; 4096])
        .expect("Failed to create inotify event stream");

    while let Some(event_or_err) = stream.next().await {
        match event_or_err {
            Ok(event) => {
                let relevant = event.name.as_ref().is_some_and(|n| *n == *file_name);

                if relevant {
                    log::info!("Config change detected, debouncing...");
                    tokio::time::sleep(Duration::from_millis(DEBOUNCE_MS)).await;
                    log::info!("Config change debounced. Requesting restart...");
                    guido::restart_app();
                    return;
                }
            }
            Err(e) => {
                log::error!("inotify stream error: {}", e);
                return;
            }
        }
    }
}
