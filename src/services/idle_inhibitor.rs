use crate::components::icons::StaticIcon;
use log::warn;

pub use ashell_services::idle_inhibitor::IdleInhibitor;

/// `None`, after a warning, when the compositor can't be reached.
pub fn connect() -> Option<IdleInhibitor> {
    IdleInhibitor::connect()
        .inspect_err(|err| warn!("Failed to initialize idle inhibitor: {err}"))
        .ok()
}

pub fn icon(active: bool) -> StaticIcon {
    if active {
        StaticIcon::EyeOpened
    } else {
        StaticIcon::EyeClosed
    }
}
