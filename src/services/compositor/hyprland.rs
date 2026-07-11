use super::types::{
    ActiveWindow, ActiveWindowHyprland, CompositorCommand, CompositorEvent, CompositorMonitor,
    CompositorState, CompositorWorkspace,
};
use crate::services::{ServiceEvent, compositor::CompositorService};
use anyhow::Result;
use hyprland::{
    data::{Client, Clients, Devices, Monitors, Workspace, Workspaces},
    dispatch::{Dispatch, DispatchType, MonitorIdentifier, WorkspaceIdentifierWithSpecial},
    event_listener::AsyncEventListener,
    prelude::*,
};
use itertools::Itertools;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

/// Detect whether Hyprland is using Lua or hyprlang config.
/// Checks `hyprctl status` for the `configProvider` field.
/// Falls back to hyprlang (false) if detection fails.
async fn is_lua_config() -> bool {
    tokio::process::Command::new("hyprctl")
        .arg("status")
        .output()
        .await
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.lines().any(|l| l.starts_with("configProvider: lua")))
        .unwrap_or(false)
}

/// Dispatch a command using the old hyprlang socket protocol.
/// Works on all Hyprland versions but is broken on 0.55+ with Lua config.
fn dispatch_hyprlang(cmd: CompositorCommand) -> Result<()> {
    match cmd {
        CompositorCommand::FocusWorkspace(id) => {
            Dispatch::call(DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Id(
                id,
            )))?;
        }
        CompositorCommand::FocusSpecialWorkspace(name) => {
            Dispatch::call(DispatchType::Workspace(
                WorkspaceIdentifierWithSpecial::Special(Some(name.as_str())),
            ))?;
        }
        CompositorCommand::ToggleSpecialWorkspace(name) => {
            Dispatch::call(DispatchType::ToggleSpecialWorkspace(Some(name)))?;
        }
        CompositorCommand::FocusMonitor(id) => {
            Dispatch::call(DispatchType::FocusMonitor(MonitorIdentifier::Id(id)))?;
        }
        CompositorCommand::ScrollWorkspace(dir) => {
            let d = if dir > 0 { "+1" } else { "-1" };
            Dispatch::call(DispatchType::Workspace(
                WorkspaceIdentifierWithSpecial::Relative(d.to_string().parse()?),
            ))?;
        }
        CompositorCommand::NextLayout => {
            hyprland::ctl::switch_xkb_layout::call(
                "main",
                hyprland::ctl::switch_xkb_layout::SwitchXKBLayoutCmdTypes::Next,
            )?;
        }
        CompositorCommand::CustomDispatch(dispatcher, args) => {
            Dispatch::call(DispatchType::Custom(&dispatcher, &args))?;
        }
    }
    Ok(())
}

/// Dispatch a command using the new Lua eval protocol.
/// Required for Hyprland 0.55+ with Lua config.
async fn dispatch_lua(cmd: CompositorCommand) -> Result<()> {
    let lua = match cmd {
        CompositorCommand::FocusWorkspace(id) => {
            format!("hl.dispatch(hl.dsp.focus({{ workspace = {id} }}))")
        }
        CompositorCommand::FocusSpecialWorkspace(name) => {
            format!("hl.dispatch(hl.dsp.focus({{ workspace = \"special:{name}\" }}))")
        }
        CompositorCommand::ToggleSpecialWorkspace(name) => {
            format!("hl.dispatch(hl.dsp.workspace.toggle_special(\"{name}\"))")
        }
        CompositorCommand::FocusMonitor(id) => {
            format!("hl.dispatch(hl.dsp.focus({{ monitor = {id} }}))")
        }
        CompositorCommand::ScrollWorkspace(dir) => {
            let d = if dir > 0 { "+1" } else { "-1" };
            format!("hl.dispatch(hl.dsp.focus({{ workspace = \"{d}\" }}))")
        }
        CompositorCommand::NextLayout => {
            hyprland::ctl::switch_xkb_layout::call(
                "main",
                hyprland::ctl::switch_xkb_layout::SwitchXKBLayoutCmdTypes::Next,
            )?;
            return Ok(());
        }
        CompositorCommand::CustomDispatch(dispatcher, args) => {
            format!("hl.dispatch(hl.dsp.{dispatcher}({args}))")
        }
    };
    tokio::process::Command::new("hyprctl")
        .args(["eval", &lua])
        .output()
        .await?;
    Ok(())
}

pub async fn execute_command(cmd: CompositorCommand) -> Result<()> {
    if is_lua_config().await {
        dispatch_lua(cmd).await
    } else {
        dispatch_hyprlang(cmd)
    }
}

#[derive(Debug, Clone, Default)]
struct HyprInternalState {
    submap: String,
}

pub fn is_available() -> bool {
    const IPC_ENV_VAR: &str = "HYPRLAND_INSTANCE_SIGNATURE";
    std::env::var_os(IPC_ENV_VAR).is_some()
}

pub async fn run_listener(tx: &broadcast::Sender<ServiceEvent<CompositorService>>) -> Result<()> {
    // copying this strategy from how niri's IPC works
    let internal_state = Arc::new(RwLock::new(HyprInternalState::default()));

    // Initial fetch
    {
        let state_guard = internal_state.read().await;

        match fetch_full_state(&state_guard) {
            Ok(state) => {
                let _ = tx.send(ServiceEvent::Update(CompositorEvent::StateChanged(
                    Box::new(state),
                )));
            }
            Err(e) => {
                log::error!("Failed to fetch initial compositor state: {}", e);
            }
        }
    }

    let mut listener = AsyncEventListener::new();

    macro_rules! add_refresh_handler {
        ($method:ident) => {
            listener.$method({
                let tx = tx.clone();
                let internal_state = Arc::clone(&internal_state);
                move |_| {
                    let tx = tx.clone();
                    let internal_state = Arc::clone(&internal_state);
                    Box::pin(async move {
                        let state_guard = internal_state.read().await;
                        if let Ok(state) = fetch_full_state(&*state_guard) {
                            let _ = tx.send(ServiceEvent::Update(CompositorEvent::StateChanged(
                                Box::new(state),
                            )));
                        }
                    })
                }
            });
        };
    }

    add_refresh_handler!(add_workspace_added_handler);
    add_refresh_handler!(add_workspace_changed_handler);
    add_refresh_handler!(add_workspace_deleted_handler);
    add_refresh_handler!(add_workspace_moved_handler);
    add_refresh_handler!(add_changed_special_handler);
    add_refresh_handler!(add_special_removed_handler);
    add_refresh_handler!(add_active_monitor_changed_handler);

    add_refresh_handler!(add_window_closed_handler);
    add_refresh_handler!(add_window_opened_handler);
    add_refresh_handler!(add_window_moved_handler);
    add_refresh_handler!(add_active_window_changed_handler);

    add_refresh_handler!(add_layout_changed_handler);

    // custom refresh handler that takes the changed value as the submap
    listener.add_sub_map_changed_handler({
        let tx = tx.clone();
        move |new_submap| {
            let tx = tx.clone();
            let internal_state = Arc::clone(&internal_state);
            Box::pin(async move {
                let mut state_guard = internal_state.write().await;
                state_guard.submap = new_submap;
                if let Ok(state) = fetch_full_state(&state_guard) {
                    let _ = tx.send(ServiceEvent::Update(CompositorEvent::StateChanged(
                        Box::new(state),
                    )));
                }
            })
        }
    });

    listener
        .start_listener_async()
        .await
        .map_err(|e| anyhow::anyhow!(e))
}

fn fetch_full_state(internal_state: &HyprInternalState) -> Result<CompositorState> {
    let collect_classes = super::should_collect_window_classes();

    let mut workspace_classes: HashMap<i32, Vec<String>> = HashMap::new();
    if collect_classes {
        for client in Clients::get()? {
            workspace_classes
                .entry(client.workspace.id)
                .or_default()
                .push(client.class);
        }
    }

    let workspaces = Workspaces::get()?
        .into_iter()
        .sorted_by_key(|w| w.id)
        .map(|w| {
            let window_classes = if collect_classes {
                workspace_classes.remove(&w.id).unwrap_or_default()
            } else {
                Vec::new()
            };
            CompositorWorkspace {
                id: w.id,
                index: w.id,
                name: w.name,
                monitor: w.monitor,
                monitor_id: w.monitor_id,
                windows: w.windows,
                is_special: w.id < 0,
                has_urgent: false,
                window_classes,
            }
        })
        .collect();

    let monitors = Monitors::get()?
        .into_iter()
        .map(|m| CompositorMonitor {
            id: m.id,
            name: m.name,
            active_workspace_id: m.active_workspace.id,
            special_workspace_id: m.special_workspace.id,
        })
        .collect();

    let active_workspace_ids = Workspace::get_active()
        .ok()
        .map(|w| w.id)
        .into_iter()
        .collect();

    let active_window = Client::get_active().ok().flatten().map(|w| {
        ActiveWindow::Hyprland(ActiveWindowHyprland {
            title: w.title,
            class: w.class,
            address: w.address.to_string(),
            initial_title: w.initial_title,
            initial_class: w.initial_class,
        })
    });

    let keyboard_layout = Devices::get()
        .ok()
        .and_then(|d| {
            d.keyboards
                .into_iter()
                .find(|k| k.main)
                .map(|k| k.active_keymap)
        })
        .unwrap_or_else(|| "Unknown".to_string());

    Ok(CompositorState {
        workspaces,
        monitors,
        active_workspace_ids,
        active_window,
        keyboard_layout,
        submap: if internal_state.submap.is_empty() {
            None
        } else {
            Some(internal_state.submap.clone())
        },
    })
}
