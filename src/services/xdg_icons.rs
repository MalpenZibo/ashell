use crate::components::icons::StaticIcon;
use ashell_services::xdg_icons;
use iced::widget::{image, svg};

pub use ashell_services::xdg_icons::{find_icon_path, warm_cache_async};

#[derive(Debug, Clone)]
pub enum XdgIcon {
    Image(image::Handle),
    Svg(svg::Handle),
    NerdFont(StaticIcon),
}

impl From<xdg_icons::XdgIcon> for XdgIcon {
    fn from(icon: xdg_icons::XdgIcon) -> Self {
        match icon {
            xdg_icons::XdgIcon::Image(path) => Self::Image(image::Handle::from_path(path)),
            xdg_icons::XdgIcon::Svg(path) => Self::Svg(svg::Handle::from_path(path)),
        }
    }
}

pub fn fallback_icon() -> XdgIcon {
    XdgIcon::NerdFont(StaticIcon::Point)
}

pub fn get_icon_from_name(icon_name: &str) -> Option<XdgIcon> {
    xdg_icons::get_icon_from_name(icon_name).map(XdgIcon::from)
}
