//! MangoWC backend. Mango is a tag-based compositor controlled through its
//! `mmsg` IPC CLI; tags map onto ashell workspaces and several can be active at
//! once (reported via `CompositorState::active_workspace_ids`).

use super::types::{
    ActiveWindow, ActiveWindowMango, CompositorCommand, CompositorEvent, CompositorMonitor,
    CompositorService, CompositorState, CompositorWorkspace,
};
use crate::services::ServiceEvent;
use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
use std::process::Command as StdCommand;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::broadcast,
};

pub fn is_available() -> bool {
    StdCommand::new("mmsg")
        .args(["-g", "-O"])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

pub async fn run_listener(tx: &broadcast::Sender<ServiceEvent<CompositorService>>) -> Result<()> {
    send_latest_state(tx).await?;

    let mut child = Command::new("mmsg")
        .args(["-w", "-t", "-c", "-k", "-b"])
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

        if !line.trim().is_empty() {
            send_latest_state(tx).await?;
        }
    }

    Err(anyhow!("mmsg watch stream exited"))
}

pub async fn execute_command(cmd: CompositorCommand) -> Result<()> {
    match cmd {
        CompositorCommand::FocusWorkspace(id) => {
            run_mmsg(["-s", "-t", &id.to_string()]).await.map(|_| ())
        }
        CompositorCommand::ScrollWorkspace(dir) => {
            let dispatch = if dir > 0 {
                "viewtoleft_have_client"
            } else {
                "viewtoright_have_client"
            };
            run_mmsg(["-s", "-d", dispatch]).await.map(|_| ())
        }
        CompositorCommand::NextLayout => run_mmsg(["-s", "-d", "switch_keyboard_layout"])
            .await
            .map(|_| ()),
        CompositorCommand::CustomDispatch(dispatcher, args) => {
            let full_dispatch = if args.is_empty() {
                dispatcher
            } else {
                format!("{dispatcher},{args}")
            };
            run_mmsg(["-s", "-d", &full_dispatch]).await.map(|_| ())
        }
        other => Err(anyhow!("{other:?} is not supported on the MangoWC backend")),
    }
}

#[derive(Debug, Default, Clone)]
struct TagInfo {
    clients: u16,
    selected: bool,
}

#[derive(Debug, Default, Clone)]
struct OutputTagState {
    tags: HashMap<i32, TagInfo>,
    selected_mask: u32,
    urgent_mask: u32,
}

async fn send_latest_state(tx: &broadcast::Sender<ServiceEvent<CompositorService>>) -> Result<()> {
    let state = fetch_full_state().await?;
    let _ = tx.send(ServiceEvent::Update(CompositorEvent::StateChanged(
        Box::new(state),
    )));
    Ok(())
}

async fn fetch_full_state() -> Result<CompositorState> {
    let main_raw = run_mmsg(["-g", "-t", "-c", "-k", "-b"]).await?;
    let outputs_raw = run_mmsg(["-g", "-O"]).await?;

    let (tag_state, fallback_outputs) = parse_tags(&main_raw);
    let mut outputs = parse_outputs(&outputs_raw);
    for output in fallback_outputs {
        if !outputs.contains(&output) {
            outputs.push(output);
        }
    }

    let mut active_workspace_ids = Vec::new();
    let mut monitors = Vec::new();
    let mut workspaces = Vec::new();

    for (idx, output_name) in outputs.iter().enumerate() {
        let state = tag_state.get(output_name).cloned().unwrap_or_default();
        let selected_ids = resolve_selected_tag_ids(&state);
        for id in &selected_ids {
            if !active_workspace_ids.contains(id) {
                active_workspace_ids.push(*id);
            }
        }

        let urgent_mask = state.urgent_mask;
        for (tag_id, info) in state.tags {
            workspaces.push(CompositorWorkspace {
                id: tag_id,
                index: tag_id,
                name: tag_id.to_string(),
                monitor: output_name.clone(),
                monitor_id: Some(idx as i128),
                windows: info.clients,
                is_special: false,
                has_urgent: tag_in_mask(urgent_mask, tag_id),
            });
        }

        monitors.push(CompositorMonitor {
            id: idx as i128,
            name: output_name.clone(),
            active_workspace_id: selected_ids.first().copied().unwrap_or(-1),
            special_workspace_id: -1,
        });
    }

    workspaces.sort_by(|a, b| {
        a.monitor
            .cmp(&b.monitor)
            .then(a.index.cmp(&b.index))
            .then(a.id.cmp(&b.id))
    });

    let active_window = parse_active_window(&main_raw);
    let keyboard_layout = parse_keyboard_layout(&main_raw);
    let submap = parse_keymode(&main_raw);

    active_workspace_ids.sort_unstable();
    active_workspace_ids.dedup();

    Ok(CompositorState {
        workspaces,
        monitors,
        active_workspace_ids,
        active_window,
        keyboard_layout,
        submap,
    })
}

async fn run_mmsg<const N: usize>(args: [&str; N]) -> Result<String> {
    let output = Command::new("mmsg")
        .args(args)
        .output()
        .await
        .with_context(|| format!("Failed to run mmsg with args: {args:?}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(anyhow!("mmsg command failed ({args:?}): {stderr}"));
    }

    String::from_utf8(output.stdout).context("mmsg output was not valid UTF-8")
}

fn parse_outputs(raw: &str) -> Vec<String> {
    raw.lines()
        .filter_map(|line| {
            let stripped = line.trim().strip_prefix('+')?;
            Some(stripped.trim().to_string())
        })
        .collect()
}

fn parse_active_window(raw: &str) -> Option<ActiveWindow> {
    let mut title = String::new();
    let mut class = String::new();

    for line in raw.lines() {
        let mut parts = line.split_whitespace();
        let _output = parts.next().unwrap_or_default();
        let key = parts.next().unwrap_or_default();
        let value = parts.collect::<Vec<_>>().join(" ");

        match key {
            "title" => title = value,
            "appid" => class = value,
            _ => {}
        }
    }

    if title.is_empty() && class.is_empty() {
        None
    } else {
        Some(ActiveWindow::Mango(ActiveWindowMango { title, class }))
    }
}

fn parse_keyboard_layout(raw: &str) -> String {
    for line in raw.lines() {
        let mut parts = line.split_whitespace();
        let _output = parts.next();
        let key = parts.next().unwrap_or_default();
        if key == "kb_layout" {
            let value = parts.collect::<Vec<_>>().join(" ");
            if !value.is_empty() {
                return value;
            }
        }
    }
    "Unknown".to_string()
}

fn parse_keymode(raw: &str) -> Option<String> {
    for line in raw.lines() {
        let mut parts = line.split_whitespace();
        let _output = parts.next();
        let key = parts.next().unwrap_or_default();
        if key == "keymode" {
            let value = parts.collect::<Vec<_>>().join(" ");
            if !value.is_empty() && value != "default" {
                return Some(value);
            }
        }
    }
    None
}

fn parse_tags(raw: &str) -> (HashMap<String, OutputTagState>, Vec<String>) {
    let mut output_states: HashMap<String, OutputTagState> = HashMap::new();
    let mut outputs = Vec::new();

    for line in raw.lines() {
        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 2 {
            continue;
        }

        let output = parts[0].to_string();
        if !outputs.contains(&output) {
            outputs.push(output.clone());
        }

        let state = output_states.entry(output).or_default();

        if parts[1] == "tag" && parts.len() >= 6 {
            let tag_id = parts[2].parse::<i32>().unwrap_or(0);
            if tag_id <= 0 {
                continue;
            }

            let selected = parts[3].parse::<u8>().unwrap_or(0) != 0;
            let clients = parts[4].parse::<u16>().unwrap_or(0);

            state.tags.insert(tag_id, TagInfo { clients, selected });
        }

        if parts[1] == "tags" && parts.len() >= 5 {
            state.selected_mask = parse_mask(parts[2]).unwrap_or(state.selected_mask);
            state.urgent_mask = parse_mask(parts[4]).unwrap_or(state.urgent_mask);
        }
    }

    (output_states, outputs)
}

fn resolve_selected_tag_ids(state: &OutputTagState) -> Vec<i32> {
    // Per-tag flags are more reliable than the bitmask, whose field order varies.
    let mut selected = state
        .tags
        .iter()
        .filter_map(|(id, info)| info.selected.then_some(*id))
        .collect::<Vec<_>>();

    if selected.is_empty() {
        selected = mask_to_tag_ids(state.selected_mask);
    }

    selected.sort_unstable();
    selected.dedup();
    selected
}

fn mask_to_tag_ids(mask: u32) -> Vec<i32> {
    (0..32)
        .filter(|idx| (mask & (1u32 << idx)) != 0)
        .map(|idx| idx + 1)
        .collect()
}

fn tag_in_mask(mask: u32, tag_id: i32) -> bool {
    (1..=32).contains(&tag_id) && (mask & (1 << (tag_id - 1))) != 0
}

fn parse_mask(value: &str) -> Option<u32> {
    if value.chars().all(|c| c == '0' || c == '1') {
        u32::from_str_radix(value, 2)
            .ok()
            .or_else(|| value.parse::<u32>().ok())
    } else {
        value.parse::<u32>().ok()
    }
}
