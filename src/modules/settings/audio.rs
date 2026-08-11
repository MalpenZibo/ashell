use guido::prelude::*;

use crate::components::{IconKind, StaticIcon, bar_indicator, selectable_item, slider};
use crate::config::SettingsFormat;
use crate::services::audio::{AudioCommand, AudioService, ChannelVolumesExt, DevicePortType, Port};
use crate::services::compat::ServiceSignal;
use crate::theme::ThemeColors;

use super::SubMenu;

const NORMAL_VOLUME: u32 = libpulse_binding::volume::Volume::NORMAL.0;

fn to_percent(raw: u32) -> i32 {
    (raw as f32 / NORMAL_VOLUME as f32 * 100.0).round() as i32
}

fn to_raw(percent: i32) -> u32 {
    (percent.max(0) as f32 / 100.0 * NORMAL_VOLUME as f32).round() as u32
}

fn sink_icon(s: &AudioService) -> StaticIcon {
    match s.active_sink() {
        Some(sink) if !sink.is_mute => match to_percent(sink.volume.get_volume()) {
            0..=33 => StaticIcon::Speaker1,
            34..=66 => StaticIcon::Speaker2,
            _ => StaticIcon::Speaker3,
        },
        _ => StaticIcon::Speaker0,
    }
}

fn source_icon(s: &AudioService) -> StaticIcon {
    match s.active_source() {
        Some(source) if !source.is_mute => StaticIcon::Mic1,
        _ => StaticIcon::Mic0,
    }
}

fn port_icon(port: Option<&Port>) -> StaticIcon {
    match port.map(|p| p.device_type) {
        Some(DevicePortType::Headphones) => StaticIcon::Headphones1,
        Some(DevicePortType::Headset) => StaticIcon::Headset,
        Some(DevicePortType::HDMI | DevicePortType::TV | DevicePortType::Video) => {
            StaticIcon::MonitorSpeaker
        }
        Some(DevicePortType::Mic) => StaticIcon::Mic1,
        _ => StaticIcon::Speaker3,
    }
}

pub fn sink_slider(
    data: ServiceSignal<AudioService>,
    svc: Service<AudioCommand>,
    submenu: RwSignal<Option<SubMenu>>,
) -> impl Widget {
    let svc_change = svc.clone();
    let svc_mute = svc.clone();

    slider()
        .value(move || {
            data.with(|s| {
                s.as_ref()
                    .map(|x| to_percent(x.sink_slider.value()))
                    .unwrap_or(0)
            })
        })
        .kind(move || -> IconKind {
            data.with(|s| {
                s.as_ref()
                    .map(|x| sink_icon(x))
                    .unwrap_or(StaticIcon::Speaker0)
            })
            .into()
        })
        .muted(move || {
            data.with(|s| {
                s.as_ref()
                    .and_then(|x| x.active_sink())
                    .map(|d| d.is_mute)
                    .unwrap_or(false)
            })
        })
        .on_change(move |vol| svc_change.send(AudioCommand::SinkVolume(to_raw(vol))))
        .on_mute_toggle(move || svc_mute.send(AudioCommand::ToggleSinkMute))
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
    data: ServiceSignal<AudioService>,
    svc: Service<AudioCommand>,
    submenu: RwSignal<Option<SubMenu>>,
) -> impl Widget {
    let svc_change = svc.clone();
    let svc_mute = svc.clone();

    slider()
        .value(move || {
            data.with(|s| {
                s.as_ref()
                    .map(|x| to_percent(x.source_slider.value()))
                    .unwrap_or(0)
            })
        })
        .kind(move || -> IconKind {
            data.with(|s| {
                s.as_ref()
                    .map(|x| source_icon(x))
                    .unwrap_or(StaticIcon::Mic0)
            })
            .into()
        })
        .muted(move || {
            data.with(|s| {
                s.as_ref()
                    .and_then(|x| x.active_source())
                    .map(|d| d.is_mute)
                    .unwrap_or(false)
            })
        })
        .on_change(move |vol| svc_change.send(AudioCommand::SourceVolume(to_raw(vol))))
        .on_mute_toggle(move || svc_mute.send(AudioCommand::ToggleSourceMute))
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
pub fn sink_indicator(data: ServiceSignal<AudioService>, format: SettingsFormat) -> impl Widget {
    let theme = expect_context::<ThemeColors>();

    bar_indicator()
        .kind(move || -> IconKind {
            data.with(|s| {
                s.as_ref()
                    .map(|x| sink_icon(x))
                    .unwrap_or(StaticIcon::Speaker0)
            })
            .into()
        })
        .label(move || {
            Some(format!(
                "{}%",
                data.with(|s| {
                    s.as_ref()
                        .map(|x| to_percent(x.sink_slider.value()))
                        .unwrap_or(0)
                })
            ))
        })
        .color(theme.text)
        .format(format)
}

/// Bar indicator: mic icon and/or volume %
pub fn source_indicator(data: ServiceSignal<AudioService>, format: SettingsFormat) -> impl Widget {
    let theme = expect_context::<ThemeColors>();

    bar_indicator()
        .kind(move || -> IconKind {
            data.with(|s| {
                s.as_ref()
                    .map(|x| source_icon(x))
                    .unwrap_or(StaticIcon::Mic0)
            })
            .into()
        })
        .label(move || {
            Some(format!(
                "{}%",
                data.with(|s| {
                    s.as_ref()
                        .map(|x| to_percent(x.source_slider.value()))
                        .unwrap_or(0)
                })
            ))
        })
        .color(theme.text)
        .format(format)
}

/// Sinks submenu: selectable audio routes (upstream's sink_iter — ports,
/// portless devices and smart filters)
pub fn sinks_submenu(data: ServiceSignal<AudioService>, svc: Service<AudioCommand>) -> impl Widget {
    submenu_view(data, svc, true)
}

/// Sources submenu
pub fn sources_submenu(
    data: ServiceSignal<AudioService>,
    svc: Service<AudioCommand>,
) -> impl Widget {
    submenu_view(data, svc, false)
}

fn submenu_view(
    data: ServiceSignal<AudioService>,
    svc: Service<AudioCommand>,
    sinks: bool,
) -> impl Widget {
    // Memo cutoff: the service snapshot notifies on EVERY audio event
    // (volume ticks included) — the row list must rebuild only when the
    // routes actually change.
    let routes_memo = create_memo(
        move || -> Vec<(String, StaticIcon, String, Option<String>, bool)> {
            data.with(|s| {
                let Some(s) = s.as_ref() else {
                    return Vec::new();
                };
                let default = if sinks {
                    &s.server_info.default_sink
                } else {
                    &s.server_info.default_source
                };
                let iter = if sinks {
                    s.sink_iter().collect::<Vec<_>>()
                } else {
                    s.source_iter().collect::<Vec<_>>()
                };
                iter.into_iter()
                    .map(|route| {
                        (
                            route.to_string(),
                            port_icon(route.port),
                            route.device.name.clone(),
                            route.port.map(|p| p.name.clone()),
                            route.device.name == *default,
                        )
                    })
                    .collect()
            })
        },
    );
    container()
        .width(fill())
        .layout(Flex::column().spacing(4))
        .child(move || {
            let routes = routes_memo.get();

            let mut col = container().width(fill()).layout(Flex::column().spacing(2));
            for (label, ic, device_name, port_name, is_active) in routes {
                let svc = svc.clone();
                col = col.child(
                    selectable_item()
                        .kind(ic)
                        .label(label)
                        .selected(is_active)
                        .on_click(move || {
                            svc.send(if sinks {
                                AudioCommand::DefaultSink(device_name.clone(), port_name.clone())
                            } else {
                                AudioCommand::DefaultSource(device_name.clone(), port_name.clone())
                            });
                        }),
                );
            }
            Some(col)
        })
}
