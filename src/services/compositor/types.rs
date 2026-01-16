#[derive(Debug, Clone, PartialEq)]
pub struct CompositorWorkspace {
    pub id: i32,
    pub name: String,
    pub monitor: String,
    pub monitor_id: Option<i128>,
    pub windows: u16,
    pub is_special: bool,
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

#[derive(Debug, Clone, Default)]
pub struct CompositorState {
    pub workspaces: Vec<CompositorWorkspace>,
    pub monitors: Vec<CompositorMonitor>,
    pub active_workspace_id: Option<i32>,
    pub active_window: Option<ActiveWindow>,
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
    pub state: CompositorState,
    pub backend: CompositorChoice,
}

#[derive(Debug, Clone)]
pub enum CompositorEvent {
    ActionPerformed, // for now a noop to respond to commands
    StateChanged(CompositorState),
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
