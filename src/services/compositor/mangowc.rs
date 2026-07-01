//! MangoWC backend. Mango is a tag-based compositor controlled through its
//! `mmsg` IPC CLI (socket-based JSON protocol, mango >= 0.14). Tags map onto
//! ashell workspaces and several can be active at once (reported via
//! `CompositorState::active_workspace_ids`).

use super::types::{
    ActiveWindow, ActiveWindowMango, CompositorCommand, CompositorEvent, CompositorMonitor,
    CompositorService, CompositorState, CompositorWorkspace,
};
use crate::services::ServiceEvent;
use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use std::process::Command as StdCommand;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::broadcast,
};

#[derive(Deserialize)]
struct AllMonitors {
    #[serde(default)]
    monitors: Vec<Monitor>,
}

#[derive(Deserialize)]
struct Monitor {
    name: String,
    #[serde(default)]
    active: bool,
    #[serde(default)]
    tags: Vec<Tag>,
    #[serde(default)]
    active_tags: Vec<i32>,
    #[serde(default)]
    active_client: ActiveClient,
    #[serde(default)]
    keymode: String,
    #[serde(default)]
    keyboardlayout: String,
}

#[derive(Deserialize)]
struct Tag {
    index: i32,
    #[serde(default)]
    is_urgent: bool,
    #[serde(default)]
    client_count: u16,
}

#[derive(Deserialize, Default)]
struct ActiveClient {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    appid: Option<String>,
}

pub fn is_available() -> bool {
    // mmsg exits 0 even on failure, so success is decided by the payload: a valid
    // version object means we are talking to a live mango >= 0.14 instance.
    StdCommand::new("mmsg")
        .args(["get", "version"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| serde_json::from_slice::<serde_json::Value>(&out.stdout).ok())
        .is_some_and(|value| value.get("version").is_some())
}

pub async fn run_listener(tx: &broadcast::Sender<ServiceEvent<CompositorService>>) -> Result<()> {
    send_state(tx, &fetch_state().await?);

    let mut child = Command::new("mmsg")
        .args(["watch", "all-monitors"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .context("Failed to spawn mmsg watch process")?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("Failed to capture mmsg stdout"))?;
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();

    loop {
        line.clear();
        let read = reader.read_line(&mut line).await?;
        if read == 0 {
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // A malformed event must not kill the listener: log it and keep watching.
        match serde_json::from_str::<AllMonitors>(trimmed) {
            Ok(all) => send_state(tx, &build_state(all)),
            Err(e) => log::warn!("Failed to parse MangoWC watch event: {e}"),
        }
    }

    Err(anyhow!("mmsg watch stream exited"))
}

pub async fn execute_command(cmd: CompositorCommand) -> Result<()> {
    match cmd {
        CompositorCommand::FocusWorkspace(id) => focus_workspace(id).await,
        CompositorCommand::ScrollWorkspace(dir) => {
            let func = if dir > 0 {
                "viewtoleft_have_client"
            } else {
                "viewtoright_have_client"
            };
            dispatch(func).await
        }
        CompositorCommand::NextLayout => dispatch("switch_keyboard_layout").await,
        CompositorCommand::CustomDispatch(dispatcher, args) => {
            let func = if args.is_empty() {
                dispatcher
            } else {
                format!("{dispatcher},{args}")
            };
            dispatch(&func).await
        }
        other => Err(anyhow!("{other:?} is not supported on the MangoWC backend")),
    }
}

fn send_state(tx: &broadcast::Sender<ServiceEvent<CompositorService>>, state: &CompositorState) {
    let _ = tx.send(ServiceEvent::Update(CompositorEvent::StateChanged(
        Box::new(state.clone()),
    )));
}

async fn fetch_state() -> Result<CompositorState> {
    let raw = run_mmsg(["get", "all-monitors"]).await?;
    let all: AllMonitors =
        serde_json::from_str(&raw).context("Failed to parse MangoWC all-monitors output")?;
    Ok(build_state(all))
}

// Per-monitor id offset; keeps every (monitor, tag) pair unique while leaving
// monitor 0's ids equal to the tag index (single-monitor stays unchanged).
const TAG_STRIDE: i32 = 100;

fn encode_workspace_id(monitor_idx: usize, tag: i32) -> i32 {
    monitor_idx as i32 * TAG_STRIDE + tag
}

fn decode_workspace_id(id: i32) -> (usize, i32) {
    ((id / TAG_STRIDE) as usize, id % TAG_STRIDE)
}

fn build_state(all: AllMonitors) -> CompositorState {
    let mut workspaces = Vec::new();
    let mut active_workspace_ids = Vec::new();
    let mut monitors = Vec::new();
    let mut active_window = None;
    let mut keyboard_layout = None;
    let mut submap = None;

    // Mango tags are per-monitor, so each (monitor, tag) pair is its own
    // workspace; `monitor_id` drives its color as on the Niri/Hyprland backends.
    for (idx, monitor) in all.monitors.iter().enumerate() {
        for tag in &monitor.tags {
            workspaces.push(CompositorWorkspace {
                id: encode_workspace_id(idx, tag.index),
                index: tag.index,
                name: tag.index.to_string(),
                monitor: monitor.name.clone(),
                monitor_id: Some(idx as i128),
                windows: tag.client_count,
                is_special: false,
                has_urgent: tag.is_urgent,
            });
        }

        let mut selected_ids = monitor
            .active_tags
            .iter()
            .map(|tag| encode_workspace_id(idx, *tag))
            .collect::<Vec<_>>();
        selected_ids.sort_unstable();
        active_workspace_ids.extend(&selected_ids);

        monitors.push(CompositorMonitor {
            id: idx as i128,
            name: monitor.name.clone(),
            active_workspace_id: selected_ids.first().copied().unwrap_or(-1),
            special_workspace_id: -1,
        });

        // Prefer the focused monitor's data, else the first that has any.
        if (active_window.is_none() || monitor.active)
            && let Some(window) = monitor.active_client.to_active_window()
        {
            active_window = Some(window);
        }
        if (keyboard_layout.is_none() || monitor.active) && !monitor.keyboardlayout.is_empty() {
            keyboard_layout = Some(monitor.keyboardlayout.clone());
        }
        if (submap.is_none() || monitor.active) && !monitor.keymode.is_empty() {
            submap = (monitor.keymode != "default").then(|| monitor.keymode.clone());
        }
    }

    workspaces.sort_by_key(|w| w.id);

    active_workspace_ids.sort_unstable();
    active_workspace_ids.dedup();

    CompositorState {
        workspaces,
        monitors,
        active_workspace_ids,
        active_window,
        keyboard_layout: keyboard_layout.unwrap_or_else(|| "Unknown".to_string()),
        submap,
    }
}

impl ActiveClient {
    fn to_active_window(&self) -> Option<ActiveWindow> {
        let title = self.title.clone().unwrap_or_default();
        let class = self.appid.clone().unwrap_or_default();
        if title.is_empty() && class.is_empty() {
            None
        } else {
            Some(ActiveWindow::Mango(ActiveWindowMango { title, class }))
        }
    }
}

async fn focus_workspace(id: i32) -> Result<()> {
    let (monitor_idx, tag) = decode_workspace_id(id);

    // `view` acts on the selected monitor, so select the target one first.
    let monitors = fetch_monitor_names().await?;
    if monitors.len() > 1
        && let Some(name) = monitors.get(monitor_idx)
    {
        dispatch(&format!("focusmon,{name}")).await?;
    }

    dispatch(&format!("view,{tag}")).await
}

async fn fetch_monitor_names() -> Result<Vec<String>> {
    let raw = run_mmsg(["get", "all-monitors"]).await?;
    let all: AllMonitors =
        serde_json::from_str(&raw).context("Failed to parse MangoWC all-monitors output")?;
    Ok(all.monitors.into_iter().map(|m| m.name).collect())
}

async fn dispatch(func: &str) -> Result<()> {
    run_mmsg(["dispatch", func]).await.map(|_| ())
}

async fn run_mmsg<const N: usize>(args: [&str; N]) -> Result<String> {
    let output = Command::new("mmsg")
        .args(args)
        .output()
        .await
        .with_context(|| format!("Failed to run mmsg with args: {args:?}"))?;

    let stdout = String::from_utf8(output.stdout).context("mmsg output was not valid UTF-8")?;

    // mmsg exits 0 even on failure and reports errors as `{"error": ...}`.
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(stdout.trim())
        && let Some(error) = value.get("error").and_then(|e| e.as_str())
    {
        return Err(anyhow!("mmsg command failed ({args:?}): {error}"));
    }

    Ok(stdout)
}
