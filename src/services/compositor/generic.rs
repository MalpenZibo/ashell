//! Generic Wayland backend.
//!
//! Baseline for compositors without a dedicated implementation. Opens its own
//! Wayland connection (separate from the one iced uses for rendering) on a
//! dedicated blocking thread and binds the standard protocols that most
//! wlroots-style compositors expose:
//!
//! - `wl_output` (v4 `name`)            -> monitors
//! - `ext-workspace-v1`                 -> workspaces + active workspace
//! - `wlr-foreign-toplevel-management`  -> active window title/class
//!
//! Each protocol is optional: a compositor that does not advertise it simply
//! leaves the corresponding slice empty, which is the intended "capability not
//! available" behaviour. Keyboard layout and submap have no standard Wayland
//! protocol and are therefore never populated here.

use super::patch::StatePatch;
use super::types::{ActiveWindow, ActiveWindowGeneric, CompositorMonitor, CompositorWorkspace};
use anyhow::{Context, Result};
use std::collections::HashMap;
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

/// Selects which generic capabilities to bind. Lets a future hybrid backend
/// reuse the generic Wayland source while overriding a slice with a
/// compositor-specific one (e.g. Sway: keep generic window/output, replace
/// workspaces with the Sway IPC).
#[derive(Debug, Clone, Copy)]
pub struct GenericCaps {
    pub outputs: bool,
    pub workspaces: bool,
    pub toplevels: bool,
}

impl GenericCaps {
    pub fn all() -> Self {
        Self {
            outputs: true,
            workspaces: true,
            toplevels: true,
        }
    }
}

/// The generic backend is viable whenever a Wayland display is reachable.
pub fn is_available() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
}

/// Run the generic Wayland source with the full set of capabilities.
pub async fn run(patch_tx: mpsc::Sender<StatePatch>) -> Result<()> {
    run_with(patch_tx, GenericCaps::all()).await
}

/// Run the generic Wayland source binding only the selected capabilities.
pub async fn run_with(patch_tx: mpsc::Sender<StatePatch>, caps: GenericCaps) -> Result<()> {
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

    loop {
        queue.blocking_dispatch(&mut state)?;
        if state.emit_dirty().is_err() {
            // The merge loop dropped the receiver; nothing left to feed.
            return Ok(());
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
    _workspace_manager: Option<ExtWorkspaceManagerV1>,
    _toplevel_manager: Option<ZwlrForeignToplevelManagerV1>,

    outputs: Vec<(ObjectId, OutputEntry)>,
    groups: HashMap<ObjectId, GroupEntry>,
    workspaces: HashMap<ObjectId, WorkspaceEntry>,
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
            _workspace_manager: None,
            _toplevel_manager: None,
            outputs: Vec::new(),
            groups: HashMap::new(),
            workspaces: HashMap::new(),
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
        self.send(self.build_topology())?;
        self.send(self.build_active_window())?;
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
                    && state._workspace_manager.is_none()
                {
                    state._workspace_manager = Some(registry.bind(name, version.min(1), qh, ()));
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
