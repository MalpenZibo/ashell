use super::types::{
    ActiveWindow, ActiveWindowNiri, CompositorCommand, CompositorMonitor, CompositorState,
    CompositorStateWriters, CompositorWorkspace,
};
use anyhow::{Context as _, anyhow};
use itertools::Itertools;
use niri_ipc::{
    Action, Event, Reply, Request, WorkspaceReferenceArg,
    state::{EventStreamState, EventStreamStatePart},
};
use std::{env, os::unix::net::UnixStream as StdUnixStream};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};

pub async fn execute_command(cmd: CompositorCommand) -> anyhow::Result<()> {
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

pub async fn run_listener(state: CompositorStateWriters) -> anyhow::Result<()> {
    let mut stream = connect().await?;

    let request_json = serde_json::to_string(&Request::EventStream)? + "\n";
    stream.write_all(request_json.as_bytes()).await?;
    stream.flush().await?;

    let mut reader = BufReader::new(stream);

    // Read the Handled response
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let reply: Reply = serde_json::from_str(&line).context("Failed to parse handshake")?;
    if let Err(e) = reply {
        return Err(anyhow!("Niri refused EventStream: {}", e));
    }

    // Shutdown write half
    let _ = reader.get_mut().shutdown().await;

    let mut internal_state = EventStreamState::default();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            break;
        }

        let event: Event = match serde_json::from_str(&line) {
            Ok(ev) => ev,
            Err(e) => {
                log::debug!(
                    "Failed to parse Niri event (IPC version mismatch) -> {:?}",
                    e
                );
                continue;
            }
        };

        internal_state.apply(event);

        let new_state = map_state(&internal_state);
        state.set(new_state);
    }

    Ok(())
}

async fn connect() -> anyhow::Result<UnixStream> {
    let socket_path = env::var_os("NIRI_SOCKET")
        .or_else(|| env::var_os("NIRI_SOCKET_PATH"))
        .ok_or_else(|| anyhow!("NIRI_SOCKET or NIRI_SOCKET_PATH environment variable not set"))?;

    let std_stream = StdUnixStream::connect(socket_path)?;
    std_stream.set_nonblocking(true)?;
    UnixStream::from_std(std_stream).context("Failed to convert stream")
}

async fn send_command_request(stream: &mut UnixStream, request: Request) -> anyhow::Result<()> {
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

    let outputs = output_to_active_ws
        .keys()
        .sorted_unstable()
        .collect::<Vec<_>>();

    let mut workspaces: Vec<CompositorWorkspace> = niri
        .workspaces
        .workspaces
        .values()
        .sorted_by_key(|w| w.idx)
        .map(|w| CompositorWorkspace {
            id: w.id as i32,
            index: w.idx as i32,
            name: w.name.clone().unwrap_or_else(|| w.idx.to_string()),
            monitor: w.output.clone().unwrap_or_default(),
            monitor_id: w.output.as_ref().map(|wo| {
                outputs
                    .iter()
                    .position(|o| *o == wo)
                    .map_or(-1, |i| i as i32) as i128
            }),
            windows: 0,
            is_special: false,
        })
        .collect();

    // Calculate window counts
    for win in niri.windows.windows.values() {
        if let Some(ws_id) = win.workspace_id
            && let Some(ws) = niri.workspaces.workspaces.get(&ws_id)
            && let Some(generic_ws) = workspaces.iter_mut().find(|w| w.id == ws.id as i32)
        {
            generic_ws.windows += 1;
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
