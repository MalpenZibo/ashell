#[derive(Debug, Clone, PartialEq)]
pub struct CompositorWorkspace {
    pub id: i32,
    pub index: i32,
    pub name: String,
    pub monitor: String,
    pub monitor_id: Option<i128>,
    pub windows: u16,
    pub is_special: bool,
    pub has_urgent: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositorMonitor {
    pub id: i128,
    pub name: String,
    pub active_workspace_id: i32,
    pub special_workspace_id: i32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActiveWindowHyprland {
    pub title: String,
    pub class: String,
    pub address: String,
    pub initial_title: String,
    pub initial_class: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActiveWindowNiri {
    pub title: String,
    pub class: String,
    pub address: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActiveWindow {
    Hyprland(ActiveWindowHyprland),
    Niri(ActiveWindowNiri),
}

impl ActiveWindow {
    pub fn title(&self) -> &str {
        match self {
            ActiveWindow::Hyprland(w) => &w.title,
            ActiveWindow::Niri(w) => &w.title,
        }
    }

    pub fn class(&self) -> &str {
        match self {
            ActiveWindow::Hyprland(w) => &w.class,
            ActiveWindow::Niri(w) => &w.class,
        }
    }

    pub fn initial_title(&self) -> Result<&str, &str> {
        match self {
            ActiveWindow::Hyprland(w) => Ok(&w.initial_title),
            ActiveWindow::Niri(_) => Err("InitialTitle isn't supported on Niri"),
        }
    }

    pub fn initial_class(&self) -> Result<&str, &str> {
        match self {
            ActiveWindow::Hyprland(w) => Ok(&w.initial_class),
            ActiveWindow::Niri(_) => Err("InitialClass isn't supported on Niri"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositorWindow {
    pub id: u64,
    pub workspace_id: Option<i32>,
    pub is_focused: bool,
    pub is_floating: bool,
    pub is_urgent: bool,
    /// Position in a tile grid (column, row), if the compositor lays this
    /// window out in a grid. Only relative ordering is meaningful; the
    /// origin index is compositor-defined.
    pub tile_position: Option<(u32, u32)>,
    /// Tile dimensions in compositor pixels (including decorations), used
    /// for proportional minimap sizing.
    pub tile_width: f32,
    pub tile_height: f32,
}

#[derive(Debug, Clone, Default)]
pub struct CompositorState {
    pub workspaces: Vec<CompositorWorkspace>,
    pub monitors: Vec<CompositorMonitor>,
    pub active_workspace_id: Option<i32>,
    pub active_window: Option<ActiveWindow>,
    pub windows: Vec<CompositorWindow>,
    pub keyboard_layout: String,
    pub submap: Option<String>,
}

#[derive(Debug, Copy, Clone)]
pub enum CompositorChoice {
    Hyprland,
    Niri,
}

#[derive(Debug, Clone)]
pub struct CompositorService {
    /// State is boxed so `ServiceEvent<CompositorService>` stays small —
    /// otherwise `Message` enums embedding it trip clippy's
    /// `large_enum_variant` lint.
    pub state: Box<CompositorState>,
    pub backend: CompositorChoice,
}

#[derive(Debug, Clone)]
pub enum CompositorEvent {
    ActionPerformed, // for now a noop to respond to commands
    StateChanged(Box<CompositorState>),
    // We can add specific events if needed, but a full state sync is safer for workspaces
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum CompositorCommand {
    FocusWorkspace(i32),
    FocusSpecialWorkspace(String),
    FocusMonitor(i128),
    ToggleSpecialWorkspace(String),
    ScrollWorkspace(i32),           // +1 or -1
    CustomDispatch(String, String), // For "vdesk"
    NextLayout,
}
