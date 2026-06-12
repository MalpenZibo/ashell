use super::types::{
    ActiveWindow, ActiveWindowHyprland, CompositorCommand, CompositorEvent, CompositorMonitor,
    CompositorState, CompositorWorkspace,
};
use crate::services::{ServiceEvent, compositor::CompositorService};
use anyhow::Result;
use hyprland::{
    data::{Client, Devices, Monitors, Workspace, Workspaces},
    dispatch::{Dispatch, DispatchType, MonitorIdentifier, WorkspaceIdentifierWithSpecial},
    event_listener::AsyncEventListener,
    prelude::*,
};
use itertools::Itertools;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

/// Detect whether Hyprland is using Lua or hyprlang config.
/// Checks `hyprctl status` for the `configProvider` field.
/// Falls back to hyprlang (false) if detection fails.
fn is_lua_config() -> bool {
    std::process::Command::new("hyprctl")
        .arg("status")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.lines().any(|l| l.starts_with("configProvider: lua")))
        .unwrap_or(false)
}

enum DispatchStrategy {
    Socket,
    Lua,
}

fn dispatch(cmd: CompositorCommand, strategy: DispatchStrategy) -> Result<()> {
    let lua_for = |lua: &str| {
        std::process::Command::new("hyprctl")
            .args(["eval", lua])
            .output()?;
        Ok(())
    };
    let socket_call = |dt: DispatchType| -> Result<()> { Ok(Dispatch::call(dt)?) };

    match cmd {
        CompositorCommand::FocusWorkspace(id) => {
            let dt = DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Id(id));
            match strategy {
                DispatchStrategy::Socket => socket_call(dt),
                DispatchStrategy::Lua => lua_for(&format!(
                    "hl.dispatch(hl.dsp.focus({{ workspace = {id} }}))"
                )),
            }
        }
        CompositorCommand::FocusSpecialWorkspace(name) => {
            let dt = DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Special(Some(
                name.as_str(),
            )));
            match strategy {
                DispatchStrategy::Socket => socket_call(dt),
                DispatchStrategy::Lua => lua_for(&format!(
                    "hl.dispatch(hl.dsp.focus({{ workspace = \"special:{name}\" }}))"
                )),
            }
        }
        CompositorCommand::ToggleSpecialWorkspace(name) => {
            let dt = DispatchType::ToggleSpecialWorkspace(Some(name.clone()));
            match strategy {
                DispatchStrategy::Socket => socket_call(dt),
                DispatchStrategy::Lua => lua_for(&format!(
                    "hl.dispatch(hl.dsp.workspace.toggle_special(\"{name}\"))"
                )),
            }
        }
        CompositorCommand::FocusMonitor(id) => {
            let dt = DispatchType::FocusMonitor(MonitorIdentifier::Id(id));
            match strategy {
                DispatchStrategy::Socket => socket_call(dt),
                DispatchStrategy::Lua => {
                    lua_for(&format!("hl.dispatch(hl.dsp.focus({{ monitor = {id} }}))"))
                }
            }
        }
        CompositorCommand::ScrollWorkspace(dir) => {
            let d = if dir > 0 { "+1" } else { "-1" };
            let dt = DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Relative(
                d.to_string().parse()?,
            ));
            match strategy {
                DispatchStrategy::Socket => socket_call(dt),
                DispatchStrategy::Lua => lua_for(&format!(
                    "hl.dispatch(hl.dsp.focus({{ workspace = \"{d}\" }}))"
                )),
            }
        }
        CompositorCommand::NextLayout => {
            hyprland::ctl::switch_xkb_layout::call(
                "all",
                hyprland::ctl::switch_xkb_layout::SwitchXKBLayoutCmdTypes::Next,
            )?;
            Ok(())
        }
        CompositorCommand::CustomDispatch(dispatcher, args) => {
            let dt = DispatchType::Custom(&dispatcher, &args);
            match strategy {
                DispatchStrategy::Socket => socket_call(dt),
                DispatchStrategy::Lua => {
                    lua_for(&format!("hl.dispatch(hl.dsp.{dispatcher}({args}))"))
                }
            }
        }
    }
}

pub async fn execute_command(cmd: CompositorCommand) -> Result<()> {
    let strategy = if is_lua_config() {
        DispatchStrategy::Lua
    } else {
        DispatchStrategy::Socket
    };
    dispatch(cmd, strategy)
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
        let state_guard = internal_state
            .read()
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

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
                        if let Ok(state_guard) = internal_state.read()
                            && let Ok(state) = fetch_full_state(&*state_guard)
                        {
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
                if let Ok(mut state_guard) = internal_state.write() {
                    state_guard.submap = new_submap;
                    if let Ok(state) = fetch_full_state(&state_guard) {
                        let _ = tx.send(ServiceEvent::Update(CompositorEvent::StateChanged(
                            Box::new(state),
                        )));
                    }
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
    let workspaces = Workspaces::get()?
        .into_iter()
        .sorted_by_key(|w| w.id)
        .map(|w| CompositorWorkspace {
            id: w.id,
            index: w.id,
            name: w.name,
            monitor: w.monitor,
            monitor_id: w.monitor_id,
            windows: w.windows,
            is_special: w.id < 0,
            has_urgent: false,
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

    let active_workspace_id = Workspace::get_active().ok().map(|w| w.id);

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
        active_workspace_id,
        active_window,
        keyboard_layout,
        submap: if internal_state.submap.is_empty() {
            None
        } else {
            Some(internal_state.submap.clone())
        },
    })
}
