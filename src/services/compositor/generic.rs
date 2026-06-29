//! Generic Wayland backend: a separate Wayland connection on a dedicated
//! blocking thread binds `wl_output` (monitors), `ext-workspace-v1`
//! (workspaces) and `wlr-foreign-toplevel-management` (active window). Each
//! protocol is optional; an unadvertised one leaves its slice empty.

use super::backend::{Compositor, PatchSink};
use super::patch::StatePatch;
use super::types::{ActiveWindow, ActiveWindowGeneric, CompositorMonitor, CompositorWorkspace};
use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tokio::sync::mpsc;
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

/// Selects which protocols to bind, so a hybrid backend can reuse only some
/// generic capabilities (e.g. Sway: generic window/output, Sway-IPC workspaces).
#[derive(Debug, Clone, Copy)]
pub struct GenericCaps {
    pub outputs: bool,
    pub workspaces: bool,
    pub toplevels: bool,
}

impl GenericCaps {
    fn workspaces() -> Self {
        Self {
            outputs: true,
            workspaces: true,
            toplevels: false,
        }
    }

    fn window() -> Self {
        Self {
            outputs: false,
            workspaces: false,
            toplevels: true,
        }
    }

    fn topology(&self) -> bool {
        self.outputs || self.workspaces
    }
}

pub struct Generic;

impl Compositor for Generic {
    fn name(&self) -> &'static str {
        "generic Wayland"
    }

    async fn focus_workspace(&self, id: i32) -> Result<()> {
        activate_workspace(|handles| handles.iter().find(|h| h.numeric_id == id).cloned())
    }

    async fn scroll_workspace(&self, dir: i32) -> Result<()> {
        activate_workspace(|handles| {
            let active = handles.iter().position(|h| h.active).unwrap_or(0);
            let target = if dir > 0 {
                (active + 1).min(handles.len().saturating_sub(1))
            } else {
                active.saturating_sub(1)
            };
            handles.get(target).cloned()
        })
    }
}

/// A workspace handle exposed to the command path, kept in sync by the
/// `ext-workspace` listener so commands can `activate` without a second
/// connection.
#[derive(Clone)]
struct WorkspaceHandle {
    numeric_id: i32,
    active: bool,
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

/// Resolve a target workspace via `pick` and request its activation. The
/// `ext-workspace` protocol stages requests, so `activate` is followed by the
/// manager `commit` and a flush.
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

pub fn is_available() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
}

pub async fn workspaces(sink: PatchSink) -> Result<()> {
    run_with(sink, GenericCaps::workspaces()).await
}

pub async fn window(sink: PatchSink) -> Result<()> {
    run_with(sink, GenericCaps::window()).await
}

async fn run_with(patch_tx: PatchSink, caps: GenericCaps) -> Result<()> {
    tokio::task::spawn_blocking(move || event_loop(patch_tx, caps))
        .await
        .context("generic Wayland thread panicked")?
}

fn event_loop(patch_tx: mpsc::Sender<StatePatch>, caps: GenericCaps) -> Result<()> {
    let conn = Connection::connect_to_env().context("connect to Wayland")?;
    let mut queue = conn.new_event_queue();
    let qh = queue.handle();
    let _registry = conn.display().get_registry(&qh, ());

    let mut state = GenericState::new(patch_tx, caps);

    // First roundtrip: discover and bind globals. Second roundtrip: receive the
    // initial burst of group/workspace/toplevel objects and their properties.
    queue.roundtrip(&mut state)?;
    queue.roundtrip(&mut state)?;

    if state.emit_all().is_err() {
        return Ok(());
    }
    if caps.workspaces {
        state.publish_commands(&conn);
    }

    loop {
        queue.blocking_dispatch(&mut state)?;
        let topology_changed = state.topology_dirty;
        if state.emit_dirty().is_err() {
            // The merge loop dropped the receiver; nothing left to feed.
            return Ok(());
        }
        if topology_changed && caps.workspaces {
            state.publish_commands(&conn);
        }
    }
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

struct GenericState {
    patch_tx: mpsc::Sender<StatePatch>,
    caps: GenericCaps,

    // Kept alive so their objects stay bound and keep delivering events.
    workspace_manager: Option<ExtWorkspaceManagerV1>,
    _toplevel_manager: Option<ZwlrForeignToplevelManagerV1>,

    outputs: Vec<(ObjectId, OutputEntry)>,
    groups: HashMap<ObjectId, GroupEntry>,
    workspaces: HashMap<ObjectId, WorkspaceEntry>,
    workspace_handles: HashMap<ObjectId, ExtWorkspaceHandleV1>,
    workspace_order: Vec<ObjectId>,
    next_workspace_id: i32,
    toplevels: HashMap<ObjectId, ToplevelEntry>,

    topology_dirty: bool,
    window_dirty: bool,
}

impl GenericState {
    fn new(patch_tx: mpsc::Sender<StatePatch>, caps: GenericCaps) -> Self {
        Self {
            patch_tx,
            caps,
            workspace_manager: None,
            _toplevel_manager: None,
            outputs: Vec::new(),
            groups: HashMap::new(),
            workspaces: HashMap::new(),
            workspace_handles: HashMap::new(),
            workspace_order: Vec::new(),
            next_workspace_id: 1,
            toplevels: HashMap::new(),
            topology_dirty: false,
            window_dirty: false,
        }
    }

    fn output_index(&self, id: &ObjectId) -> Option<usize> {
        self.outputs.iter().position(|(oid, _)| oid == id)
    }

    /// The monitor (output index + name) a workspace lives on, via its group.
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

    fn build_topology(&self) -> StatePatch {
        let workspaces: Vec<CompositorWorkspace> = self
            .workspace_order
            .iter()
            .filter_map(|id| self.workspaces.get(id).map(|ws| (id, ws)))
            .map(|(_, ws)| {
                let (monitor, monitor_id) = self.workspace_monitor(ws);
                CompositorWorkspace {
                    id: ws.numeric_id,
                    index: ws.numeric_id,
                    name: ws.name.clone(),
                    monitor,
                    monitor_id,
                    windows: 0,
                    is_special: false,
                    has_urgent: ws.urgent,
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

        let active_workspace_id = self
            .workspace_order
            .iter()
            .filter_map(|id| self.workspaces.get(id))
            .find(|ws| ws.active)
            .map(|ws| ws.numeric_id);

        StatePatch::Topology {
            workspaces,
            monitors,
            active_workspace_id,
        }
    }

    fn build_active_window(&self) -> StatePatch {
        let window = self.toplevels.values().find(|t| t.activated).map(|t| {
            ActiveWindow::Generic(ActiveWindowGeneric {
                title: t.title.clone(),
                class: t.app_id.clone(),
            })
        });
        StatePatch::ActiveWindow(window)
    }

    fn emit_all(&mut self) -> Result<(), ()> {
        if self.caps.topology() {
            self.send(self.build_topology())?;
        }
        if self.caps.toplevels {
            self.send(self.build_active_window())?;
        }
        self.topology_dirty = false;
        self.window_dirty = false;
        Ok(())
    }

    fn emit_dirty(&mut self) -> Result<(), ()> {
        if self.topology_dirty {
            self.send(self.build_topology())?;
            self.topology_dirty = false;
        }
        if self.window_dirty {
            self.send(self.build_active_window())?;
            self.window_dirty = false;
        }
        Ok(())
    }

    fn send(&self, patch: StatePatch) -> Result<(), ()> {
        self.patch_tx.blocking_send(patch).map_err(|_| ())
    }

    /// Publish the current workspace handles so the command path can activate
    /// them. No-op until the `ext-workspace` manager is bound.
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
                    active: ws.active,
                    handle: handle.clone(),
                })
            })
            .collect();
        *command_slot().lock().unwrap() = Some(CommandState {
            conn: conn.clone(),
            manager: manager.clone(),
            handles,
        });
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
                if state.caps.outputs && interface == WlOutput::interface().name {
                    let output: WlOutput = registry.bind(name, version.min(4), qh, ());
                    state.outputs.push((
                        output.id(),
                        OutputEntry {
                            global_name: name,
                            name: String::new(),
                        },
                    ));
                    state.topology_dirty = true;
                } else if state.caps.workspaces
                    && interface == ExtWorkspaceManagerV1::interface().name
                    && state.workspace_manager.is_none()
                {
                    state.workspace_manager = Some(registry.bind(name, version.min(1), qh, ()));
                } else if state.caps.toplevels
                    && interface == ZwlrForeignToplevelManagerV1::interface().name
                    && state._toplevel_manager.is_none()
                {
                    state._toplevel_manager = Some(registry.bind(name, version.min(3), qh, ()));
                }
            }
            wl_registry::Event::GlobalRemove { name } => {
                if let Some(idx) = state
                    .outputs
                    .iter()
                    .position(|(_, o)| o.global_name == name)
                {
                    state.outputs.remove(idx);
                    state.topology_dirty = true;
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
            state.topology_dirty = true;
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
                let numeric_id = state.next_workspace_id;
                state.next_workspace_id += 1;
                state.workspaces.insert(
                    id.clone(),
                    WorkspaceEntry {
                        numeric_id,
                        ..WorkspaceEntry::default()
                    },
                );
                state.workspace_handles.insert(id.clone(), workspace);
                state.workspace_order.push(id);
            }
            ext_workspace_manager_v1::Event::Done => state.topology_dirty = true,
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
                    state.topology_dirty = true;
                }
            }
            ext_workspace_group_handle_v1::Event::OutputLeave { output } => {
                if let Some(g) = state.groups.get_mut(&group_id) {
                    g.outputs.retain(|o| *o != output.id());
                    state.topology_dirty = true;
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
                state.topology_dirty = true;
            }
            ext_workspace_group_handle_v1::Event::WorkspaceLeave { workspace } => {
                if let Some(g) = state.groups.get_mut(&group_id) {
                    g.workspaces.retain(|w| *w != workspace.id());
                    state.topology_dirty = true;
                }
            }
            ext_workspace_group_handle_v1::Event::Removed => {
                state.groups.remove(&group_id);
                state.topology_dirty = true;
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
                    state.topology_dirty = true;
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
                    state.topology_dirty = true;
                }
            }
            ext_workspace_handle_v1::Event::Removed => {
                state.workspaces.remove(&id);
                state.workspace_handles.remove(&id);
                state.workspace_order.retain(|w| *w != id);
                state.topology_dirty = true;
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
            state
                .toplevels
                .insert(toplevel.id(), ToplevelEntry::default());
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
                    let activated = zwlr_foreign_toplevel_handle_v1::State::Activated as u32;
                    t.activated = tl_state
                        .chunks_exact(4)
                        .filter_map(|c| c.try_into().ok())
                        .any(|c| u32::from_ne_bytes(c) == activated);
                }
            }
            zwlr_foreign_toplevel_handle_v1::Event::Done => state.window_dirty = true,
            zwlr_foreign_toplevel_handle_v1::Event::Closed => {
                state.toplevels.remove(&id);
                state.window_dirty = true;
                handle.destroy();
            }
            _ => {}
        }
    }
}
