pub mod audio;
pub mod bluetooth;
pub mod brightness;
pub mod compat;
pub mod compositor;
pub mod idle_inhibitor;
pub mod logind;
pub mod mpris;
pub mod network;
pub mod notifications;
pub mod privacy;
pub mod system_info;
mod throttle;
pub mod tray;
pub mod updates;
pub mod upower;
pub mod xdg_icons;

// Upstream ashell service files are copied here nearly verbatim; they
// import the service traits via `use super::{...}`, which these re-exports
// satisfy (see compat.rs for the iced-surface mimic).
pub use compat::{ReadOnlyService, Service, ServiceEvent};
