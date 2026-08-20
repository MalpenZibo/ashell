//! Wayfire backend. Wayfire exposes a length-prefixed JSON IPC socket
//! (`WAYFIRE_SOCKET`, provided by the `ipc` and `ipc-rules` plugins) and models
//! workspaces as a per-output 2D grid, so every (output, grid cell) pair becomes
//! one ashell workspace. Switching workspaces additionally needs the `vswitch`
//! plugin.

use super::types::{
    ActiveWindow, ActiveWindowWayfire, CompositorCommand, CompositorEvent, CompositorMonitor,
    CompositorService, CompositorState, CompositorWorkspace,
};
use crate::services::ServiceEvent;
use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::os::unix::net::UnixStream as StdUnixStream;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
    sync::{Mutex as AsyncMutex, broadcast, mpsc},
    time::timeout,
};

/// Events that change anything the bar displays. Every name here exists in
/// Wayfire's `ipc-rules` event map.
const WATCHED_EVENTS: [&str; 17] = [
    "view-mapped",
    "view-unmapped",
    "view-focused",
    "view-title-changed",
    "view-app-id-changed",
    "view-minimized",
    "view-fullscreen",
    "view-sticky",
    "view-workspace-changed",
    "view-set-output",
    "view-wset-changed",
    "output-added",
    "output-removed",
    "output-gain-focus",
    "output-layout-changed",
    "output-wset-changed",
    "wset-workspace-changed",
];

/// A single user action makes Wayfire emit several events, and an app updating
/// its title emits one per change; wait for such a burst to settle instead of
/// refetching the whole state for every event.
const EVENT_COALESCE: Duration = Duration::from_millis(50);

#[derive(Deserialize)]
struct WfOutput {
    id: i64,
    name: String,
    #[serde(default)]
    geometry: WfGeometry,
    workspace: WfWorkspaceInfo,
}

impl WfOutput {
    /// Grid size, guaranteed to be at least 1x1.
    fn grid(&self) -> (i64, i64) {
        (
            self.workspace.grid_width.unwrap_or(1).max(1),
            self.workspace.grid_height.unwrap_or(1).max(1),
        )
    }
}

/// Wayfire nests the workspace grid inside the output's `workspace` object, and
/// spells the grid keys with underscores while everything else uses dashes.
#[derive(Deserialize)]
struct WfWorkspaceInfo {
    x: i64,
    y: i64,
    #[serde(default)]
    grid_width: Option<i64>,
    #[serde(default)]
    grid_height: Option<i64>,
}

#[derive(Deserialize, Default)]
struct WfGeometry {
    #[serde(default)]
    x: i64,
    #[serde(default)]
    y: i64,
    #[serde(default)]
    width: i64,
    #[serde(default)]
    height: i64,
}

#[derive(Deserialize)]
struct WfView {
    id: i64,
    #[serde(default)]
    title: Option<String>,
    #[serde(rename = "app-id", default)]
    app_id: Option<String>,
    #[serde(rename = "output-id", default)]
    output_id: Option<i64>,
    #[serde(default)]
    geometry: WfGeometry,
    #[serde(default)]
    role: String,
    #[serde(default)]
    mapped: bool,
}

#[derive(Deserialize)]
struct WfKeyboardState {
    #[serde(default)]
    layout: Option<String>,
    #[serde(rename = "layout-index", default)]
    layout_index: Option<i64>,
    #[serde(rename = "possible-layouts", default)]
    possible_layouts: Vec<String>,
}

/// Mapping from an ashell workspace id back to the Wayfire grid cell it stands
/// for, rebuilt on every state refresh.
#[derive(Clone, Copy)]
struct WfCell {
    id: i32,
    output_id: i64,
    x: i64,
    y: i64,
}

static SLOT_MAP: OnceLock<Mutex<Vec<WfCell>>> = OnceLock::new();

fn slot_map() -> &'static Mutex<Vec<WfCell>> {
    SLOT_MAP.get_or_init(|| Mutex::new(Vec::new()))
}

/// Per-output id offset. Workspace ids have to be positive — the workspaces
/// module routes `id <= 0` to the special-workspace handling — and for the first
/// output they should line up with `workspace_names`, so cell `n` of output `i`
/// becomes `i * OUTPUT_STRIDE + n + 1`.
const OUTPUT_STRIDE: i64 = 100;

fn encode_workspace_id(output_index: usize, slot: i64) -> Option<i32> {
    (0..OUTPUT_STRIDE)
        .contains(&slot)
        .then(|| (output_index as i64 * OUTPUT_STRIDE + slot + 1) as i32)
}

pub fn is_available() -> bool {
    env::var_os("WAYFIRE_SOCKET").is_some()
}

pub async fn run_listener(tx: &broadcast::Sender<ServiceEvent<CompositorService>>) -> Result<()> {
    let mut stream = connect().await?;
    watch_events(&mut stream).await?;

    // Publish a snapshot up front: otherwise the bar stays empty until the
    // compositor happens to emit its first event.
    send_state(tx, &fetch_full_state().await?);

    // Reading events in a separate task keeps the coalescing timeout below from
    // ever cancelling a half-read message and desyncing the stream.
    let (events_tx, mut events_rx) = mpsc::channel::<()>(1);
    tokio::spawn(async move {
        loop {
            match read_message(&mut stream).await {
                Ok(msg) => {
                    if msg.get("event").is_some()
                        // A full one-slot channel already means "refresh pending".
                        && let Err(mpsc::error::TrySendError::Closed(())) = events_tx.try_send(())
                    {
                        break;
                    }
                }
                Err(e) => {
                    log::warn!("Wayfire event stream ended: {e}");
                    break;
                }
            }
        }
    });

    while events_rx.recv().await.is_some() {
        while matches!(
            timeout(EVENT_COALESCE, events_rx.recv()).await,
            Ok(Some(()))
        ) {}

        match fetch_full_state().await {
            Ok(state) => send_state(tx, &state),
            Err(e) => log::warn!("Wayfire state refresh failed: {e}"),
        }
    }

    Err(anyhow!("Wayfire event stream closed"))
}

async fn watch_events(stream: &mut UnixStream) -> Result<()> {
    const METHOD: &str = "window-rules/events/watch";

    // Wayfire rejects the whole subscription when a single event name is unknown
    // to it, so a version that lacks one of them falls back to watching every
    // event instead (an omitted list means "all").
    let curated = serde_json::json!({ "events": WATCHED_EVENTS });
    let response = request_on_stream(stream, METHOD, &curated).await?;
    if let Err(e) = check_error(METHOD, response) {
        log::info!("{e}; subscribing to all Wayfire events instead");
        let response = request_on_stream(stream, METHOD, &serde_json::json!({})).await?;
        check_error(METHOD, response)?;
    }

    Ok(())
}

pub async fn execute_command(cmd: CompositorCommand) -> Result<()> {
    match cmd {
        CompositorCommand::FocusWorkspace(id) => focus_workspace(id).await,
        CompositorCommand::ScrollWorkspace(dir) => scroll_workspace(dir).await,
        CompositorCommand::NextLayout => next_layout().await,
        other => Err(anyhow!("{other:?} is not supported on the Wayfire backend")),
    }
}

fn send_state(tx: &broadcast::Sender<ServiceEvent<CompositorService>>, state: &CompositorState) {
    let _ = tx.send(ServiceEvent::Update(CompositorEvent::StateChanged(
        Box::new(state.clone()),
    )));
}

async fn connect() -> Result<UnixStream> {
    let socket_path = env::var_os("WAYFIRE_SOCKET")
        .ok_or_else(|| anyhow!("WAYFIRE_SOCKET environment variable not set"))?;

    let std_stream = StdUnixStream::connect(socket_path)?;
    std_stream.set_nonblocking(true)?;
    UnixStream::from_std(std_stream).context("Failed to convert Wayfire socket")
}

static REQUEST_STREAM: OnceLock<AsyncMutex<Option<UnixStream>>> = OnceLock::new();

fn request_stream() -> &'static AsyncMutex<Option<UnixStream>> {
    REQUEST_STREAM.get_or_init(|| AsyncMutex::new(None))
}

/// Wayfire answers requests in order on the connection they arrived on, so one
/// long-lived stream is reused for every method call — a state refresh would
/// otherwise open a socket per request.
async fn call_method(method: &str, data: &Value) -> Result<Value> {
    let mut guard = request_stream().lock().await;

    // The stream is taken out for the duration of the call: a failed (or
    // cancelled) request can leave a partially read message behind, and such a
    // desynced connection must not be reused.
    let mut response = None;
    if let Some(mut stream) = guard.take() {
        match request_on_stream(&mut stream, method, data).await {
            Ok(r) => {
                *guard = Some(stream);
                response = Some(r);
            }
            Err(e) => log::debug!("Wayfire IPC connection dropped ({method}): {e}; reconnecting"),
        }
    }

    let response = match response {
        Some(response) => response,
        None => {
            let mut stream = connect().await?;
            let response = request_on_stream(&mut stream, method, data).await?;
            *guard = Some(stream);
            response
        }
    };

    check_error(method, response)
}

/// Sends one request and reads its reply; fails only on transport or parse
/// errors, so the caller can tell a broken connection from an IPC error reply.
async fn request_on_stream(stream: &mut UnixStream, method: &str, data: &Value) -> Result<Value> {
    let request = serde_json::json!({
        "method": method,
        "data": data,
    });
    write_message(stream, &request).await?;
    read_message(stream).await
}

fn check_error(method: &str, response: Value) -> Result<Value> {
    if let Some(error) = response.get("error").and_then(|e| e.as_str()) {
        return Err(anyhow!("Wayfire IPC error ({method}): {error}"));
    }

    Ok(response)
}

/// `get-focused-view` and `get-focused-output` wrap their payload as
/// `{"result": "ok", "info": …}`, with `info` set to `null` when nothing is
/// focused.
fn take_info(mut response: Value) -> Option<Value> {
    let info = response.get_mut("info")?.take();
    (!info.is_null()).then_some(info)
}

async fn write_message(stream: &mut UnixStream, msg: &Value) -> Result<()> {
    let json = serde_json::to_vec(msg)?;
    let len = (json.len() as u32).to_le_bytes();
    stream.write_all(&len).await?;
    stream.write_all(&json).await?;
    stream.flush().await?;
    Ok(())
}

async fn read_message(stream: &mut UnixStream) -> Result<Value> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_le_bytes(len_buf) as usize;

    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;

    serde_json::from_slice(&buf).context("Failed to parse Wayfire IPC response")
}

async fn list_outputs() -> Result<Vec<WfOutput>> {
    let response = call_method("window-rules/list-outputs", &serde_json::json!({})).await?;
    serde_json::from_value(response).context("Failed to parse Wayfire list-outputs")
}

async fn focused_output() -> Result<WfOutput> {
    let response = call_method("window-rules/get-focused-output", &serde_json::json!({})).await?;
    let info = take_info(response).ok_or_else(|| anyhow!("Wayfire reports no focused output"))?;
    serde_json::from_value(info).context("Failed to parse Wayfire get-focused-output")
}

async fn fetch_full_state() -> Result<CompositorState> {
    let outputs = list_outputs().await?;

    let focused_output_id = focused_output()
        .await
        .inspect_err(|e| log::debug!("Wayfire focused output unavailable: {e}"))
        .map(|output| output.id)
        .ok();

    let views_resp = call_method("window-rules/list-views", &serde_json::json!({})).await?;
    let views: Vec<WfView> =
        serde_json::from_value(views_resp).context("Failed to parse Wayfire list-views")?;

    let focused_resp = call_method("window-rules/get-focused-view", &serde_json::json!({})).await?;
    let focused_view = take_info(focused_resp).and_then(|info| {
        serde_json::from_value::<WfView>(info)
            .inspect_err(|e| log::debug!("Failed to parse Wayfire focused view: {e}"))
            .ok()
    });

    // Available since Wayfire 0.9; a missing method only costs the layout name.
    let kb_state = match call_method("wayfire/get-keyboard-state", &serde_json::json!({})).await {
        Ok(response) => serde_json::from_value::<WfKeyboardState>(response)
            .inspect_err(|e| log::debug!("Failed to parse Wayfire keyboard state: {e}"))
            .ok(),
        Err(e) => {
            log::debug!("Wayfire keyboard state unavailable: {e}");
            None
        }
    };

    Ok(build_state(
        &outputs,
        &views,
        focused_output_id,
        focused_view.as_ref(),
        kb_state.as_ref(),
    ))
}

/// Wayfire keeps view geometry in output-local coordinates relative to the
/// *current* workspace: the cell at `(x, y)` spans
/// `((x - current.x) * width, (y - current.y) * height)`. The workspace holding
/// a view therefore follows from its centre point.
fn view_workspace(view: &WfView, output: &WfOutput) -> (i64, i64) {
    let (grid_w, grid_h) = output.grid();
    let width = output.geometry.width.max(1);
    let height = output.geometry.height.max(1);

    let x = output.workspace.x + (view.geometry.x + view.geometry.width / 2).div_euclid(width);
    let y = output.workspace.y + (view.geometry.y + view.geometry.height / 2).div_euclid(height);

    (x.clamp(0, grid_w - 1), y.clamp(0, grid_h - 1))
}

#[derive(Default)]
struct Occupancy {
    windows: u16,
    classes: Vec<String>,
}

fn build_state(
    outputs: &[WfOutput],
    views: &[WfView],
    focused_output_id: Option<i64>,
    focused_view: Option<&WfView>,
    kb_state: Option<&WfKeyboardState>,
) -> CompositorState {
    let collect_classes = super::should_collect_window_classes();
    let outputs_by_id: HashMap<i64, &WfOutput> = outputs.iter().map(|o| (o.id, o)).collect();

    // Wayfire has no per-workspace view list, so the views are bucketed into the
    // grid cell each one sits on.
    let mut occupancy: HashMap<(i64, i64, i64), Occupancy> = HashMap::new();
    for view in views {
        if !view.mapped || view.role != "toplevel" {
            continue;
        }

        let Some(output) = view.output_id.and_then(|id| outputs_by_id.get(&id)) else {
            continue;
        };

        let (x, y) = view_workspace(view, output);
        let cell = occupancy.entry((output.id, x, y)).or_default();
        cell.windows = cell.windows.saturating_add(1);
        if collect_classes && let Some(class) = view.app_id.as_ref().filter(|c| !c.is_empty()) {
            cell.classes.push(class.clone());
        }
    }

    let mut workspaces = Vec::new();
    let mut monitors = Vec::new();
    let mut active_workspace_ids = Vec::new();
    let mut new_slots = Vec::new();

    for (output_index, output) in outputs.iter().enumerate() {
        let (grid_w, grid_h) = output.grid();
        let current_x = output.workspace.x.clamp(0, grid_w - 1);
        let current_y = output.workspace.y.clamp(0, grid_h - 1);

        for slot in 0..grid_w * grid_h {
            let (x, y) = (slot % grid_w, slot / grid_w);
            let Some(id) = encode_workspace_id(output_index, slot) else {
                log::warn!(
                    "Wayfire workspace grid on output {} exceeds {OUTPUT_STRIDE} cells; ignoring the rest",
                    output.name
                );
                break;
            };

            let cell = occupancy.remove(&(output.id, x, y)).unwrap_or_default();

            workspaces.push(CompositorWorkspace {
                id,
                index: (slot + 1) as i32,
                name: (slot + 1).to_string(),
                monitor: output.name.clone(),
                monitor_id: Some(output_index as i128),
                windows: cell.windows,
                is_special: false,
                has_urgent: false,
                window_classes: cell.classes,
            });

            new_slots.push(WfCell {
                id,
                output_id: output.id,
                x,
                y,
            });

            // Only the focused output's current cell counts as active; the other
            // outputs' current cells are reported as visible through
            // `CompositorMonitor::active_workspace_id`, as on Hyprland and Niri.
            if x == current_x && y == current_y && Some(output.id) == focused_output_id {
                active_workspace_ids.push(id);
            }
        }

        monitors.push(CompositorMonitor {
            id: output_index as i128,
            name: output.name.clone(),
            active_workspace_id: encode_workspace_id(output_index, current_y * grid_w + current_x)
                .unwrap_or(-1),
            special_workspace_id: -1,
        });
    }

    {
        let mut slots = slot_map().lock().unwrap();
        *slots = new_slots;
    }

    let active_window = focused_view.and_then(|v| {
        let title = v.title.clone().unwrap_or_default();
        let class = v.app_id.clone().unwrap_or_default();
        if title.is_empty() && class.is_empty() {
            None
        } else {
            Some(ActiveWindow::Wayfire(ActiveWindowWayfire {
                title,
                class,
                address: v.id.to_string(),
            }))
        }
    });

    let keyboard_layout = kb_state
        .and_then(|kb| kb.layout.clone())
        .filter(|layout| !layout.is_empty())
        .unwrap_or_else(|| "Unknown".to_string());

    CompositorState {
        workspaces,
        monitors,
        active_workspace_ids,
        active_window,
        keyboard_layout,
        submap: None,
    }
}

async fn set_workspace(output_id: i64, x: i64, y: i64) -> Result<()> {
    let data = serde_json::json!({
        "x": x,
        "y": y,
        "output-id": output_id,
    });
    call_method("vswitch/set-workspace", &data)
        .await
        .map(|_| ())
}

async fn focus_workspace(id: i32) -> Result<()> {
    let cell = find_cell(id)?;
    set_workspace(cell.output_id, cell.x, cell.y).await
}

/// `dir > 0` moves to the previous workspace, matching the Niri and MangoWC
/// backends and the scroll direction of the workspaces module.
async fn scroll_workspace(dir: i32) -> Result<()> {
    let output = focused_output().await?;

    let (grid_w, grid_h) = output.grid();
    let cells = grid_w * grid_h;
    let current =
        output.workspace.y.clamp(0, grid_h - 1) * grid_w + output.workspace.x.clamp(0, grid_w - 1);
    let target = (current + if dir > 0 { -1 } else { 1 }).rem_euclid(cells);

    set_workspace(output.id, target % grid_w, target / grid_w).await
}

async fn next_layout() -> Result<()> {
    let kb_resp = call_method("wayfire/get-keyboard-state", &serde_json::json!({})).await?;
    let kb: WfKeyboardState =
        serde_json::from_value(kb_resp).context("Failed to parse Wayfire keyboard state")?;

    if kb.possible_layouts.len() < 2 {
        return Err(anyhow!("Wayfire reports fewer than two keyboard layouts"));
    }

    let next_idx = (kb.layout_index.unwrap_or(0) + 1).rem_euclid(kb.possible_layouts.len() as i64);

    let data = serde_json::json!({ "layout-index": next_idx });
    call_method("wayfire/set-keyboard-state", &data)
        .await
        .map(|_| ())
}

fn find_cell(id: i32) -> Result<WfCell> {
    let slots = slot_map().lock().unwrap();
    slots
        .iter()
        .find(|c| c.id == id)
        .copied()
        .ok_or_else(|| anyhow!("Workspace id {id} not found in slot map"))
}
