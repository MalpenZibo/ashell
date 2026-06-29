//! The `Compositor` abstraction: default methods provide the generic-Wayland
//! baseline, and specific backends (Hyprland, Niri) override only the methods
//! they implement specifically.

use super::patch::StatePatch;
use super::types::CompositorCommand;
use super::{generic, hyprland, niri};
use anyhow::{Result, bail};
use iced::futures::future::try_join4;
use tokio::sync::mpsc;

pub type PatchSink = mpsc::Sender<StatePatch>;

#[async_trait::async_trait]
pub trait Compositor: Send + Sync {
    fn name(&self) -> &'static str;

    async fn run(&self, sink: PatchSink) -> Result<()> {
        try_join4(
            self.run_workspaces(sink.clone()),
            self.run_window(sink.clone()),
            self.run_keyboard(sink.clone()),
            self.run_submap(sink),
        )
        .await
        .map(|_| ())
    }

    async fn run_workspaces(&self, sink: PatchSink) -> Result<()> {
        generic::workspaces(sink).await
    }
    async fn run_window(&self, sink: PatchSink) -> Result<()> {
        generic::window(sink).await
    }
    async fn run_keyboard(&self, _sink: PatchSink) -> Result<()> {
        Ok(())
    }
    async fn run_submap(&self, _sink: PatchSink) -> Result<()> {
        Ok(())
    }

    async fn focus_workspace(&self, _id: i32) -> Result<()> {
        unsupported("focus workspace")
    }
    async fn focus_special_workspace(&self, _name: String) -> Result<()> {
        unsupported("focus special workspace")
    }
    async fn toggle_special_workspace(&self, _name: String) -> Result<()> {
        unsupported("toggle special workspace")
    }
    async fn focus_monitor(&self, _id: i128) -> Result<()> {
        unsupported("focus monitor")
    }
    async fn scroll_workspace(&self, _dir: i32) -> Result<()> {
        unsupported("scroll workspace")
    }
    async fn custom_dispatch(&self, _dispatcher: String, _args: String) -> Result<()> {
        unsupported("custom dispatch")
    }
    async fn next_layout(&self) -> Result<()> {
        unsupported("switch keyboard layout")
    }

    async fn execute(&self, command: CompositorCommand) -> Result<()> {
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

fn unsupported(what: &str) -> Result<()> {
    bail!("{what} is not supported on this compositor")
}

pub fn detect() -> Option<Box<dyn Compositor>> {
    if hyprland::is_available() {
        Some(Box::new(hyprland::Hyprland))
    } else if niri::is_available() {
        Some(Box::new(niri::Niri))
    } else if generic::is_available() {
        Some(Box::new(generic::Generic))
    } else {
        None
    }
}
