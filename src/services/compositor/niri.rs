use super::types::{
    ActiveWindow, CompositorCommand, CompositorEvent, CompositorMonitor, CompositorService,
    CompositorState, CompositorWorkspace,
};
use crate::services::ServiceEvent;
use anyhow::{Context, Result, anyhow};
use iced::futures::channel::mpsc::Sender;
use niri_ipc::{
    Action, Event, Reply, Request, WorkspaceReferenceArg,
    state::{EventStreamState, EventStreamStatePart},
};
use std::{
    collections::hash_map::DefaultHasher,
    env,
    hash::{Hash, Hasher},
    os::unix::net::UnixStream as StdUnixStream,
    sync::{Arc, RwLock},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};

// --- Public Interface ---

pub async fn execute_command(cmd: CompositorCommand) -> Result<()> {
    let mut stream = connect().await?;

    let action = match cmd {
        CompositorCommand::FocusWorkspace(id) => match u8::try_from(id) {
            Ok(idx) => Action::FocusWorkspace {
                reference: WorkspaceReferenceArg::Index(idx),
            },
            Err(_) => return Err(anyhow!("Invalid workspace index for Niri (must be u8)")),
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
                Action::FocusWorkspaceDown {}
            } else {
                Action::FocusWorkspaceUp {}
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

pub async fn run_listener(
    output: Arc<RwLock<Sender<ServiceEvent<CompositorService>>>>,
) -> Result<()> {
    // 1. Send Initial "Empty" State
    // This is critical: Modules wait for Init before processing Updates.
    if let Ok(mut o) = output.write() {
        let _ = o.try_send(ServiceEvent::Init(CompositorService {
            state: CompositorState::default(),
        }));
    }

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

        let event: Event = serde_json::from_str(&line).context("Failed to parse Niri event")?;

        // Apply to internal Niri state tracker
        internal_state.apply(event);

        // Map to generic Ashell state
        let state = map_state(&internal_state);

        // Emit Update
        if let Ok(mut o) = output.write() {
            let _ = o.try_send(ServiceEvent::Update(CompositorEvent::StateChanged(state)));
        } else {
            log::error!("Failed to acquire output lock in Niri listener");
        }
    }

    Ok(())
}

// --- Internal Helpers ---

async fn connect() -> Result<UnixStream> {
    let socket_path = env::var_os("NIRI_SOCKET")
        .or_else(|| env::var_os("NIRI_SOCKET_PATH"))
        .ok_or_else(|| anyhow!("NIRI_SOCKET environment variable not set"))?;

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
    let mut workspaces: Vec<CompositorWorkspace> = niri
        .workspaces
        .workspaces
        .values()
        .map(|w| {
            CompositorWorkspace {
                id: w.idx as i32, // Visual index as ID
                name: w.name.clone().unwrap_or_else(|| w.idx.to_string()),
                monitor: w.output.clone().unwrap_or_default(),
                monitor_id: w.output.as_ref().map(|n| hash_string(n)),
                windows: 0,
                is_special: false,
            }
        })
        .collect();

    // Calculate window counts
    for win in niri.windows.windows.values() {
        if let Some(ws_id) = win.workspace_id {
            // Resolve Niri Workspace ID (u64) -> Visual Index (u8) -> Generic ID (i32)
            if let Some(ws) = niri.workspaces.workspaces.get(&ws_id) {
                if let Some(generic_ws) = workspaces.iter_mut().find(|w| w.id == ws.idx as i32) {
                    generic_ws.windows += 1;
                }
            }
        }
    }

    workspaces.sort_by_key(|w| w.id);

    let mut monitors = Vec::new();
    let mut output_to_active_ws = std::collections::HashMap::new();
    for ws in niri.workspaces.workspaces.values() {
        if let Some(out) = &ws.output {
            if ws.is_active {
                output_to_active_ws.insert(out.clone(), ws.idx as i32);
            }
        }
    }

    for (name, active_ws_id) in output_to_active_ws {
        monitors.push(CompositorMonitor {
            id: hash_string(&name),
            name: name.clone(),
            active_workspace_id: active_ws_id,
            special_workspace_id: -1,
        });
    }

    let active_workspace_id = niri
        .workspaces
        .workspaces
        .values()
        .find(|w| w.is_focused)
        .map(|w| w.idx as i32);

    let active_window = niri
        .windows
        .windows
        .values()
        .find(|w| w.is_focused)
        .map(|w| ActiveWindow {
            title: w.title.clone().unwrap_or_default(),
            class: w.app_id.clone().unwrap_or_default(),
            address: w.id.to_string(),
        });

    let keyboard_layout = niri
        .keyboard_layouts
        .keyboard_layouts
        .as_ref()
        .map(|k| {
            k.names
                .get(k.current_idx as usize)
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string())
        })
        .unwrap_or_else(|| "Unknown".to_string());

    CompositorState {
        workspaces,
        monitors,
        active_workspace_id,
        active_window,
        keyboard_layout,
        submap: None,
    }
}

fn hash_string(s: &str) -> i128 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish() as i128
}
