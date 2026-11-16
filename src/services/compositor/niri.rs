use super::types::{
    ActiveWindow, ActiveWindowNiri, CompositorCommand, CompositorEvent, CompositorMonitor,
    CompositorService, CompositorState, CompositorWorkspace,
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
    let mut output_order = query_output_order().await;

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

        // Niri sends no event when outputs change, but adding or removing one
        // moves workspaces, so the layout is read again on every workspace change.
        let refresh_outputs = matches!(event, Event::WorkspacesChanged { .. });

        // Apply to internal Niri state tracker
        internal_state.apply(event);

        if refresh_outputs {
            output_order = query_output_order().await;
        }

        // Map to generic Ashell state
        let state = map_state(&internal_state, &output_order);

        // Emit Update
        let _ = tx.send(ServiceEvent::Update(CompositorEvent::StateChanged(
            Box::new(state),
        )));
    }

    Ok(())
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

/// Output names in the order niri lays them out, empty if the request fails.
async fn query_output_order() -> Vec<String> {
    let outputs = async {
        let mut stream = connect().await?;
        match send_request(&mut stream, Request::Outputs).await? {
            Response::Outputs(outputs) => Ok(outputs),
            _ => Err(anyhow!("Unexpected reply to a Niri outputs request")),
        }
    }
    .await;

    match outputs {
        Ok(outputs) => order_outputs(
            outputs
                .into_values()
                .map(|o| (o.name, o.logical.map(|l| (l.x, l.y)))),
        ),
        Err(e) => {
            log::warn!("Failed to read Niri outputs, ordering them by name: {e}");
            Vec::new()
        }
    }
}

/// Orders outputs left to right by their logical position, then top to bottom
/// for outputs at the same x. Disabled outputs have no position and go last.
fn order_outputs(outputs: impl Iterator<Item = (String, Option<(i32, i32)>)>) -> Vec<String> {
    outputs
        .sorted_by(|(a_name, a_pos), (b_name, b_pos)| {
            a_pos
                .is_none()
                .cmp(&b_pos.is_none())
                .then(a_pos.cmp(b_pos))
                .then(a_name.cmp(b_name))
        })
        .map(|(name, _)| name)
        .collect()
}

/// Niri keeps one empty, unnamed workspace at the end of every output to open
/// new windows on. It carries nothing until it is in use, so it is shown only
/// while it is the active workspace of its output.
fn is_hidden_placeholder(ws: &niri_ipc::Workspace, last_idx: Option<u8>) -> bool {
    ws.name.is_none() && ws.active_window_id.is_none() && !ws.is_active && last_idx == Some(ws.idx)
}

fn map_state(niri: &EventStreamState, output_order: &[String]) -> CompositorState {
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

    // Outputs in layout order, then any the last outputs request did not
    // report, by name.
    let outputs = output_to_active_ws
        .keys()
        .sorted_by_key(|name| {
            (
                output_order
                    .iter()
                    .position(|o| o == *name)
                    .unwrap_or(usize::MAX),
                *name,
            )
        })
        .collect::<Vec<_>>();
    let output_rank = |output: Option<&String>| {
        output
            .and_then(|o| outputs.iter().position(|known| *known == o))
            .unwrap_or(usize::MAX)
    };

    let mut last_idx: HashMap<Option<&String>, u8> = HashMap::new();
    for ws in niri.workspaces.workspaces.values() {
        let last = last_idx.entry(ws.output.as_ref()).or_default();
        *last = (*last).max(ws.idx);
    }

    let collect_classes = super::should_collect_window_classes();

    // Collect window classes per workspace. Niri's idx restarts on every
    // output, so they are keyed by id.
    let mut workspace_classes: HashMap<u64, Vec<String>> = HashMap::new();
    if collect_classes {
        for win in niri.windows.windows.values() {
            if let Some(ws_id) = win.workspace_id
                && let Some(ws) = niri.workspaces.workspaces.get(&ws_id)
                && let Some(app_id) = &win.app_id
            {
                workspace_classes
                    .entry(ws.id)
                    .or_default()
                    .push(app_id.clone());
            }
        }
    }

    let mut workspaces: Vec<CompositorWorkspace> = niri
        .workspaces
        .workspaces
        .values()
        .filter(|w| !is_hidden_placeholder(w, last_idx.get(&w.output.as_ref()).copied()))
        .sorted_by_key(|w| (output_rank(w.output.as_ref()), w.idx))
        .enumerate()
        .map(|(position, w)| {
            CompositorWorkspace {
                id: w.id as i32,
                // Niri's idx restarts on every output, so the index is the
                // position across all outputs in layout order.
                index: position as i32 + 1,
                name: w.name.clone().unwrap_or_else(|| w.idx.to_string()),
                monitor: w.output.clone().unwrap_or_default(),
                // niri does not have an output index
                monitor_id: w.output.as_ref().map(|wo| {
                    outputs
                        .iter()
                        .position(|o| *o == wo)
                        .map_or(-1, |i| i as i32) as i128
                }),
                windows: 0,
                is_special: false,
                has_urgent: w.is_urgent,
                window_classes: if collect_classes {
                    workspace_classes.remove(&w.id).unwrap_or_default()
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

    let monitors: Vec<CompositorMonitor> = outputs
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

    let active_workspace_ids = niri
        .workspaces
        .workspaces
        .values()
        .find(|w| w.is_focused)
        .map(|w| w.id as i32)
        .into_iter()
        .collect();

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
        active_window,
        keyboard_layout,
        submap: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ws(id: u64, idx: u8, name: Option<&str>, output: &str, active: bool) -> niri_ipc::Workspace {
        niri_ipc::Workspace {
            id,
            idx,
            name: name.map(str::to_owned),
            output: Some(output.to_owned()),
            is_urgent: false,
            is_active: active,
            is_focused: false,
            active_window_id: None,
        }
    }

    fn state(workspaces: Vec<niri_ipc::Workspace>) -> EventStreamState {
        let mut state = EventStreamState::default();
        state.apply(Event::WorkspacesChanged { workspaces });
        state
    }

    fn labels(state: &CompositorState) -> Vec<(i32, &str, &str)> {
        state
            .workspaces
            .iter()
            .sorted_by_key(|w| w.index)
            .map(|w| (w.index, w.name.as_str(), w.monitor.as_str()))
            .collect()
    }

    #[test]
    fn outputs_follow_the_layout() {
        let order = order_outputs(
            [
                ("DP-2".to_owned(), Some((0, 0))),
                ("HDMI-A-1".to_owned(), None),
                ("DP-1".to_owned(), Some((-1920, 0))),
                ("DP-3".to_owned(), Some((0, -1080))),
            ]
            .into_iter(),
        );
        assert_eq!(order, ["DP-1", "DP-3", "DP-2", "HDMI-A-1"]);
    }

    #[test]
    fn workspaces_group_by_output_without_placeholders() {
        // Named 1-5 on the primary DP-2 and 6-9 on DP-1 to its left, each
        // output ending in niri's empty unnamed workspace.
        let mut workspaces = (1..=5)
            .map(|i| ws(i, i as u8, Some(&i.to_string()), "DP-2", i == 3))
            .chain((6..=9).map(|i| ws(i, i as u8 - 5, Some(&i.to_string()), "DP-1", i == 6)))
            .collect_vec();
        workspaces.push(ws(11, 6, None, "DP-2", false));
        workspaces.push(ws(23, 5, None, "DP-1", false));

        let mapped = map_state(&state(workspaces), &["DP-1".to_owned(), "DP-2".to_owned()]);
        assert_eq!(
            labels(&mapped),
            [
                (1, "6", "DP-1"),
                (2, "7", "DP-1"),
                (3, "8", "DP-1"),
                (4, "9", "DP-1"),
                (5, "1", "DP-2"),
                (6, "2", "DP-2"),
                (7, "3", "DP-2"),
                (8, "4", "DP-2"),
                (9, "5", "DP-2"),
            ]
        );
        assert_eq!(
            mapped
                .monitors
                .iter()
                .map(|m| m.name.as_str())
                .collect_vec(),
            ["DP-1", "DP-2"]
        );
    }

    #[test]
    fn placeholder_shows_while_active() {
        let workspaces = vec![
            ws(1, 1, Some("1"), "DP-1", false),
            ws(2, 2, None, "DP-1", true),
        ];
        let mapped = map_state(&state(workspaces), &["DP-1".to_owned()]);
        assert_eq!(labels(&mapped), [(1, "1", "DP-1"), (2, "2", "DP-1")]);
    }

    #[test]
    fn unnamed_workspaces_with_windows_stay() {
        // Only the last workspace of an output is a placeholder, and one
        // with a window is in use.
        let mut in_use = ws(2, 2, None, "DP-1", false);
        in_use.active_window_id = Some(7);
        let workspaces = vec![
            ws(1, 1, None, "DP-1", true),
            in_use,
            ws(3, 3, None, "DP-1", false),
        ];
        let mapped = map_state(&state(workspaces), &["DP-1".to_owned()]);
        assert_eq!(labels(&mapped), [(1, "1", "DP-1"), (2, "2", "DP-1")]);
    }

    #[test]
    fn unknown_outputs_sort_by_name_after_known_ones() {
        let workspaces = vec![
            ws(1, 1, Some("a"), "HDMI-A-1", true),
            ws(2, 1, Some("b"), "DP-1", true),
            ws(3, 1, Some("c"), "DP-2", true),
        ];
        let mapped = map_state(&state(workspaces), &["DP-2".to_owned()]);
        assert_eq!(
            labels(&mapped),
            [(1, "c", "DP-2"), (2, "b", "DP-1"), (3, "a", "HDMI-A-1")]
        );
    }
}
