//! Wayfire backend: length-prefixed JSON IPC on `WAYFIRE_SOCKET` (needs the
//! `ipc` and `ipc-rules` plugins; switching needs `vswitch`). Each (output,
//! grid cell) pair becomes one ashell workspace.

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
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
    sync::{Mutex as AsyncMutex, broadcast, mpsc},
    time::timeout,
};

/// Events that change anything the bar displays.
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

/// Coalesce the burst of events one user action produces.
const EVENT_COALESCE: Duration = Duration::from_millis(50);

/// Since 0.11 Wayfire's `geometry_t` is double-based, so geometry arrives as
/// `"width": 2240.0` and serde rejects it for integer fields; accept both.
fn de_number<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct NumberVisitor;

    impl<'de> serde::de::Visitor<'de> for NumberVisitor {
        type Value = i64;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a JSON number")
        }

        fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<i64, E> {
            Ok(value)
        }

        fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<i64, E> {
            Ok(value as i64)
        }

        fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<i64, E> {
            Ok(value as i64)
        }
    }

    deserializer.deserialize_any(NumberVisitor)
}

fn de_opt_number<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<serde_json::Value>::deserialize(deserializer)?.map_or(Ok(None), |value| match value {
        serde_json::Value::Number(number) => {
            if let Some(int) = number.as_i64() {
                Ok(Some(int))
            } else {
                number
                    .as_f64()
                    .map(|float| Some(float as i64))
                    .ok_or_else(|| serde::de::Error::custom(format!("unexpected number {number}")))
            }
        }
        other => Err(serde::de::Error::custom(format!(
            "expected a number, got {other}"
        ))),
    })
}

#[derive(Deserialize)]
struct WfOutput {
    #[serde(deserialize_with = "de_number")]
    id: i64,
    name: String,
    #[serde(default)]
    geometry: WfGeometry,
    workspace: WfWorkspaceInfo,
}

impl WfOutput {
    fn grid(&self) -> (i64, i64) {
        (
            self.workspace.grid_width.unwrap_or(1).max(1),
            self.workspace.grid_height.unwrap_or(1).max(1),
        )
    }
}

/// Wayfire nests the grid inside `workspace` and spells those keys with underscores.
#[derive(Deserialize)]
struct WfWorkspaceInfo {
    #[serde(deserialize_with = "de_number")]
    x: i64,
    #[serde(deserialize_with = "de_number")]
    y: i64,
    #[serde(default, deserialize_with = "de_opt_number")]
    grid_width: Option<i64>,
    #[serde(default, deserialize_with = "de_opt_number")]
    grid_height: Option<i64>,
}

#[derive(Deserialize, Default)]
struct WfGeometry {
    #[serde(default, deserialize_with = "de_number")]
    x: i64,
    #[serde(default, deserialize_with = "de_number")]
    y: i64,
    #[serde(default, deserialize_with = "de_number")]
    width: i64,
    #[serde(default, deserialize_with = "de_number")]
    height: i64,
}

#[derive(Deserialize)]
struct WfView {
    #[serde(default, deserialize_with = "de_number")]
    id: i64,
    #[serde(default)]
    title: Option<String>,
    #[serde(rename = "app-id", default)]
    app_id: Option<String>,
    #[serde(rename = "output-id", default, deserialize_with = "de_opt_number")]
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
    #[serde(rename = "layout-index", default, deserialize_with = "de_opt_number")]
    layout_index: Option<i64>,
    #[serde(rename = "possible-layouts", default)]
    possible_layouts: Vec<String>,
}

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

/// Workspace ids must stay positive; the offset keeps outputs from colliding and
/// lines the first output up with `workspace_names`.
const OUTPUT_STRIDE: i64 = 100;

fn encode_workspace_id(output_index: usize, slot: i64) -> Option<i32> {
    (0..OUTPUT_STRIDE)
        .contains(&slot)
        .then(|| (output_index as i64 * OUTPUT_STRIDE + slot + 1) as i32)
}

/// `WAYFIRE_SOCKET` is the documented source; fall back to the socket Wayfire
/// drops in the runtime directory when the variable was not inherited.
fn socket_path() -> Option<PathBuf> {
    if let Some(path) = env::var_os("WAYFIRE_SOCKET").filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(path));
    }

    let runtime_dir = PathBuf::from(env::var_os("XDG_RUNTIME_DIR")?);
    let display = env::var("WAYLAND_DISPLAY").unwrap_or_default();

    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&runtime_dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("wayfire-") && name.ends_with(".socket"))
        })
        .collect();

    candidates.sort_by_key(|path| {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        (!name.contains(&display)) as u8
    });

    candidates.into_iter().next()
}

pub fn is_available() -> bool {
    socket_path().is_some()
}

pub async fn run_listener(tx: &broadcast::Sender<ServiceEvent<CompositorService>>) -> Result<()> {
    log::warn!("Wayfire backend: starting listener");
    let mut stream = connect().await?;
    watch_events(&mut stream).await?;
    log::warn!("Wayfire backend: event subscription established");

    // Publish a snapshot up front; a transient failure must not kill the
    // listener and leave the bar empty until the next event.
    match fetch_full_state().await {
        Ok(state) => {
            log::warn!(
                "Wayfire backend: initial state has {} monitors, {} workspaces, active ids {:?}",
                state.monitors.len(),
                state.workspaces.len(),
                state.active_workspace_ids
            );
            send_state(tx, &state);
        }
        Err(e) => log::warn!("Wayfire initial state refresh failed: {e}"),
    }

    // Read on a separate task so the coalesce timeout cannot desync the stream.
    let (events_tx, mut events_rx) = mpsc::channel::<()>(1);
    tokio::spawn(async move {
        loop {
            match read_message(&mut stream).await {
                Ok(msg) => {
                    if msg.get("event").is_some()
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
            Ok(state) => {
                log::warn!(
                    "Wayfire backend: refreshed state has {} monitors, {} workspaces, active ids {:?}",
                    state.monitors.len(),
                    state.workspaces.len(),
                    state.active_workspace_ids
                );
                send_state(tx, &state);
            }
            Err(e) => log::warn!("Wayfire state refresh failed: {e}"),
        }
    }

    Err(anyhow!("Wayfire event stream closed"))
}

async fn watch_events(stream: &mut UnixStream) -> Result<()> {
    const METHOD: &str = "window-rules/events/watch";

    // Wayfire rejects unknown event names, so fall back to watching all.
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
    let path = socket_path().ok_or_else(|| {
        anyhow!("Wayfire IPC socket not found (WAYFIRE_SOCKET unset, no wayfire-*.socket in XDG_RUNTIME_DIR)")
    })?;
    log::warn!(
        "Wayfire backend: connecting to IPC socket {}",
        path.display()
    );

    let std_stream = StdUnixStream::connect(&path)?;
    std_stream.set_nonblocking(true)?;
    UnixStream::from_std(std_stream).context("Failed to convert Wayfire socket")
}

static REQUEST_STREAM: OnceLock<AsyncMutex<Option<UnixStream>>> = OnceLock::new();

fn request_stream() -> &'static AsyncMutex<Option<UnixStream>> {
    REQUEST_STREAM.get_or_init(|| AsyncMutex::new(None))
}

/// One long-lived stream is reused for method calls; a failed call drops it.
async fn call_method(method: &str, data: &Value) -> Result<Value> {
    let mut guard = request_stream().lock().await;

    // A cancelled request can leave a partial message behind, so take the stream.
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

/// `get-focused-*` wrap the payload in `{"result": "ok", "info": …}`.
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
    log::warn!(
        "Wayfire backend: list-outputs returned {} outputs: {:?}",
        outputs.len(),
        outputs
            .iter()
            .map(|o| (o.name.clone(), o.workspace.x, o.workspace.y, o.grid()))
            .collect::<Vec<_>>()
    );

    let focused_output_id = focused_output()
        .await
        .inspect_err(|e| log::debug!("Wayfire focused output unavailable: {e}"))
        .map(|output| output.id)
        .ok();

    // Workspaces only need the output list; a bad view must not blank the bar.
    let views = match call_method("window-rules/list-views", &serde_json::json!({})).await {
        Ok(views_resp) => serde_json::from_value::<Vec<WfView>>(views_resp)
            .inspect_err(|e| log::debug!("Wayfire list-views parse failed: {e}"))
            .unwrap_or_default(),
        Err(e) => {
            log::debug!("Wayfire list-views unavailable: {e}");
            Vec::new()
        }
    };
    log::warn!("Wayfire backend: list-views returned {} views", views.len());

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

/// View geometry is output-local and relative to the current workspace, so the
/// cell follows from the view's centre point.
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

    // Wayfire has no per-workspace view list, so bucket views by grid cell.
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

            // Only the focused output's current cell counts as active.
            if x == current_x && y == current_y && Some(output.id) == focused_output_id {
                active_workspace_ids.push(id);
            }
        }

        let active_workspace_id =
            encode_workspace_id(output_index, current_y * grid_w + current_x).unwrap_or(-1);
        let active_workspace_name = if active_workspace_id >= 0 {
            (current_y * grid_w + current_x + 1).to_string()
        } else {
            String::new()
        };

        monitors.push(CompositorMonitor {
            id: output_index as i128,
            name: output.name.clone(),
            focused: Some(output.id) == focused_output_id,
            active_workspace_id,
            active_workspace_name,
            special_workspace_name: String::new(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn output(id: i64, name: &str, grid_w: i64, grid_h: i64, x: i64, y: i64) -> WfOutput {
        serde_json::from_value(json!({
            "id": id,
            "name": name,
            "geometry": { "x": 0, "y": 0, "width": 1920, "height": 1080 },
            "workspace": {
                "x": x,
                "y": y,
                "grid_width": grid_w,
                "grid_height": grid_h,
            },
        }))
        .expect("output fixture parses")
    }

    fn view(
        id: i64,
        output_id: i64,
        geom_x: i64,
        geom_y: i64,
        app_id: &str,
        title: &str,
    ) -> WfView {
        serde_json::from_value(json!({
            "id": id,
            "app-id": app_id,
            "title": title,
            "output-id": output_id,
            "geometry": { "x": geom_x, "y": geom_y, "width": 800, "height": 600 },
            "role": "toplevel",
            "mapped": true,
        }))
        .expect("view fixture parses")
    }

    #[test]
    fn grid_produces_one_workspace_per_cell() {
        let outputs = vec![output(1, "DP-1", 3, 2, 0, 0)];
        let state = build_state(&outputs, &[], Some(1), None, None);

        // 3x2 grid -> 6 workspaces numbered 1..=6 for the first output.
        let mut ids: Vec<i32> = state.workspaces.iter().map(|w| w.id).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec![1, 2, 3, 4, 5, 6]);
        assert!(state.workspaces.iter().all(|w| w.monitor == "DP-1"));
        assert_eq!(state.monitors.len(), 1);
        assert_eq!(state.monitors[0].name, "DP-1");
    }

    #[test]
    fn current_cell_is_active_and_others_visible() {
        let outputs = vec![output(1, "DP-1", 3, 3, 1, 1)];
        let state = build_state(&outputs, &[], Some(1), None, None);

        assert_eq!(state.active_workspace_ids, vec![5]); // (1*3 + 1) + 1
        let active: Vec<i32> = state
            .workspaces
            .iter()
            .filter(|w| state.active_workspace_ids.contains(&w.id))
            .map(|w| w.id)
            .collect();
        assert_eq!(active, vec![5]);
        assert_eq!(state.monitors[0].active_workspace_id, 5);
        assert_eq!(state.monitors[0].active_workspace_name, "5");
        assert!(state.monitors[0].focused);
    }

    #[test]
    fn view_buckets_into_its_ground_cell() {
        // 2x1 grid, 1920 wide: the view centre at 2000 lands in cell (1,0).
        let outputs = vec![output(1, "DP-1", 2, 1, 0, 0)];
        let views = vec![view(10, 1, 1600, 0, "firefox", "Mozilla Firefox")];
        let state = build_state(&outputs, &views, Some(1), None, None);

        let ws2 = state.workspaces.iter().find(|w| w.id == 2).unwrap();
        assert_eq!(ws2.windows, 1);
        let ws1 = state.workspaces.iter().find(|w| w.id == 1).unwrap();
        assert_eq!(ws1.windows, 0);
    }

    #[test]
    fn focused_view_becomes_active_window() {
        let outputs = vec![output(1, "DP-1", 1, 1, 0, 0)];
        let views = [view(10, 1, 0, 0, "firefox", "Mozilla Firefox")];
        let state = build_state(&outputs, &views, Some(1), views.first(), None);

        let active = state.active_window.expect("active window set");
        assert_eq!(active.title(), "Mozilla Firefox");
        assert_eq!(active.class(), "firefox");
    }

    #[test]
    fn empty_outputs_yield_no_workspaces() {
        let state = build_state(&[], &[], None, None, None);
        assert!(state.workspaces.is_empty());
        assert!(state.monitors.is_empty());
        assert!(state.active_window.is_none());
    }

    #[test]
    fn wayfire_float_geometry_parses() {
        // Verbatim `list-outputs` payload from Wayfire 0.11 (float geometry).
        let payload = json!([
            {
                "id": 1,
                "name": "eDP-1",
                "geometry": { "x": 0.0, "y": 0.0, "width": 2240.0, "height": 1400.0 },
                "workarea": { "x": 0.0, "y": 74.0, "width": 2240.0, "height": 1326.0 },
                "wset-index": 1,
                "workspace": { "x": 0, "y": 0, "grid_width": 3, "grid_height": 3 }
            },
            {
                "id": 3,
                "name": "DP-1",
                "geometry": { "x": 2240.0, "y": 0.0, "width": 3840.0, "height": 2160.0 },
                "workarea": { "x": 0.0, "y": 74.0, "width": 3840.0, "height": 2086.0 },
                "wset-index": 2,
                "workspace": { "x": 1, "y": 2, "grid_width": 3, "grid_height": 3 }
            }
        ]);

        let outputs: Vec<WfOutput> =
            serde_json::from_value(payload).expect("float geometry must parse");
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].name, "eDP-1");
        assert_eq!(outputs[0].geometry.height, 1400);
        assert_eq!(outputs[0].grid(), (3, 3));
        assert_eq!(outputs[1].id, 3);
        assert_eq!(outputs[1].geometry.x, 2240);

        let state = build_state(&outputs, &[], Some(3), None, None);
        assert_eq!(state.workspaces.len(), 18);
        assert_eq!(state.monitors.len(), 2);
        assert_eq!(state.active_workspace_ids, vec![108]);
    }

    #[test]
    fn view_with_float_geometry_and_unset_output_parses() {
        // Unmanaged views report `output-id` as -1 in unsigned form.
        let payload = json!({
            "id": 5,
            "title": "nil",
            "app-id": "nil",
            "output-id": 4294967295u32,
            "geometry": { "x": 0.0, "y": 0.0, "width": 0.0, "height": 0.0 },
            "role": "unmanaged",
            "mapped": false
        });

        let view: WfView = serde_json::from_value(payload).expect("view must parse");
        assert_eq!(view.output_id, Some(4294967295));
        assert_eq!(view.geometry.width, 0);
    }
}
