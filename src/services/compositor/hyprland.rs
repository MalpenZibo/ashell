use super::types::{
    ActiveWindow, ActiveWindowHyprland, CompositorCommand, CompositorEvent, CompositorMonitor,
    CompositorState, CompositorWorkspace,
};
use crate::services::{ServiceEvent, compositor::CompositorService};
use anyhow::{Result, anyhow};
use hyprland::{
    data::{Client, Clients, Devices, Monitors, Workspace, Workspaces},
    dispatch::{Dispatch, DispatchType, WorkspaceIdentifierWithSpecial},
    event_listener::AsyncEventListener,
    prelude::*,
};
use itertools::Itertools;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::{RwLock, broadcast};

/// Cached result of a successful [`detect_lua_config`] probe. Hyprland picks
/// its config provider once at startup, so the answer stays valid for the
/// lifetime of the process. Failed probes are not cached, so the next dispatch
/// tries again instead of pinning the wrong protocol.
static LUA_CONFIG: OnceLock<bool> = OnceLock::new();

/// Detect whether Hyprland is using Lua or hyprlang config by checking
/// `hyprctl status` for the `configProvider` field. Returns `None` when the
/// probe itself failed, so a failed run is not mistaken for hyprlang.
async fn detect_lua_config() -> Option<bool> {
    let output = tokio::process::Command::new("hyprctl")
        .arg("status")
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let status = String::from_utf8(output.stdout).ok()?;
    let Some(provider) = status
        .lines()
        .find_map(|line| line.strip_prefix("configProvider:"))
    else {
        // `hyprctl status` only exists since Hyprland 0.55. Older versions
        // answer "unknown request" but still exit 0, so a successful call
        // without the field means hyprlang and is safe to cache.
        return Some(false);
    };

    Some(provider.trim() == "lua")
}

/// Falls back to hyprlang (false) when detection fails.
async fn is_lua_config() -> bool {
    if let Some(cached) = LUA_CONFIG.get() {
        return *cached;
    }

    let Some(detected) = detect_lua_config().await else {
        return false;
    };

    *LUA_CONFIG.get_or_init(|| detected)
}

/// Dispatch a command using the old hyprlang socket protocol.
/// Works on all Hyprland versions but is broken on 0.55+ with Lua config.
async fn dispatch_hyprlang(cmd: CompositorCommand) -> Result<()> {
    match cmd {
        CompositorCommand::FocusWorkspace(id) => {
            Dispatch::call(DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Id(
                id,
            )))?;
        }
        CompositorCommand::FocusNamedWorkspace(name) => {
            Dispatch::call(DispatchType::Workspace(
                WorkspaceIdentifierWithSpecial::Name(name.as_str()),
            ))?;
        }
        CompositorCommand::ToggleSpecialWorkspace(name) => {
            Dispatch::call(DispatchType::ToggleSpecialWorkspace(Some(name)))?;
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
        CompositorCommand::FocusAppWindow { pid, apps, title } => {
            let selector = app_window_selector(pid, &apps, title.as_deref()).await?;
            Dispatch::call(DispatchType::Custom("focuswindow", &selector))?;
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
        CompositorCommand::FocusNamedWorkspace(name) => {
            let name = escape_lua_string(&name);
            format!("hl.dispatch(hl.dsp.focus({{ workspace = \"name:{name}\" }}))")
        }
        CompositorCommand::ToggleSpecialWorkspace(name) => {
            let name = escape_lua_string(&name);
            format!("hl.dispatch(hl.dsp.workspace.toggle_special(\"{name}\"))")
        }
        CompositorCommand::NextLayout => {
            hyprland::ctl::switch_xkb_layout::call(
                "main",
                hyprland::ctl::switch_xkb_layout::SwitchXKBLayoutCmdTypes::Next,
            )?;
            return Ok(());
        }
        CompositorCommand::CustomDispatch(dispatcher, args) => {
            // `dispatcher` is a Lua identifier and `args` is spliced in as a raw
            // Lua expression (e.g. `{ workspace = "3" }`), not a string literal —
            // escaping either would break valid input. The contract here is that
            // the caller hands us Lua.
            format!("hl.dispatch(hl.dsp.{dispatcher}({args}))")
        }
        CompositorCommand::FocusAppWindow { pid, apps, title } => {
            format!(
                "hl.dispatch(hl.dsp.focus({{ window = \"{}\" }}))",
                app_window_selector(pid, &apps, title.as_deref()).await?
            )
        }
    };
    tokio::process::Command::new("hyprctl")
        .args(["eval", &lua])
        .output()
        .await?;
    Ok(())
}

async fn app_window_selector(
    pid: Option<u32>,
    apps: &[String],
    title: Option<&str>,
) -> Result<String> {
    let clients = Clients::get_async().await?;

    // Sandboxed apps can report another pid, so fall back to the window class
    // when no window has the player's pid.
    let by_pid: Vec<_> = clients
        .iter()
        .filter(|c| pid.is_some_and(|pid| u32::try_from(c.pid).is_ok_and(|p| p == pid)))
        .collect();
    let candidates = if by_pid.is_empty() {
        clients
            .iter()
            .filter(|c| {
                super::app_matches(Some(&c.class), apps)
                    || super::app_matches(Some(&c.initial_class), apps)
            })
            .collect()
    } else {
        by_pid
    };

    // Prefer the window whose title shows the media, then the most recently
    // focused (0 is the current one); a negative id, if any, means never
    // focused and sorts last.
    candidates
        .into_iter()
        .max_by_key(|c| {
            (
                super::title_matches(Some(&c.title), title),
                (c.focus_history_id >= 0).then(|| -i16::from(c.focus_history_id)),
            )
        })
        .map(|c| format!("address:{}", c.address))
        .ok_or_else(|| anyhow!("No window found for pid {pid:?} or apps {apps:?}"))
}

pub async fn execute_command(cmd: CompositorCommand) -> Result<()> {
    if is_lua_config().await {
        dispatch_lua(cmd).await
    } else {
        dispatch_hyprlang(cmd).await
    }
}

// Names reach the Lua layer by string interpolation, so quotes, backslashes
// and control characters must be escaped or a workspace name containing them
// breaks the generated program.
// TODO: switch to `hyprland::lua::{quote_string, escape_string}` once
// hyprland-rs#399 lands, since it also covers the remaining control
// characters as zero-padded \ddd sequences.
fn escape_lua_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
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

    // 0.57 special/named workspaces have no id, so key by name.
    let mut workspace_classes: HashMap<String, Vec<String>> = HashMap::new();
    if collect_classes {
        for client in Clients::get()? {
            workspace_classes
                .entry(client.workspace.name)
                .or_default()
                .push(client.class);
        }
    }

    let workspaces = Workspaces::get()?
        .into_iter()
        .sorted_by_key(|w| w.id().unwrap_or(i32::MAX))
        .map(|w| {
            // Zero is not a real workspace id: identify by name instead.
            let id = w.id().unwrap_or(0);
            let is_special = w.is_special();
            let window_classes = if collect_classes {
                workspace_classes.remove(&w.name).unwrap_or_default()
            } else {
                Vec::new()
            };
            CompositorWorkspace {
                id,
                index: id,
                name: w.name,
                monitor: w.monitor,
                monitor_id: w.monitor_id,
                windows: w.windows,
                is_special,
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
            focused: m.focused,
            active_workspace_id: m.active_workspace.id().unwrap_or(-1),
            active_workspace_name: m.active_workspace.name,
            special_workspace_name: m.special_workspace.name,
        })
        .collect();

    let active_workspace_ids = Workspace::get_active()
        .ok()
        .and_then(|w| w.id().ok())
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

#[cfg(test)]
mod tests {
    use super::escape_lua_string;

    #[test]
    fn quotes_and_backslashes_are_escaped() {
        assert_eq!(escape_lua_string(r#"a\b"c"#), r#"a\\b\"c"#);
    }

    #[test]
    fn control_characters_are_escaped() {
        assert_eq!(escape_lua_string("a\nb\rc\td"), r"a\nb\rc\td");
    }

    #[test]
    fn plain_names_are_untouched() {
        assert_eq!(escape_lua_string("special:2"), "special:2");
    }
}
