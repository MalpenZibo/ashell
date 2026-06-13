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
    pub window_classes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositorMonitor {
    pub id: i128,
    pub name: String,
    pub focused: bool,
    pub active_workspace_id: i32,
    pub active_workspace_name: String,
    pub special_workspace_name: String,
    /// Logical size of the monitor in compositor pixels, if known.
    pub logical_size: Option<(u32, u32)>,
    /// Top-left of this output's view onto its active workspace, in
    /// workspace-layout coordinates (column 1 at x = 0, the tops of the
    /// columns at y = 0). The output shows the region
    /// `view_point..view_point + logical_size`. `None` if the
    /// compositor doesn't expose a scrolling view position.
    pub view_point: Option<(f32, f32)>,
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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActiveWindowGeneric {
    pub title: String,
    pub class: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActiveWindowMango {
    pub title: String,
    pub class: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActiveWindowWayfire {
    pub title: String,
    pub class: String,
    pub address: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActiveWindow {
    Hyprland(ActiveWindowHyprland),
    Niri(ActiveWindowNiri),
    Generic(ActiveWindowGeneric),
    Mango(ActiveWindowMango),
    Wayfire(ActiveWindowWayfire),
}

impl ActiveWindow {
    pub fn title(&self) -> &str {
        match self {
            ActiveWindow::Hyprland(w) => &w.title,
            ActiveWindow::Niri(w) => &w.title,
            ActiveWindow::Generic(w) => &w.title,
            ActiveWindow::Mango(w) => &w.title,
            ActiveWindow::Wayfire(w) => &w.title,
        }
    }

    pub fn class(&self) -> &str {
        match self {
            ActiveWindow::Hyprland(w) => &w.class,
            ActiveWindow::Niri(w) => &w.class,
            ActiveWindow::Generic(w) => &w.class,
            ActiveWindow::Mango(w) => &w.class,
            ActiveWindow::Wayfire(w) => &w.class,
        }
    }

    pub fn initial_title(&self) -> Result<&str, &str> {
        match self {
            ActiveWindow::Hyprland(w) => Ok(&w.initial_title),
            ActiveWindow::Niri(_) => Err("InitialTitle isn't supported on Niri"),
            ActiveWindow::Generic(_) => Err("InitialTitle isn't supported on generic Wayland"),
            ActiveWindow::Mango(_) => Err("InitialTitle isn't supported on MangoWC"),
            ActiveWindow::Wayfire(_) => Err("InitialTitle isn't supported on Wayfire"),
        }
    }

    pub fn initial_class(&self) -> Result<&str, &str> {
        match self {
            ActiveWindow::Hyprland(w) => Ok(&w.initial_class),
            ActiveWindow::Niri(_) => Err("InitialClass isn't supported on Niri"),
            ActiveWindow::Generic(_) => Err("InitialClass isn't supported on generic Wayland"),
            ActiveWindow::Mango(_) => Err("InitialClass isn't supported on MangoWC"),
            ActiveWindow::Wayfire(_) => Err("InitialClass isn't supported on Wayfire"),
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
    /// Tile dimensions in logical pixels (including decorations), used
    /// for proportional minimap sizing.
    pub tile_width: f32,
    pub tile_height: f32,
    /// Absolute position of a floating window in workspace-layout
    /// coordinates (the same space as the tiled grid; column 1 at x = 0).
    /// Set only for floating windows, which aren't placed by grid index;
    /// `None` for tiled windows and when the compositor doesn't expose
    /// enough to locate the window in that space.
    pub floating_pos: Option<(f32, f32)>,
}

#[derive(Debug, Clone, Default)]
pub struct CompositorState {
    pub workspaces: Vec<CompositorWorkspace>,
    pub monitors: Vec<CompositorMonitor>,
    // Tag-based compositors (e.g. MangoWC) can have several active workspaces at
    // once; single-workspace compositors fill this with zero or one id.
    pub active_workspace_ids: Vec<i32>,
    /// The workspace holding keyboard focus, when the compositor exposes a
    /// global focus. `None` where it doesn't (e.g. ext-workspace); don't fall
    /// back to `active_workspace_ids`, whose ordering is backend-defined.
    pub focused_workspace_id: Option<i32>,
    pub active_window: Option<ActiveWindow>,
    /// Per-window state. Only the Niri backend fills it; it's empty on the
    /// other backends.
    pub windows: Vec<CompositorWindow>,
    pub keyboard_layout: String,
    pub submap: Option<String>,
}

#[derive(Debug, Copy, Clone)]
pub enum CompositorChoice {
    Hyprland,
    Niri,
    Generic,
    Mango,
    Wayfire,
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
pub enum CompositorCommand {
    FocusWorkspace(i32),
    FocusNamedWorkspace(String),
    ToggleSpecialWorkspace(String),
    CustomDispatch(String, String), // For "vdesk"
    NextLayout,
    /// Focus a window of an application: one owned by `pid`, or, when none is
    /// (XWayland or sandboxed apps report another pid), one whose app id or
    /// class is in `apps`. Among those, prefer the one whose title contains
    /// `title` (e.g. the browser window showing the tab that plays the media).
    FocusAppWindow {
        pid: Option<u32>,
        apps: Vec<String>,
        title: Option<String>,
    },
}
