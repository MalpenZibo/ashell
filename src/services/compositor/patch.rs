use super::types::CompositorState;

/// A partial update to the merged [`CompositorState`].
///
/// Each state source (compositor-specific or generic Wayland protocol) emits
/// patches for the slice it owns; the central listener applies them onto the
/// authoritative state. [`StatePatch::Full`] lets a source that already
/// produces a complete snapshot (Hyprland/Niri) replace the state wholesale.
#[derive(Debug, Clone)]
pub enum StatePatch {
    Full(Box<CompositorState>),
}

impl StatePatch {
    pub fn apply_to(self, state: &mut CompositorState) {
        match self {
            StatePatch::Full(new_state) => *state = *new_state,
        }
    }
}
