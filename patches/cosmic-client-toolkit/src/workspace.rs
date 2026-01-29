use cosmic_protocols::workspace::v2::client::{
    zcosmic_workspace_handle_v2, zcosmic_workspace_manager_v2,
};
use sctk::registry::{GlobalProxy, RegistryState};
use std::collections::HashSet;
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum, protocol::wl_output};
use wayland_protocols::ext::workspace::v1::client::{
    ext_workspace_group_handle_v1, ext_workspace_handle_v1, ext_workspace_manager_v1,
};

use crate::GlobalData;

#[derive(Clone, Debug)]
pub struct WorkspaceGroup {
    pub handle: ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1,
    pub capabilities: ext_workspace_group_handle_v1::GroupCapabilities,
    pub outputs: Vec<wl_output::WlOutput>,
    pub workspaces: HashSet<ext_workspace_handle_v1::ExtWorkspaceHandleV1>,
}

#[derive(Debug)]
struct WorkspaceGroupData {
    handle: ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1,
    current: Option<WorkspaceGroup>,
    pending: Option<WorkspaceGroup>,
}

impl WorkspaceGroupData {
    fn pending(&mut self) -> &mut WorkspaceGroup {
        if self.pending.is_none() {
            self.pending = Some(self.current.clone().unwrap_or(WorkspaceGroup {
                handle: self.handle.clone(),
                capabilities: ext_workspace_group_handle_v1::GroupCapabilities::empty(),
                outputs: Vec::new(),
                workspaces: HashSet::new(),
            }));
        }
        self.pending.as_mut().unwrap()
    }

    fn commit_pending(&mut self) {
        if let Some(pending) = self.pending.take() {
            self.current = Some(pending);
        }
    }
}

#[derive(Clone, Debug)]
pub struct Workspace {
    pub handle: ext_workspace_handle_v1::ExtWorkspaceHandleV1,
    pub cosmic_handle: Option<zcosmic_workspace_handle_v2::ZcosmicWorkspaceHandleV2>,
    pub name: String,
    pub coordinates: Vec<u32>,
    pub state: ext_workspace_handle_v1::State,
    pub cosmic_state: zcosmic_workspace_handle_v2::State,
    pub capabilities: ext_workspace_handle_v1::WorkspaceCapabilities,
    pub cosmic_capabilities: zcosmic_workspace_handle_v2::WorkspaceCapabilities,
    pub tiling: Option<WEnum<zcosmic_workspace_handle_v2::TilingState>>,
    pub id: Option<String>,
}

#[derive(Debug)]
struct WorkspaceData {
    handle: ext_workspace_handle_v1::ExtWorkspaceHandleV1,
    cosmic_handle: Option<zcosmic_workspace_handle_v2::ZcosmicWorkspaceHandleV2>,
    current: Option<Workspace>,
    pending: Option<Workspace>,
    has_cosmic_info: bool,
}

impl WorkspaceData {
    fn pending(&mut self) -> &mut Workspace {
        if self.pending.is_none() {
            self.pending = Some(self.current.clone().unwrap_or(Workspace {
                handle: self.handle.clone(),
                cosmic_handle: self.cosmic_handle.clone(),
                name: String::new(),
                coordinates: Vec::new(),
                state: ext_workspace_handle_v1::State::empty(),
                cosmic_state: zcosmic_workspace_handle_v2::State::empty(),
                capabilities: ext_workspace_handle_v1::WorkspaceCapabilities::empty(),
                cosmic_capabilities: zcosmic_workspace_handle_v2::WorkspaceCapabilities::empty(),
                tiling: None,
                id: None,
            }));
        }
        self.pending.as_mut().unwrap()
    }

    fn commit_pending(&mut self) {
        if let Some(pending) = self.pending.take() {
            self.current = Some(pending);
        }
    }
}

#[derive(Debug)]
pub struct WorkspaceState {
    workspace_groups: Vec<WorkspaceGroupData>,
    workspaces: Vec<WorkspaceData>,
    manager: GlobalProxy<ext_workspace_manager_v1::ExtWorkspaceManagerV1>,
    cosmic_manager: GlobalProxy<zcosmic_workspace_manager_v2::ZcosmicWorkspaceManagerV2>,
}

impl WorkspaceState {
    pub fn new<D>(registry: &RegistryState, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<ext_workspace_manager_v1::ExtWorkspaceManagerV1, GlobalData>
            + Dispatch<zcosmic_workspace_manager_v2::ZcosmicWorkspaceManagerV2, GlobalData>
            + 'static,
    {
        Self {
            workspace_groups: Vec::new(),
            workspaces: Vec::new(),
            manager: GlobalProxy::from(registry.bind_one(qh, 1..=1, GlobalData)),
            cosmic_manager: GlobalProxy::from(registry.bind_one(qh, 1..=2, GlobalData)),
        }
    }

    pub fn workspace_manager(
        &self,
    ) -> &GlobalProxy<ext_workspace_manager_v1::ExtWorkspaceManagerV1> {
        &self.manager
    }

    pub fn workspace_groups(&self) -> impl Iterator<Item = &WorkspaceGroup> {
        self.workspace_groups
            .iter()
            .filter_map(|data| data.current.as_ref())
    }

    pub fn workspace_group_info(
        &self,
        handle: &ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1,
    ) -> Option<&WorkspaceGroup> {
        self.workspace_groups
            .iter()
            .find(|g| g.handle == *handle)?
            .current
            .as_ref()
    }

    pub fn workspaces(&self) -> impl Iterator<Item = &Workspace> {
        self.workspaces
            .iter()
            .filter_map(|data| data.current.as_ref())
    }

    pub fn workspace_info(
        &self,
        handle: &ext_workspace_handle_v1::ExtWorkspaceHandleV1,
    ) -> Option<&Workspace> {
        self.workspaces
            .iter()
            .find(|g| g.handle == *handle)?
            .current
            .as_ref()
    }
}

pub trait WorkspaceHandler {
    fn workspace_state(&mut self) -> &mut WorkspaceState;

    // TODO: Added/remove/update methods? How to do that with groups and workspaces?
    fn done(&mut self);
}

impl<D> Dispatch<ext_workspace_manager_v1::ExtWorkspaceManagerV1, GlobalData, D> for WorkspaceState
where
    D: Dispatch<ext_workspace_manager_v1::ExtWorkspaceManagerV1, GlobalData>
        + Dispatch<ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1, GlobalData>
        + Dispatch<ext_workspace_handle_v1::ExtWorkspaceHandleV1, GlobalData>
        + Dispatch<zcosmic_workspace_handle_v2::ZcosmicWorkspaceHandleV2, GlobalData>
        + WorkspaceHandler
        + 'static,
{
    fn event(
        state: &mut D,
        _: &ext_workspace_manager_v1::ExtWorkspaceManagerV1,
        event: ext_workspace_manager_v1::Event,
        _: &GlobalData,
        _: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            ext_workspace_manager_v1::Event::WorkspaceGroup { workspace_group } => {
                state
                    .workspace_state()
                    .workspace_groups
                    .push(WorkspaceGroupData {
                        handle: workspace_group,
                        current: None,
                        pending: None,
                    });
            }
            ext_workspace_manager_v1::Event::Workspace { workspace } => {
                let cosmic_handle =
                    state
                        .workspace_state()
                        .cosmic_manager
                        .get()
                        .ok()
                        .map(|cosmic_manager| {
                            cosmic_manager.get_cosmic_workspace(&workspace, qh, GlobalData)
                        });
                state.workspace_state().workspaces.push(WorkspaceData {
                    handle: workspace,
                    cosmic_handle,
                    current: None,
                    pending: None,
                    has_cosmic_info: false,
                });
            }
            ext_workspace_manager_v1::Event::Done => {
                // If any workspace doesn't have cosmic info yet, we should wait for the
                // server to send, it instead of providing incomplete data.
                // Ignore this `done`, and wait for the one sent after the cosmic info.
                if state.workspace_state().cosmic_manager.get().is_ok()
                    && state
                        .workspace_state()
                        .workspaces
                        .iter()
                        .any(|w| !w.has_cosmic_info)
                {
                    return;
                }
                for data in &mut state.workspace_state().workspace_groups {
                    data.commit_pending();
                }
                for data in &mut state.workspace_state().workspaces {
                    data.commit_pending();
                }
                state.done();
            }
            ext_workspace_manager_v1::Event::Finished => {}
            _ => unreachable!(),
        }
    }

    wayland_client::event_created_child!(D, ext_workspace_manager_v1::ExtWorkspaceManagerV1, [
        ext_workspace_manager_v1::EVT_WORKSPACE_GROUP_OPCODE => (ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1, GlobalData),
        ext_workspace_manager_v1::EVT_WORKSPACE_OPCODE => (ext_workspace_handle_v1::ExtWorkspaceHandleV1, GlobalData)
    ]);
}

impl<D> Dispatch<ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1, GlobalData, D>
    for WorkspaceState
where
    D: Dispatch<ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1, GlobalData>
        + Dispatch<ext_workspace_handle_v1::ExtWorkspaceHandleV1, GlobalData>
        + WorkspaceHandler
        + 'static,
{
    fn event(
        state: &mut D,
        handle: &ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1,
        event: ext_workspace_group_handle_v1::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        let group = &mut state
            .workspace_state()
            .workspace_groups
            .iter_mut()
            .find(|group| &group.handle == handle)
            .unwrap();
        match event {
            ext_workspace_group_handle_v1::Event::Capabilities { capabilities } => {
                group.pending().capabilities = bitflags_retained(capabilities);
            }
            ext_workspace_group_handle_v1::Event::OutputEnter { output } => {
                group.pending().outputs.push(output);
            }
            ext_workspace_group_handle_v1::Event::OutputLeave { output } => {
                let pending = group.pending();
                if let Some(idx) = pending.outputs.iter().position(|x| x == &output) {
                    pending.outputs.remove(idx);
                }
            }
            ext_workspace_group_handle_v1::Event::WorkspaceEnter { workspace } => {
                group.pending().workspaces.insert(workspace);
            }
            ext_workspace_group_handle_v1::Event::WorkspaceLeave { workspace } => {
                group.pending().workspaces.remove(&workspace);
            }
            ext_workspace_group_handle_v1::Event::Removed => {
                if let Some(idx) = state
                    .workspace_state()
                    .workspace_groups
                    .iter()
                    .position(|group| &group.handle == handle)
                {
                    state.workspace_state().workspace_groups.remove(idx);
                }
            }
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch<ext_workspace_handle_v1::ExtWorkspaceHandleV1, GlobalData, D> for WorkspaceState
where
    D: Dispatch<ext_workspace_handle_v1::ExtWorkspaceHandleV1, GlobalData> + WorkspaceHandler,
{
    fn event(
        state: &mut D,
        handle: &ext_workspace_handle_v1::ExtWorkspaceHandleV1,
        event: ext_workspace_handle_v1::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        let workspace = state
            .workspace_state()
            .workspaces
            .iter_mut()
            .find(|w| &w.handle == handle)
            .unwrap();
        match event {
            ext_workspace_handle_v1::Event::Name { name } => {
                workspace.pending().name = name;
            }
            ext_workspace_handle_v1::Event::Coordinates { coordinates } => {
                workspace.pending().coordinates = coordinates
                    .chunks(4)
                    .map(|chunk| u32::from_ne_bytes(chunk.try_into().unwrap()))
                    .collect();
            }
            ext_workspace_handle_v1::Event::State { state } => {
                workspace.pending().state = bitflags_retained(state);
            }
            ext_workspace_handle_v1::Event::Capabilities { capabilities } => {
                workspace.pending().capabilities = bitflags_retained(capabilities);
            }
            ext_workspace_handle_v1::Event::Id { id } => {
                workspace.pending().id = Some(id);
            }
            ext_workspace_handle_v1::Event::Removed => {
                // Protocol guarantees it will already have been removed from group,
                // so no need to do that here.

                if let Some(idx) = state
                    .workspace_state()
                    .workspaces
                    .iter()
                    .position(|w| &w.handle == handle)
                {
                    state.workspace_state().workspaces.remove(idx);
                }
            }
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch<zcosmic_workspace_manager_v2::ZcosmicWorkspaceManagerV2, GlobalData, D>
    for WorkspaceState
where
    D: Dispatch<zcosmic_workspace_manager_v2::ZcosmicWorkspaceManagerV2, GlobalData>
        + WorkspaceHandler
        + 'static,
{
    fn event(
        _: &mut D,
        _: &zcosmic_workspace_manager_v2::ZcosmicWorkspaceManagerV2,
        _: zcosmic_workspace_manager_v2::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D> Dispatch<zcosmic_workspace_handle_v2::ZcosmicWorkspaceHandleV2, GlobalData, D>
    for WorkspaceState
where
    D: Dispatch<zcosmic_workspace_handle_v2::ZcosmicWorkspaceHandleV2, GlobalData>
        + WorkspaceHandler
        + 'static,
{
    fn event(
        state: &mut D,
        handle: &zcosmic_workspace_handle_v2::ZcosmicWorkspaceHandleV2,
        event: zcosmic_workspace_handle_v2::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        let workspace = state
            .workspace_state()
            .workspaces
            .iter_mut()
            .find(|w| w.cosmic_handle.as_ref() == Some(&handle))
            .unwrap();
        match event {
            zcosmic_workspace_handle_v2::Event::Capabilities { capabilities } => {
                workspace.pending().cosmic_capabilities = bitflags_retained(capabilities);
                workspace.has_cosmic_info = true;
            }
            zcosmic_workspace_handle_v2::Event::TilingState { state } => {
                workspace.pending().tiling = Some(state);
            }
            zcosmic_workspace_handle_v2::Event::State { state } => {
                workspace.pending().cosmic_state = bitflags_retained(state);
            }
            _ => unreachable!(),
        }
    }
}

// Convert bitflags `WEnum` to bitflag type, retaining unrecognized bits
fn bitflags_retained<T: bitflags::Flags<Bits = u32>>(flags: WEnum<T>) -> T {
    match flags {
        WEnum::Value(value) => value,
        WEnum::Unknown(value) => T::from_bits_retain(value),
    }
}

#[macro_export]
macro_rules! delegate_workspace {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::wayland_protocols::ext::workspace::v1::client::ext_workspace_manager_v1::ExtWorkspaceManagerV1: $crate::GlobalData
        ] => $crate::workspace::WorkspaceState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::wayland_protocols::ext::workspace::v1::client::ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1: $crate::GlobalData
        ] => $crate::workspace::WorkspaceState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::wayland_protocols::ext::workspace::v1::client::ext_workspace_handle_v1::ExtWorkspaceHandleV1: $crate::GlobalData
        ] => $crate::workspace::WorkspaceState);

        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::cosmic_protocols::workspace::v2::client::zcosmic_workspace_manager_v2::ZcosmicWorkspaceManagerV2: $crate::GlobalData
        ] => $crate::workspace::WorkspaceState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::cosmic_protocols::workspace::v2::client::zcosmic_workspace_handle_v2::ZcosmicWorkspaceHandleV2: $crate::GlobalData
        ] => $crate::workspace::WorkspaceState);
    };
}
