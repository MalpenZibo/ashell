use super::types::{
    ActiveWindow, ActiveWindowNiri, CompositorCommand, CompositorEvent, CompositorMonitor,
    CompositorService, CompositorState, CompositorWindow, CompositorWorkspace,
};
use crate::services::ServiceEvent;
use anyhow::{Context, Result, anyhow};
use itertools::Itertools;
use niri_ipc::{
    Action, Event, Reply, Request, Response, WorkspaceReferenceArg,
    state::{EventStreamState, EventStreamStatePart},
};
use std::{collections::HashMap, env, os::unix::net::UnixStream as StdUnixStream};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    sync::broadcast,
};

pub async fn execute_command(cmd: CompositorCommand) -> Result<()> {
    let mut stream = connect().await?;

    let action = match cmd {
        CompositorCommand::FocusWorkspace(id) => match u64::try_from(id) {
            Ok(id) => Action::FocusWorkspace {
                reference: WorkspaceReferenceArg::Id(id),
            },
            Err(_) => {
                return Err(anyhow!(
                    "Workspace ID {} is out of range for Niri backend",
                    id
                ));
            }
        },
        CompositorCommand::FocusNamedWorkspace(_)
        | CompositorCommand::ToggleSpecialWorkspace(_) => {
            return Err(anyhow!("Special workspaces not supported in Niri backend"));
        }
        CompositorCommand::NextLayout => Action::SwitchLayout {
            layout: niri_ipc::LayoutSwitchTarget::Next,
        },
        CompositorCommand::CustomDispatch(action, args) => {
            if action == "spawn" {
                Action::SpawnSh { command: args }
            } else {
                return Err(anyhow!("Unknown custom dispatch: {}", action));
            }
        }

        CompositorCommand::FocusAppWindow { pid, apps, title } => Action::FocusWindow {
            id: app_window_id(pid, &apps, title.as_deref()).await?,
        },
    };

    send_command_request(&mut stream, Request::Action(action)).await?;
    Ok(())
}

pub fn is_available() -> bool {
    env::var_os("NIRI_SOCKET")
        .or_else(|| env::var_os("NIRI_SOCKET_PATH"))
        .is_some()
}

pub async fn run_listener(tx: &broadcast::Sender<ServiceEvent<CompositorService>>) -> Result<()> {
    // 1. Init
    let mut stream = connect().await?;

    // 2. Send the EventStream request
    let request_json = serde_json::to_string(&Request::EventStream)? + "\n";
    stream.write_all(request_json.as_bytes()).await?;
    stream.flush().await?;

    // 3. Create ONE BufReader for the lifetime of this connection
    let mut reader = BufReader::new(stream);

    // 4. Read the Handled response
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let reply: Reply = serde_json::from_str(&line).context("Failed to parse handshake")?;
    if let Err(e) = reply {
        return Err(anyhow!("Niri refused EventStream: {}", e));
    }

    // 5. Shutdown write half
    let _ = reader.get_mut().shutdown().await;

    let mut internal_state = EventStreamState::default();

    // Outputs aren't part of the event stream, so fetch them up front and
    // refresh them on the events that follow an output change.
    let mut outputs = fetch_outputs().await.unwrap_or_default();

    // 6. Loop forever using the SAME reader
    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            break; // EOF
        }

        let event: Event = match serde_json::from_str(&line) {
            Ok(ev) => ev,
            Err(e) => {
                // This can happen a lot if the installed niri version and the IPC are out of sync
                // From niri's wiki:
                // The JSON output should remain stable, as in:
                // - existing fields and enum variants should not be renamed
                // - non-optional existing fields should not be removed
                // However, new fields and enum variants will be added, so you should handle unknown fields or variants gracefully where reasonable.
                log::debug!(
                    "Failed to parse Niri event (this is caused by niri's IPC not being version bound) -> {:?}",
                    e
                );
                continue;
            }
        };

        // Checked before applying, against the tile sizes known so far.
        let need_outputs_refresh = outputs_may_have_changed(&event, &internal_state);

        // Apply to internal Niri state tracker
        internal_state.apply(event);

        if need_outputs_refresh && let Ok(fresh) = fetch_outputs().await {
            outputs = fresh;
        }

        // Map to generic Ashell state
        let state = map_state(&internal_state, &outputs);

        // Emit Update
        let _ = tx.send(ServiceEvent::Update(CompositorEvent::StateChanged(
            Box::new(state),
        )));
    }

    Ok(())
}

/// Whether `event` may follow a change to an output's logical size, given the
/// state before it's applied. niri has no output event, so this goes by what
/// an output change brings along:
/// - hotplug moves workspaces between outputs (`WorkspacesChanged`), also
///   when a different monitor comes back on the same connector;
/// - a mode, scale or transform change in the config reloads it
///   (`ConfigLoaded`);
/// - a change at runtime (`niri msg output`) resizes the tiles on that output
///   (`WindowLayoutsChanged` with a new tile size). Scrolling only moves tiles,
///   so it doesn't count.
fn outputs_may_have_changed(event: &Event, state: &EventStreamState) -> bool {
    match event {
        Event::WorkspacesChanged { .. } | Event::ConfigLoaded { .. } => true,
        Event::WindowLayoutsChanged { changes } => changes.iter().any(|(id, layout)| {
            state
                .windows
                .windows
                .get(id)
                .is_some_and(|w| w.layout.tile_size != layout.tile_size)
        }),
        _ => false,
    }
}

/// Logical size of every output with a mode, by output name.
async fn fetch_outputs() -> Result<HashMap<String, (u32, u32)>> {
    let mut stream = connect().await?;

    let request_json = serde_json::to_string(&Request::Outputs)? + "\n";
    stream.write_all(request_json.as_bytes()).await?;
    stream.flush().await?;

    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line).await?;

    let reply: Reply = serde_json::from_str(&response_line).context("parse outputs reply")?;
    let response = reply.map_err(|e| anyhow!("Niri error fetching outputs: {}", e))?;
    match response {
        Response::Outputs(outputs) => Ok(outputs
            .into_iter()
            .filter_map(|(name, out)| out.logical.map(|l| (name, (l.width, l.height))))
            .collect()),
        other => Err(anyhow!(
            "Unexpected response to Outputs request: {:?}",
            other
        )),
    }
}

async fn connect() -> Result<UnixStream> {
    let socket_path = env::var_os("NIRI_SOCKET")
        .or_else(|| env::var_os("NIRI_SOCKET_PATH"))
        .ok_or_else(|| anyhow!("NIRI_SOCKET or NIRI_SOCKET_PATH environment variable not set"))?;

    let std_stream = StdUnixStream::connect(socket_path)?;
    std_stream.set_nonblocking(true)?;
    UnixStream::from_std(std_stream).context("Failed to convert stream")
}

async fn send_command_request(stream: &mut UnixStream, request: Request) -> Result<()> {
    send_request(stream, request).await.map(|_| ())
}

async fn send_request(stream: &mut UnixStream, request: Request) -> Result<Response> {
    let mut json = serde_json::to_string(&request)?;
    json.push('\n');
    stream.write_all(json.as_bytes()).await?;
    stream.flush().await?;

    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line).await?;

    let reply: Reply = serde_json::from_str(&response_line)?;
    reply.map_err(|e| anyhow!("Niri error: {}", e))
}

async fn app_window_id(pid: Option<u32>, apps: &[String], title: Option<&str>) -> Result<u64> {
    let mut stream = connect().await?;

    let Response::Windows(windows) = send_request(&mut stream, Request::Windows).await? else {
        return Err(anyhow!("Unexpected reply to a Niri windows request"));
    };

    // XWayland windows report xwayland-satellite's pid, so fall back to the app
    // id when no window has the player's pid.
    let by_pid: Vec<_> = windows
        .iter()
        .filter(|w| {
            w.pid
                .zip(pid)
                .is_some_and(|(w, p)| i64::from(w) == i64::from(p))
        })
        .collect();
    let candidates = if by_pid.is_empty() {
        windows
            .iter()
            .filter(|w| super::app_matches(w.app_id.as_deref(), apps))
            .collect()
    } else {
        by_pid
    };

    // Prefer the window whose title shows the media, then the focused one (none
    // usually is while ashell's menu is open), then the most recently focused;
    // windows never focused sort last.
    candidates
        .into_iter()
        .max_by_key(|w| {
            (
                super::title_matches(w.title.as_deref(), title),
                w.is_focused,
                w.focus_timestamp.map(|t| (t.secs, t.nanos)),
            )
        })
        .map(|w| w.id)
        .ok_or_else(|| anyhow!("No window found for pid {pid:?} or apps {apps:?}"))
}

/// `_outputs` holds each output's logical size by name, for consumers to come.
fn map_state(niri: &EventStreamState, _outputs: &HashMap<String, (u32, u32)>) -> CompositorState {
    let output_to_active_ws: HashMap<_, _> = niri
        .workspaces
        .workspaces
        .values()
        .filter_map(|ws| {
            if let Some(out) = &ws.output
                && ws.is_active
            {
                Some((out.clone(), ws.id as i32))
            } else {
                None
            }
        })
        .collect();

    // INFO: this is how niri sorts the outpus internally (niri msg outputs - in client.rs)
    let outputs_sorted = output_to_active_ws
        .keys()
        .sorted_unstable()
        .collect::<Vec<_>>();

    let collect_classes = super::should_collect_window_classes();

    // Collect window classes per workspace (by visual index)
    let mut workspace_classes: HashMap<i32, Vec<String>> = HashMap::new();
    if collect_classes {
        for win in niri.windows.windows.values() {
            if let Some(ws_id) = win.workspace_id
                && let Some(ws) = niri.workspaces.workspaces.get(&ws_id)
                && let Some(app_id) = &win.app_id
            {
                workspace_classes
                    .entry(ws.idx as i32)
                    .or_default()
                    .push(app_id.clone());
            }
        }
    }

    let mut workspaces: Vec<CompositorWorkspace> = niri
        .workspaces
        .workspaces
        .values()
        .sorted_by_key(|w| w.idx)
        .map(|w| {
            let idx = w.idx as i32;
            CompositorWorkspace {
                id: w.id as i32,
                index: idx,
                name: w.name.clone().unwrap_or_else(|| w.idx.to_string()),
                monitor: w.output.clone().unwrap_or_default(),
                // niri does not have an output index
                monitor_id: w.output.as_ref().map(|wo| {
                    outputs_sorted
                        .iter()
                        .position(|o| *o == wo)
                        .map_or(-1, |i| i as i32) as i128
                }),
                windows: 0,
                is_special: false,
                has_urgent: w.is_urgent,
                window_classes: if collect_classes {
                    workspace_classes.remove(&idx).unwrap_or_default()
                } else {
                    Vec::new()
                },
            }
        })
        .collect();

    // Calculate window counts; also propagate per-window urgency to workspaces
    // since older niri versions may only emit window-level urgency events.
    for win in niri.windows.windows.values() {
        if let Some(ws_id) = win.workspace_id {
            // Resolve Niri Workspace ID (u64) -> Visual Index (u8) -> Generic ID (i32)
            if let Some(ws) = niri.workspaces.workspaces.get(&ws_id)
                && let Some(generic_ws) = workspaces.iter_mut().find(|w| w.id == ws.id as i32)
            {
                generic_ws.windows += 1;
                if win.is_urgent {
                    generic_ws.has_urgent = true;
                }
            }
        }
    }

    let monitors: Vec<CompositorMonitor> = outputs_sorted
        .iter()
        .enumerate()
        .map(|(i, name)| CompositorMonitor {
            id: i as i128,
            name: (*name).clone(),
            focused: false,
            active_workspace_id: output_to_active_ws.get(*name).copied().unwrap_or(-1),
            active_workspace_name: String::new(),
            special_workspace_name: String::new(),
        })
        .collect();

    let focused_workspace_id = niri
        .workspaces
        .workspaces
        .values()
        .find(|w| w.is_focused)
        .map(|w| w.id as i32);
    let active_workspace_ids = focused_workspace_id.into_iter().collect();

    let active_window = niri
        .windows
        .windows
        .values()
        .find(|w| w.is_focused)
        .map(|w| {
            ActiveWindow::Niri(ActiveWindowNiri {
                title: w.title.clone().unwrap_or_default(),
                class: w.app_id.clone().unwrap_or_default(),
                address: w.id.to_string(),
            })
        });

    let windows: Vec<CompositorWindow> = niri
        .windows
        .windows
        .values()
        .map(|w| CompositorWindow {
            id: w.id,
            workspace_id: w.workspace_id.map(|wid| wid as i32),
            is_focused: w.is_focused,
            is_floating: w.is_floating,
            is_urgent: w.is_urgent,
            tile_position: w
                .layout
                .pos_in_scrolling_layout
                .map(|(c, r)| (c as u32, r as u32)),
            tile_width: w.layout.tile_size.0 as f32,
            tile_height: w.layout.tile_size.1 as f32,
        })
        .collect();

    let keyboard_layout = niri.keyboard_layouts.keyboard_layouts.as_ref().map_or_else(
        || "Unknown".to_string(),
        |k| {
            k.names
                .get(k.current_idx as usize)
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string())
        },
    );

    CompositorState {
        workspaces,
        monitors,
        active_workspace_ids,
        focused_workspace_id,
        active_window,
        windows,
        keyboard_layout,
        submap: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use niri_ipc::{Window, WindowLayout};

    fn window(id: u64, tile_size: (f64, f64), view_pos: Option<(f64, f64)>) -> Window {
        Window {
            id,
            title: None,
            app_id: None,
            pid: None,
            workspace_id: Some(1),
            is_focused: false,
            is_floating: false,
            is_urgent: false,
            layout: WindowLayout {
                pos_in_scrolling_layout: Some((1, 1)),
                tile_size,
                window_size: (0, 0),
                tile_pos_in_workspace_view: view_pos,
                window_offset_in_tile: (0.0, 0.0),
            },
            focus_timestamp: None,
        }
    }

    fn state_with(windows: Vec<Window>) -> EventStreamState {
        let mut state = EventStreamState::default();
        state.apply(Event::WindowsChanged { windows });
        state
    }

    #[test]
    fn hotplug_and_config_reload_refresh_outputs() {
        let state = EventStreamState::default();
        let hotplug = Event::WorkspacesChanged { workspaces: vec![] };
        let reload = Event::ConfigLoaded { failed: false };
        assert!(outputs_may_have_changed(&hotplug, &state));
        assert!(outputs_may_have_changed(&reload, &state));
    }

    #[test]
    fn resized_tiles_refresh_outputs() {
        let state = state_with(vec![window(1, (800.0, 1000.0), Some((0.0, 40.0)))]);
        let rescaled = Event::WindowLayoutsChanged {
            changes: vec![(1, window(1, (400.0, 500.0), Some((0.0, 40.0))).layout)],
        };
        assert!(outputs_may_have_changed(&rescaled, &state));
    }

    #[test]
    fn scrolling_does_not_refresh_outputs() {
        let state = state_with(vec![window(1, (800.0, 1000.0), Some((0.0, 40.0)))]);
        let scrolled = Event::WindowLayoutsChanged {
            changes: vec![(1, window(1, (800.0, 1000.0), Some((-200.0, 40.0))).layout)],
        };
        let unknown = Event::WindowLayoutsChanged {
            changes: vec![(2, window(2, (400.0, 500.0), None).layout)],
        };
        assert!(!outputs_may_have_changed(&scrolled, &state));
        assert!(!outputs_may_have_changed(&unknown, &state));
    }
}
