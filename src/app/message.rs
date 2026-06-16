use crate::{
    components::{ButtonUIRef, menu::MenuType},
    config::Config,
    ipc::IpcCommand,
    modules::{self, custom_module},
    osd,
};
use iced::{OutputEvent, SurfaceId};

#[derive(Debug, Clone)]
pub enum Message {
    ConfigChanged(Box<Config>),
    ToggleMenu(MenuType, SurfaceId, ButtonUIRef),
    CloseMenu(SurfaceId),
    FinishCloseMenu(SurfaceId),
    Custom(String, custom_module::Message),
    Updates(modules::updates::Message),
    Workspaces(modules::workspaces::Message),
    WindowTitle(modules::window_title::Message),
    SystemInfo(modules::system_info::Message),
    KeyboardLayout(modules::keyboard_layout::Message),
    KeyboardSubmap(modules::keyboard_submap::Message),
    Tray(modules::tray::Message),
    Tempo(modules::tempo::Message),
    Privacy(modules::privacy::Message),
    Settings(modules::settings::Message),
    MediaPlayer(modules::media_player::Message),
    Notifications(modules::notifications::Message),
    Osd(osd::Message),
    IpcOsdCommand(IpcCommand),
    OutputEvent(OutputEvent),
    CloseAllMenus,
    ResumeFromSleep,
    None,
    ToggleVisibility,
}
