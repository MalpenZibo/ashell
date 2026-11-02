use crate::components::icons::StaticIcon;
use ashell_services::xdg_icons;
use iced::widget::{image, svg};

pub use ashell_services::xdg_icons::warm_cache_async;

#[derive(Debug, Clone)]
pub enum XdgIcon {
    Image(image::Handle),
    Svg { handle: svg::Handle, symbolic: bool },
    NerdFont(StaticIcon),
}

impl From<xdg_icons::IconFile> for XdgIcon {
    fn from(icon: xdg_icons::IconFile) -> Self {
        let symbolic = icon.is_symbolic();
        match icon {
            xdg_icons::IconFile::Image(path) => Self::Image(image::Handle::from_path(path)),
            xdg_icons::IconFile::Svg(path) => Self::Svg {
                handle: svg::Handle::from_path(path),
                symbolic,
            },
        }
    }
}

pub fn fallback_icon() -> XdgIcon {
    XdgIcon::NerdFont(StaticIcon::Point)
}

pub fn get_icon_from_name(icon_name: &str) -> Option<XdgIcon> {
    xdg_icons::get_icon_from_name(icon_name).map(XdgIcon::from)
}

pub fn get_icon_from_theme_path(theme_path: &str, icon_name: &str) -> Option<XdgIcon> {
    xdg_icons::get_icon_from_theme_path(theme_path, icon_name).map(XdgIcon::from)
}
