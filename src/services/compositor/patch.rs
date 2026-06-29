use super::types::{ActiveWindow, CompositorMonitor, CompositorState, CompositorWorkspace};

/// A partial update to the merged [`CompositorState`]. `Full` replaces the
/// whole snapshot (Hyprland/Niri); the slice variants let independent generic
/// sources contribute only the part they own.
#[derive(Debug, Clone)]
pub enum StatePatch {
    Full(Box<CompositorState>),
    Topology {
        workspaces: Vec<CompositorWorkspace>,
        monitors: Vec<CompositorMonitor>,
        active_workspace_id: Option<i32>,
    },
    ActiveWindow(Option<ActiveWindow>),
}

impl StatePatch {
    pub fn apply_to(self, state: &mut CompositorState) {
        match self {
            StatePatch::Full(new_state) => *state = *new_state,
            StatePatch::Topology {
                workspaces,
                monitors,
                active_workspace_id,
            } => {
                state.workspaces = workspaces;
                state.monitors = monitors;
                state.active_workspace_id = active_workspace_id;
            }
            StatePatch::ActiveWindow(window) => state.active_window = window,
        }
    }
}
