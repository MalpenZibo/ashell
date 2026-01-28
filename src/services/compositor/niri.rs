use super::types::{
    ActiveWindow, ActiveWindowNiri, CompositorCommand, CompositorEvent, CompositorMonitor,
    CompositorService, CompositorState, CompositorWorkspace,
};
use crate::services::ServiceEvent;
use anyhow::{Context, Result, anyhow};
use itertools::Itertools;
use niri_ipc::{
    Action, Event, Reply, Request, WorkspaceReferenceArg,
    state::{EventStreamState, EventStreamStatePart},
};
use std::{env, os::unix::net::UnixStream as StdUnixStream};
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
        CompositorCommand::FocusSpecialWorkspace(_) => {
            return Err(anyhow!("Special workspaces not supported in Niri backend"));
        }
        CompositorCommand::ToggleSpecialWorkspace(_) => {
            return Err(anyhow!("Special workspaces not supported in Niri backend"));
        }
        CompositorCommand::FocusMonitor(_) => {
            return Err(anyhow!("FocusMonitor by ID not supported in Niri backend"));
        }
        CompositorCommand::ScrollWorkspace(dir) => {
            if dir > 0 {
                Action::FocusWorkspaceUp {}
            } else {
                Action::FocusWorkspaceDown {}
            }
        }
        CompositorCommand::NextLayout => Action::SwitchLayout {
            layout: niri_ipc::LayoutSwitchTarget::Next,
        },
        CompositorCommand::CustomDispatch(action, args) => {
            if action == "spawn" {
                Action::Spawn {
                    command: vec![args],
                }
            } else {
                return Err(anyhow!("Unknown custom dispatch: {}", action));
            }
        }
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

        // Apply to internal Niri state tracker
        internal_state.apply(event);

        // Map to generic Ashell state
        let state = map_state(&internal_state);

        // Emit Update
        let _ = tx.send(ServiceEvent::Update(CompositorEvent::StateChanged(state)));
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
    let mut json = serde_json::to_string(&request)?;
    json.push('\n');
    stream.write_all(json.as_bytes()).await?;
    stream.flush().await?;

    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line).await?;

    let reply: Reply = serde_json::from_str(&response_line)?;
    reply.map_err(|e| anyhow!("Niri error: {}", e)).map(|_| ())
}

fn map_state(niri: &EventStreamState) -> CompositorState {
    let output_to_active_ws: std::collections::HashMap<_, _> = niri
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
    let outputs = output_to_active_ws
        .keys()
        .sorted_unstable()
        .collect::<Vec<_>>();

    let mut workspaces: Vec<CompositorWorkspace> = niri
        .workspaces
        .workspaces
        .values()
        .sorted_by_key(|w| w.idx)
        .map(|w| {
            CompositorWorkspace {
                id: w.id as i32,
                index: w.idx as i32,
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
            }
        })
        .collect();

    // Calculate window counts
    for win in niri.windows.windows.values() {
        if let Some(ws_id) = win.workspace_id {
            // Resolve Niri Workspace ID (u64) -> Visual Index (u8) -> Generic ID (i32)
            if let Some(ws) = niri.workspaces.workspaces.get(&ws_id)
                && let Some(generic_ws) = workspaces.iter_mut().find(|w| w.id == ws.id as i32)
            {
                generic_ws.windows += 1;
            }
        }
    }

    let mut monitors = Vec::new();
    for (name, active_ws_id) in &output_to_active_ws {
        monitors.push(CompositorMonitor {
            id: outputs
                .iter()
                .position(|o| *o == name)
                .map_or(-1, |i| i as i128),
            name: name.clone(),
            active_workspace_id: *active_ws_id,
            special_workspace_id: -1,
        });
    }

    let active_workspace_id = niri
        .workspaces
        .workspaces
        .values()
        .find(|w| w.is_focused)
        .map(|w| w.id as i32);

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
        active_workspace_id,
        active_window,
        keyboard_layout,
        submap: None,
    }
}
