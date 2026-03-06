use guido::prelude::*;

use crate::components::{IconKind, bar_indicator, selectable_item, slider};
use crate::config::SettingsFormat;
use crate::services::audio::{AudioCmd, AudioDataSignals, Sinks, Sources};
use crate::theme::ThemeColors;

use super::SubMenu;

pub fn sink_slider(
    data: AudioDataSignals,
    svc: Service<AudioCmd>,
    submenu: Signal<Option<SubMenu>>,
) -> impl Widget {
    let sinks = data.sinks;
    let server_info = data.server_info;
    let cur_vol = data.cur_sink_volume;

    let svc_change = svc.clone();
    let svc_mute = svc.clone();

    slider()
        .value(cur_vol)
        .kind(move || -> IconKind {
            sinks
                .with(|s| Sinks::get_icon(s, &server_info.with(|si| si.default_sink.clone())))
                .into()
        })
        .muted(move || {
            let si = server_info.with(|si| si.default_sink.clone());
            sinks.with(|s| {
                s.iter()
                    .find(|d| d.name == si && d.ports.iter().any(|p| p.active))
                    .map(|d| d.is_mute)
                    .unwrap_or(false)
            })
        })
        .on_change(move |vol| svc_change.send(AudioCmd::SinkVolume(vol)))
        .on_mute_toggle(move || svc_mute.send(AudioCmd::ToggleSinkMute))
        .expanded(move || submenu.get() == Some(SubMenu::Sinks))
        .on_chevron(move || {
            submenu.set(if submenu.get() == Some(SubMenu::Sinks) {
                None
            } else {
                Some(SubMenu::Sinks)
            });
        })
}

pub fn source_slider(
    data: AudioDataSignals,
    svc: Service<AudioCmd>,
    submenu: Signal<Option<SubMenu>>,
) -> impl Widget {
    let sources = data.sources;
    let server_info = data.server_info;
    let cur_vol = data.cur_source_volume;

    let svc_change = svc.clone();
    let svc_mute = svc.clone();

    slider()
        .value(cur_vol)
        .kind(move || -> IconKind {
            sources
                .with(|s| Sources::get_icon(s, &server_info.with(|si| si.default_source.clone())))
                .into()
        })
        .muted(move || {
            let si = server_info.with(|si| si.default_source.clone());
            sources.with(|s| {
                s.iter()
                    .find(|d| d.name == si && d.ports.iter().any(|p| p.active))
                    .map(|d| d.is_mute)
                    .unwrap_or(false)
            })
        })
        .on_change(move |vol| svc_change.send(AudioCmd::SourceVolume(vol)))
        .on_mute_toggle(move || svc_mute.send(AudioCmd::ToggleSourceMute))
        .expanded(move || submenu.get() == Some(SubMenu::Sources))
        .on_chevron(move || {
            submenu.set(if submenu.get() == Some(SubMenu::Sources) {
                None
            } else {
                Some(SubMenu::Sources)
            });
        })
}

/// Bar indicator: speaker icon and/or volume %
pub fn sink_indicator(data: AudioDataSignals, format: SettingsFormat) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let sinks = data.sinks;
    let server_info = data.server_info;
    let cur_vol = data.cur_sink_volume;

    bar_indicator()
        .kind(move || -> IconKind {
            sinks
                .with(|s| Sinks::get_icon(s, &server_info.with(|si| si.default_sink.clone())))
                .into()
        })
        .label(move || Some(format!("{}%", cur_vol.get())))
        .color(theme.text)
        .format(format)
}

/// Bar indicator: mic icon and/or volume %
pub fn source_indicator(data: AudioDataSignals, format: SettingsFormat) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let sources = data.sources;
    let server_info = data.server_info;
    let cur_vol = data.cur_source_volume;

    bar_indicator()
        .kind(move || -> IconKind {
            sources
                .with(|s| Sources::get_icon(s, &server_info.with(|si| si.default_source.clone())))
                .into()
        })
        .label(move || Some(format!("{}%", cur_vol.get())))
        .color(theme.text)
        .format(format)
}

/// Sinks submenu: list all sinks with active port selection
pub fn sinks_submenu(data: AudioDataSignals, svc: Service<AudioCmd>) -> impl Widget {
    let sinks = data.sinks;
    let server_info = data.server_info;

    container()
        .width(fill())
        .layout(Flex::column().spacing(4))
        .child(move || {
            let devices = sinks.with(|s| s.clone());
            let default = server_info.with(|si| si.default_sink.clone());
            let mut col = container().width(fill()).layout(Flex::column().spacing(2));
            for device in devices {
                for port in &device.ports {
                    let name = device.name.clone();
                    let port_name = port.name.clone();
                    let desc = port.description.clone();
                    let is_active = device.name == default && port.active;
                    let svc = svc.clone();
                    col = col.child(
                        selectable_item()
                            .kind(port.device_type.get_icon())
                            .label(desc)
                            .selected(is_active)
                            .on_click(move || {
                                svc.send(AudioCmd::DefaultSink(name.clone(), port_name.clone()));
                            }),
                    );
                }
            }
            Some(col)
        })
}

/// Sources submenu: list all sources with active port selection
pub fn sources_submenu(data: AudioDataSignals, svc: Service<AudioCmd>) -> impl Widget {
    let sources = data.sources;
    let server_info = data.server_info;

    container()
        .width(fill())
        .layout(Flex::column().spacing(4))
        .child(move || {
            let devices = sources.with(|s| s.clone());
            let default = server_info.with(|si| si.default_source.clone());
            let mut col = container().width(fill()).layout(Flex::column().spacing(2));
            for device in devices {
                for port in &device.ports {
                    let name = device.name.clone();
                    let port_name = port.name.clone();
                    let desc = port.description.clone();
                    let is_active = device.name == default && port.active;
                    let svc = svc.clone();
                    col = col.child(
                        selectable_item()
                            .kind(port.device_type.get_icon())
                            .label(desc)
                            .selected(is_active)
                            .on_click(move || {
                                svc.send(AudioCmd::DefaultSource(name.clone(), port_name.clone()));
                            }),
                    );
                }
            }
            Some(col)
        })
}
