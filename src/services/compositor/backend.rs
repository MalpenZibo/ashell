//! The `Compositor` abstraction: default methods provide the generic-Wayland
//! baseline, and specific backends (Hyprland, Niri) override only the methods
//! they implement specifically. Backends are known and static, so selection is
//! a plain [`Backend`] enum router — no `dyn`, no boxing.

use super::generic::Generic;
use super::hyprland::Hyprland;
use super::niri::Niri;
use super::patch::StatePatch;
use super::types::CompositorCommand;
use super::{generic, hyprland, niri};
use anyhow::{Result, bail};
use iced::futures::future::join4;
use std::future::Future;
use tokio::sync::mpsc;

pub type PatchSink = mpsc::Sender<StatePatch>;

pub trait Compositor: Sync {
    fn name(&self) -> &'static str;

    fn run(&self, sink: PatchSink) -> impl Future<Output = Result<()>> + Send {
        async move {
            // join, not try_join: the sources are independent, so one failing
            // must not cancel the others. Cancelling a spawn_blocking source
            // would also strand its thread rather than stop it.
            let (workspaces, window, keyboard, submap) = join4(
                self.run_workspaces(sink.clone()),
                self.run_window(sink.clone()),
                self.run_keyboard(sink.clone()),
                self.run_submap(sink),
            )
            .await;
            for (source, result) in [
                ("workspaces", workspaces),
                ("window", window),
                ("keyboard", keyboard),
                ("submap", submap),
            ] {
                if let Err(e) = result {
                    log::error!("generic {source} source failed: {e}");
                }
            }
            Ok(())
        }
    }

    fn run_workspaces(&self, sink: PatchSink) -> impl Future<Output = Result<()>> + Send {
        generic::workspaces(sink)
    }
    fn run_window(&self, sink: PatchSink) -> impl Future<Output = Result<()>> + Send {
        generic::window(sink)
    }
    fn run_keyboard(&self, _sink: PatchSink) -> impl Future<Output = Result<()>> + Send {
        async { Ok(()) }
    }
    fn run_submap(&self, _sink: PatchSink) -> impl Future<Output = Result<()>> + Send {
        async { Ok(()) }
    }

    fn focus_workspace(&self, _id: i32) -> impl Future<Output = Result<()>> + Send {
        async { unsupported("focus workspace") }
    }
    fn focus_special_workspace(&self, _name: String) -> impl Future<Output = Result<()>> + Send {
        async { unsupported("focus special workspace") }
    }
    fn toggle_special_workspace(&self, _name: String) -> impl Future<Output = Result<()>> + Send {
        async { unsupported("toggle special workspace") }
    }
    fn focus_monitor(&self, _id: i128) -> impl Future<Output = Result<()>> + Send {
        async { unsupported("focus monitor") }
    }
    fn scroll_workspace(&self, _dir: i32) -> impl Future<Output = Result<()>> + Send {
        async { unsupported("scroll workspace") }
    }
    fn custom_dispatch(
        &self,
        _dispatcher: String,
        _args: String,
    ) -> impl Future<Output = Result<()>> + Send {
        async { unsupported("custom dispatch") }
    }
    fn next_layout(&self) -> impl Future<Output = Result<()>> + Send {
        async { unsupported("switch keyboard layout") }
    }

    fn execute(&self, command: CompositorCommand) -> impl Future<Output = Result<()>> + Send {
        async move {
            match command {
                CompositorCommand::FocusWorkspace(id) => self.focus_workspace(id).await,
                CompositorCommand::FocusSpecialWorkspace(name) => {
                    self.focus_special_workspace(name).await
                }
                CompositorCommand::ToggleSpecialWorkspace(name) => {
                    self.toggle_special_workspace(name).await
                }
                CompositorCommand::FocusMonitor(id) => self.focus_monitor(id).await,
                CompositorCommand::ScrollWorkspace(dir) => self.scroll_workspace(dir).await,
                CompositorCommand::CustomDispatch(dispatcher, args) => {
                    self.custom_dispatch(dispatcher, args).await
                }
                CompositorCommand::NextLayout => self.next_layout().await,
            }
        }
    }
}

fn unsupported(what: &str) -> Result<()> {
    bail!("{what} is not supported on this compositor")
}

pub enum Backend {
    Hyprland,
    Niri,
    Generic,
}

impl Backend {
    pub fn name(&self) -> &'static str {
        match self {
            Backend::Hyprland => Hyprland.name(),
            Backend::Niri => Niri.name(),
            Backend::Generic => Generic.name(),
        }
    }

    pub async fn run(&self, sink: PatchSink) -> Result<()> {
        match self {
            Backend::Hyprland => Hyprland.run(sink).await,
            Backend::Niri => Niri.run(sink).await,
            Backend::Generic => Generic.run(sink).await,
        }
    }

    pub async fn execute(&self, command: CompositorCommand) -> Result<()> {
        match self {
            Backend::Hyprland => Hyprland.execute(command).await,
            Backend::Niri => Niri.execute(command).await,
            Backend::Generic => Generic.execute(command).await,
        }
    }
}

pub fn detect() -> Option<Backend> {
    if hyprland::is_available() {
        Some(Backend::Hyprland)
    } else if niri::is_available() {
        Some(Backend::Niri)
    } else if generic::is_available() {
        log::info!(
            "No native compositor detected; falling back to the generic Wayland backend. \
             If you are on Hyprland or Niri, check that its environment variables are exported."
        );
        Some(Backend::Generic)
    } else {
        None
    }
}
