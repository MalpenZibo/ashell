//! Generic Wayland fallback backend. One connection binds `wl_output`
//! (monitors), `ext-workspace-v1` (workspaces) and
//! `wlr-foreign-toplevel-management` (active window); like the Hyprland and Niri
//! backends it rebuilds the full [`CompositorState`] on every change.

use super::types::{
    ActiveWindow, ActiveWindowGeneric, CompositorCommand, CompositorEvent, CompositorMonitor,
    CompositorService, CompositorState, CompositorWorkspace,
};
use crate::services::ServiceEvent;
use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tokio::sync::broadcast;
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle,
    backend::ObjectId,
    event_created_child,
    protocol::wl_output::{self, WlOutput},
    protocol::wl_registry::{self, WlRegistry},
};
use wayland_protocols::ext::workspace::v1::client::{
    ext_workspace_group_handle_v1::{self, ExtWorkspaceGroupHandleV1},
    ext_workspace_handle_v1::{self, ExtWorkspaceHandleV1},
    ext_workspace_manager_v1::{self, ExtWorkspaceManagerV1},
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::{self, ZwlrForeignToplevelHandleV1},
    zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
};

pub fn is_available() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
}

pub async fn run_listener(tx: &broadcast::Sender<ServiceEvent<CompositorService>>) -> Result<()> {
    let tx = tx.clone();
    tokio::task::spawn_blocking(move || event_loop(tx))
        .await
        .context("generic Wayland thread panicked")?
}

fn event_loop(tx: broadcast::Sender<ServiceEvent<CompositorService>>) -> Result<()> {
    let conn = Connection::connect_to_env().context("connect to Wayland")?;
    let mut queue = conn.new_event_queue();
    let qh = queue.handle();
    let _registry = conn.display().get_registry(&qh, ());

    let mut state = GenericState::default();

    // First roundtrip: discover and bind globals. Second roundtrip: receive the
    // initial burst of group/workspace/toplevel objects and their properties.
    queue.roundtrip(&mut state)?;
    queue.roundtrip(&mut state)?;

    state.handles_changed = false;
    state.publish_commands(&conn);
    let _ = tx.send(ServiceEvent::Update(CompositorEvent::StateChanged(
        Box::new(state.build_state()),
    )));

    loop {
        queue.blocking_dispatch(&mut state)?;
        if state.dirty {
            state.dirty = false;
            if state.handles_changed {
                state.handles_changed = false;
                state.publish_commands(&conn);
            }
            let _ = tx.send(ServiceEvent::Update(CompositorEvent::StateChanged(
                Box::new(state.build_state()),
            )));
        }
    }
}

pub async fn execute_command(cmd: CompositorCommand) -> Result<()> {
    match cmd {
        CompositorCommand::FocusWorkspace(id) => {
            activate_workspace(|handles| handles.iter().find(|h| h.numeric_id == id).cloned())
        }
        other => Err(anyhow!(
            "{other:?} is not supported on the generic Wayland backend"
        )),
    }
}

// `ext-workspace` activation needs the live proxies, which belong to the
// listener's connection, so the listener publishes them into this shared slot.
#[derive(Clone)]
struct WorkspaceHandle {
    numeric_id: i32,
    handle: ExtWorkspaceHandleV1,
}

struct CommandState {
    conn: Connection,
    manager: ExtWorkspaceManagerV1,
    handles: Vec<WorkspaceHandle>,
}

fn command_slot() -> &'static Mutex<Option<CommandState>> {
    static SLOT: OnceLock<Mutex<Option<CommandState>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

// `ext-workspace` stages requests, so `activate` is followed by `commit` + flush.
fn activate_workspace(
    pick: impl FnOnce(&[WorkspaceHandle]) -> Option<WorkspaceHandle>,
) -> Result<()> {
    let guard = command_slot().lock().unwrap();
    let cmd = guard
        .as_ref()
        .ok_or_else(|| anyhow!("generic workspace control is unavailable on this compositor"))?;
    let target = pick(&cmd.handles).ok_or_else(|| anyhow!("workspace not found"))?;
    target.handle.activate();
    cmd.manager.commit();
    cmd.conn
        .flush()
        .map_err(|e| anyhow!("Wayland flush failed: {e}"))?;
    Ok(())
}

#[derive(Default)]
struct OutputEntry {
    global_name: u32,
    name: String,
}

#[derive(Default)]
struct GroupEntry {
    outputs: Vec<ObjectId>,
    workspaces: Vec<ObjectId>,
}

#[derive(Default)]
struct WorkspaceEntry {
    numeric_id: i32,
    name: String,
    active: bool,
    urgent: bool,
    group: Option<ObjectId>,
}

#[derive(Default)]
struct ToplevelEntry {
    title: String,
    app_id: String,
    activated: bool,
}

#[derive(Default)]
struct GenericState {
    // Kept alive so their objects stay bound; the names let GlobalRemove drop them.
    workspace_manager: Option<ExtWorkspaceManagerV1>,
    workspace_manager_name: Option<u32>,
    toplevel_manager: Option<ZwlrForeignToplevelManagerV1>,
    toplevel_manager_name: Option<u32>,

    outputs: Vec<(ObjectId, OutputEntry)>,
    groups: HashMap<ObjectId, GroupEntry>,
    workspaces: HashMap<ObjectId, WorkspaceEntry>,
    workspace_handles: HashMap<ObjectId, ExtWorkspaceHandleV1>,
    workspace_order: Vec<ObjectId>,
    next_workspace_id: i32,
    toplevels: HashMap<ObjectId, ToplevelEntry>,
    toplevel_order: Vec<ObjectId>,

    dirty: bool,
    // Republish the command slot only when the handle set changes, not every event.
    handles_changed: bool,
}

impl GenericState {
    fn output_index(&self, id: &ObjectId) -> Option<usize> {
        self.outputs.iter().position(|(oid, _)| oid == id)
    }

    fn workspace_monitor(&self, ws: &WorkspaceEntry) -> (String, Option<i128>) {
        let Some(output_id) = ws
            .group
            .as_ref()
            .and_then(|g| self.groups.get(g))
            .and_then(|g| g.outputs.first())
        else {
            return (String::new(), None);
        };
        match self.output_index(output_id) {
            Some(i) => (self.outputs[i].1.name.clone(), Some(i as i128)),
            None => (String::new(), None),
        }
    }

    fn build_state(&self) -> CompositorState {
        let workspaces: Vec<CompositorWorkspace> = self
            .workspace_order
            .iter()
            .filter_map(|id| self.workspaces.get(id))
            .enumerate()
            .map(|(pos, ws)| {
                let (monitor, monitor_id) = self.workspace_monitor(ws);
                // ext-workspace has no inherent index. Prefer the workspace name
                // when it is a positive integer (the common 1..N labelling),
                // else fall back to display order so the value stays small and
                // dense instead of the ever-growing numeric_id.
                let index = ws
                    .name
                    .parse::<i32>()
                    .ok()
                    .filter(|n| *n > 0)
                    .unwrap_or(pos as i32 + 1);
                CompositorWorkspace {
                    id: ws.numeric_id,
                    index,
                    name: ws.name.clone(),
                    monitor,
                    monitor_id,
                    // ext-workspace exposes no per-workspace client count; report
                    // a non-zero value so workspaces render in the occupied solid
                    // style rather than all appearing empty.
                    windows: 1,
                    is_special: false,
                    has_urgent: ws.urgent,
                    window_classes: Vec::new(),
                }
            })
            .collect();

        let monitors: Vec<CompositorMonitor> = self
            .outputs
            .iter()
            .enumerate()
            .map(|(i, (oid, o))| {
                let active = self
                    .groups
                    .values()
                    .find(|g| g.outputs.contains(oid))
                    .and_then(|g| {
                        g.workspaces
                            .iter()
                            .filter_map(|w| self.workspaces.get(w))
                            .find(|w| w.active)
                    })
                    .map_or(-1, |w| w.numeric_id);
                CompositorMonitor {
                    id: i as i128,
                    name: o.name.clone(),
                    active_workspace_id: active,
                    special_workspace_id: -1,
                }
            })
            .collect();

        // ext-workspace marks one active workspace per group (per monitor) and
        // exposes no global focus, so on multi-monitor every per-monitor active
        // workspace is reported.
        let active_workspace_ids = self
            .workspace_order
            .iter()
            .filter_map(|id| self.workspaces.get(id))
            .filter(|ws| ws.active)
            .map(|ws| ws.numeric_id)
            .collect();

        // Iterate toplevels in creation order: several can be activated at once
        // (one per output on multi-monitor), so a HashMap scan would pick a
        // non-deterministic one that flips as the map rehashes.
        let active_window = self
            .toplevel_order
            .iter()
            .filter_map(|id| self.toplevels.get(id))
            .find(|t| t.activated)
            .map(|t| {
                ActiveWindow::Generic(ActiveWindowGeneric {
                    title: t.title.clone(),
                    class: t.app_id.clone(),
                })
            });

        CompositorState {
            workspaces,
            monitors,
            active_workspace_ids,
            active_window,
            keyboard_layout: String::new(),
            submap: None,
        }
    }

    fn publish_commands(&self, conn: &Connection) {
        let Some(manager) = &self.workspace_manager else {
            return;
        };
        let handles = self
            .workspace_order
            .iter()
            .filter_map(|id| {
                let ws = self.workspaces.get(id)?;
                let handle = self.workspace_handles.get(id)?;
                Some(WorkspaceHandle {
                    numeric_id: ws.numeric_id,
                    handle: handle.clone(),
                })
            })
            .collect();
        let mut slot = command_slot().lock().unwrap();
        match slot.as_mut() {
            Some(state) => state.handles = handles,
            None => {
                *slot = Some(CommandState {
                    conn: conn.clone(),
                    manager: manager.clone(),
                    handles,
                })
            }
        }
    }
}

impl Dispatch<WlRegistry, ()> for GenericState {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                if interface == WlOutput::interface().name {
                    let output: WlOutput = registry.bind(name, version.min(4), qh, ());
                    state.outputs.push((
                        output.id(),
                        OutputEntry {
                            global_name: name,
                            name: String::new(),
                        },
                    ));
                    state.dirty = true;
                } else if interface == ExtWorkspaceManagerV1::interface().name
                    && state.workspace_manager.is_none()
                {
                    state.workspace_manager = Some(registry.bind(name, version.min(1), qh, ()));
                    state.workspace_manager_name = Some(name);
                } else if interface == ZwlrForeignToplevelManagerV1::interface().name
                    && state.toplevel_manager.is_none()
                {
                    state.toplevel_manager = Some(registry.bind(name, version.min(3), qh, ()));
                    state.toplevel_manager_name = Some(name);
                }
            }
            wl_registry::Event::GlobalRemove { name } => {
                if let Some(idx) = state
                    .outputs
                    .iter()
                    .position(|(_, o)| o.global_name == name)
                {
                    state.outputs.remove(idx);
                    state.dirty = true;
                } else if state.workspace_manager_name == Some(name) {
                    // Manager gone: its workspaces/groups are inert, so drop them.
                    state.workspace_manager = None;
                    state.workspace_manager_name = None;
                    state.groups.clear();
                    state.workspaces.clear();
                    state.workspace_handles.clear();
                    state.workspace_order.clear();
                    *command_slot().lock().unwrap() = None;
                    state.dirty = true;
                } else if state.toplevel_manager_name == Some(name) {
                    state.toplevel_manager = None;
                    state.toplevel_manager_name = None;
                    state.toplevels.clear();
                    state.toplevel_order.clear();
                    state.dirty = true;
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<WlOutput, ()> for GenericState {
    fn event(
        state: &mut Self,
        output: &WlOutput,
        event: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_output::Event::Name { name } = event
            && let Some((_, entry)) = state.outputs.iter_mut().find(|(id, _)| *id == output.id())
        {
            entry.name = name;
            state.dirty = true;
        }
    }
}

impl Dispatch<ExtWorkspaceManagerV1, ()> for GenericState {
    fn event(
        state: &mut Self,
        _: &ExtWorkspaceManagerV1,
        event: ext_workspace_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_workspace_manager_v1::Event::WorkspaceGroup { workspace_group } => {
                state
                    .groups
                    .insert(workspace_group.id(), GroupEntry::default());
            }
            ext_workspace_manager_v1::Event::Workspace { workspace } => {
                let id = workspace.id();
                // Start at 1: the UI treats id <= 0 as a special workspace.
                state.next_workspace_id += 1;
                let numeric_id = state.next_workspace_id;
                state.workspaces.insert(
                    id.clone(),
                    WorkspaceEntry {
                        numeric_id,
                        ..WorkspaceEntry::default()
                    },
                );
                state.workspace_handles.insert(id.clone(), workspace);
                state.workspace_order.push(id);
                state.handles_changed = true;
            }
            ext_workspace_manager_v1::Event::Done => state.dirty = true,
            _ => {}
        }
    }

    event_created_child!(GenericState, ExtWorkspaceManagerV1, [
        ext_workspace_manager_v1::EVT_WORKSPACE_GROUP_OPCODE => (ExtWorkspaceGroupHandleV1, ()),
        ext_workspace_manager_v1::EVT_WORKSPACE_OPCODE => (ExtWorkspaceHandleV1, ()),
    ]);
}

impl Dispatch<ExtWorkspaceGroupHandleV1, ()> for GenericState {
    fn event(
        state: &mut Self,
        group: &ExtWorkspaceGroupHandleV1,
        event: ext_workspace_group_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let group_id = group.id();
        match event {
            ext_workspace_group_handle_v1::Event::OutputEnter { output } => {
                if let Some(g) = state.groups.get_mut(&group_id) {
                    g.outputs.push(output.id());
                    state.dirty = true;
                }
            }
            ext_workspace_group_handle_v1::Event::OutputLeave { output } => {
                if let Some(g) = state.groups.get_mut(&group_id) {
                    g.outputs.retain(|o| *o != output.id());
                    state.dirty = true;
                }
            }
            ext_workspace_group_handle_v1::Event::WorkspaceEnter { workspace } => {
                let ws_id = workspace.id();
                if let Some(g) = state.groups.get_mut(&group_id) {
                    g.workspaces.push(ws_id.clone());
                }
                if let Some(ws) = state.workspaces.get_mut(&ws_id) {
                    ws.group = Some(group_id);
                }
                state.dirty = true;
            }
            ext_workspace_group_handle_v1::Event::WorkspaceLeave { workspace } => {
                if let Some(g) = state.groups.get_mut(&group_id) {
                    g.workspaces.retain(|w| *w != workspace.id());
                    state.dirty = true;
                }
            }
            ext_workspace_group_handle_v1::Event::Removed => {
                state.groups.remove(&group_id);
                state.dirty = true;
                group.destroy();
            }
            _ => {}
        }
    }
}

impl Dispatch<ExtWorkspaceHandleV1, ()> for GenericState {
    fn event(
        state: &mut Self,
        handle: &ExtWorkspaceHandleV1,
        event: ext_workspace_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let id = handle.id();
        match event {
            ext_workspace_handle_v1::Event::Name { name } => {
                if let Some(ws) = state.workspaces.get_mut(&id) {
                    ws.name = name;
                    state.dirty = true;
                }
            }
            ext_workspace_handle_v1::Event::State { state: ws_state } => {
                if let Some(ws) = state.workspaces.get_mut(&id) {
                    let bits = match ws_state {
                        wayland_client::WEnum::Value(s) => s.bits(),
                        wayland_client::WEnum::Unknown(bits) => bits,
                    };
                    ws.active = bits & ext_workspace_handle_v1::State::Active.bits() != 0;
                    ws.urgent = bits & ext_workspace_handle_v1::State::Urgent.bits() != 0;
                    state.dirty = true;
                }
            }
            ext_workspace_handle_v1::Event::Removed => {
                state.workspaces.remove(&id);
                state.workspace_handles.remove(&id);
                state.workspace_order.retain(|w| *w != id);
                state.handles_changed = true;
                state.dirty = true;
                handle.destroy();
            }
            _ => {}
        }
    }
}

impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for GenericState {
    fn event(
        state: &mut Self,
        _: &ZwlrForeignToplevelManagerV1,
        event: zwlr_foreign_toplevel_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zwlr_foreign_toplevel_manager_v1::Event::Toplevel { toplevel } = event {
            let id = toplevel.id();
            state.toplevels.insert(id.clone(), ToplevelEntry::default());
            state.toplevel_order.push(id);
        }
    }

    event_created_child!(GenericState, ZwlrForeignToplevelManagerV1, [
        zwlr_foreign_toplevel_manager_v1::EVT_TOPLEVEL_OPCODE => (ZwlrForeignToplevelHandleV1, ()),
    ]);
}

impl Dispatch<ZwlrForeignToplevelHandleV1, ()> for GenericState {
    fn event(
        state: &mut Self,
        handle: &ZwlrForeignToplevelHandleV1,
        event: zwlr_foreign_toplevel_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let id = handle.id();
        match event {
            zwlr_foreign_toplevel_handle_v1::Event::Title { title } => {
                if let Some(t) = state.toplevels.get_mut(&id) {
                    t.title = title;
                }
            }
            zwlr_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                if let Some(t) = state.toplevels.get_mut(&id) {
                    t.app_id = app_id;
                }
            }
            zwlr_foreign_toplevel_handle_v1::Event::State { state: tl_state } => {
                if let Some(t) = state.toplevels.get_mut(&id) {
                    // The protocol sends active states as a wl_array of u32 entries
                    // (one per active state, not a bitmask), so we parse 4 bytes at a time.
                    let activated = zwlr_foreign_toplevel_handle_v1::State::Activated as u32;
                    t.activated = tl_state
                        .chunks_exact(4)
                        .filter_map(|c| c.try_into().ok())
                        .any(|c| u32::from_ne_bytes(c) == activated);
                }
            }
            zwlr_foreign_toplevel_handle_v1::Event::Done => state.dirty = true,
            zwlr_foreign_toplevel_handle_v1::Event::Closed => {
                state.toplevels.remove(&id);
                state.toplevel_order.retain(|t| *t != id);
                state.dirty = true;
                handle.destroy();
            }
            _ => {}
        }
    }
}
