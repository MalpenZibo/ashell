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

// dwl-ipc tag state bitfield (mmsg prints it as the third field of a `tag` line).
const TAG_ACTIVE: u32 = 1 << 0;
const TAG_URGENT: u32 = 1 << 1;

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
            // A transient mmsg failure must not kill the listener: log it and
            // keep watching so updates resume once mmsg recovers.
            if let Err(e) = send_latest_state(tx).await {
                log::warn!("Failed to refresh MangoWC state: {e}");
            }
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
    urgent: bool,
}

#[derive(Default)]
struct OutputWindow {
    name: String,
    title: String,
    appid: String,
    focused: bool,
}

async fn send_latest_state(tx: &broadcast::Sender<ServiceEvent<CompositorService>>) -> Result<()> {
    let state = fetch_full_state().await?;
    let _ = tx.send(ServiceEvent::Update(CompositorEvent::StateChanged(
        Box::new(state),
    )));
    Ok(())
}

async fn fetch_full_state() -> Result<CompositorState> {
    // The two queries are independent, so run them concurrently.
    let (main_raw, outputs_raw) = tokio::try_join!(
        run_mmsg(["-g", "-t", "-c", "-k", "-b"]),
        run_mmsg(["-g", "-O"])
    )?;

    let (tag_state, fallback_outputs) = parse_tags(&main_raw);
    let mut outputs = parse_outputs(&outputs_raw);
    for output in fallback_outputs {
        if !outputs.contains(&output) {
            outputs.push(output);
        }
    }

    let mut active_workspace_ids = Vec::new();
    let mut monitors = Vec::new();
    // Mango tags share a single id space across outputs, but the workspaces UI
    // keys on a unique id; aggregate per tag id so multi-monitor tags are merged
    // rather than silently dropped.
    let mut tags: HashMap<i32, CompositorWorkspace> = HashMap::new();

    for (idx, output_name) in outputs.iter().enumerate() {
        let output_tags = tag_state.get(output_name);

        let mut selected_ids = output_tags
            .map(|tags| {
                tags.iter()
                    .filter_map(|(id, info)| info.selected.then_some(*id))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        selected_ids.sort_unstable();
        active_workspace_ids.extend(&selected_ids);

        if let Some(output_tags) = output_tags {
            for (tag_id, info) in output_tags {
                let workspace = tags.entry(*tag_id).or_insert_with(|| CompositorWorkspace {
                    id: *tag_id,
                    index: *tag_id,
                    name: tag_id.to_string(),
                    monitor: String::new(),
                    monitor_id: None,
                    windows: 0,
                    is_special: false,
                    has_urgent: false,
                });
                workspace.windows += info.clients;
                workspace.has_urgent |= info.urgent;
            }
        }

        monitors.push(CompositorMonitor {
            id: idx as i128,
            name: output_name.clone(),
            active_workspace_id: selected_ids.first().copied().unwrap_or(-1),
            special_workspace_id: -1,
        });
    }

    let mut workspaces = tags.into_values().collect::<Vec<_>>();
    workspaces.sort_by_key(|w| w.id);

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
    // title/appid are reported per output. Prefer the focused monitor when it
    // reports `selmon`, otherwise fall back to the first output with a window:
    // deterministic, and never blanked by a later output's empty title line.
    let mut outputs: Vec<OutputWindow> = Vec::new();

    for line in raw.lines() {
        let mut parts = line.split_whitespace();
        let (Some(output), Some(key)) = (parts.next(), parts.next()) else {
            continue;
        };
        let value = parts.collect::<Vec<_>>().join(" ");

        let entry = match outputs.iter_mut().find(|o| o.name == output) {
            Some(entry) => entry,
            None => {
                outputs.push(OutputWindow {
                    name: output.to_string(),
                    ..Default::default()
                });
                outputs.last_mut().expect("just pushed")
            }
        };

        match key {
            "title" => entry.title = value,
            "appid" => entry.appid = value,
            "selmon" => entry.focused = value == "1",
            _ => {}
        }
    }

    let window = outputs.iter().find(|o| o.focused).or_else(|| {
        outputs
            .iter()
            .find(|o| !o.title.is_empty() || !o.appid.is_empty())
    })?;

    if window.title.is_empty() && window.appid.is_empty() {
        None
    } else {
        Some(ActiveWindow::Mango(ActiveWindowMango {
            title: window.title.clone(),
            class: window.appid.clone(),
        }))
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

fn parse_tags(raw: &str) -> (HashMap<String, HashMap<i32, TagInfo>>, Vec<String>) {
    let mut output_states: HashMap<String, HashMap<i32, TagInfo>> = HashMap::new();
    let mut outputs = Vec::new();

    for line in raw.lines() {
        let mut parts = line.split_whitespace();
        let (Some(output), Some(key)) = (parts.next(), parts.next()) else {
            continue;
        };

        if key != "tag" {
            continue;
        }

        // `tag <id> <state> <clients> <focused>`; state is the TAG_* bitfield.
        let fields = parts.collect::<Vec<_>>();
        if fields.len() < 3 {
            continue;
        }

        let tag_id = fields[0].parse::<i32>().unwrap_or(0);
        if tag_id <= 0 {
            continue;
        }
        let state = fields[1].parse::<u32>().unwrap_or(0);
        let clients = fields[2].parse::<u16>().unwrap_or(0);

        if !outputs.iter().any(|o| o == output) {
            outputs.push(output.to_string());
        }

        output_states.entry(output.to_string()).or_default().insert(
            tag_id,
            TagInfo {
                clients,
                selected: state & TAG_ACTIVE != 0,
                urgent: state & TAG_URGENT != 0,
            },
        );
    }

    (output_states, outputs)
}
