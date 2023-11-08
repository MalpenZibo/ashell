use super::{section, slider, SliderToggleMenu, SubMenuType};
use crate::{
    nodes,
    reactive_gtk::{
        container, label, Dynamic, EllipsizeMode, Node, NodeBuilder, Orientation, TextAlign,
    },
    utils::audio::{
        set_microphone, set_sink, set_source, set_volume, toggle_microphone, toggle_volume, Sink,
        Source,
    },
};
use futures_signals::signal::Mutable;

pub fn audio_indicator(
    sinks: Mutable<Vec<Sink>>,
    sources: Mutable<Vec<Source>>,
) -> impl Into<Node> {
    container()
        .spacing(4)
        .children(nodes!(source_indicator(sources), sink_indicator(sinks)))
}

pub fn source_indicator(sources: Mutable<Vec<Source>>) -> impl Into<Node> {
    let format = sources.signal_ref(|sources| {
        sources
            .iter()
            .find_map(|s| {
                if s.active {
                    Some(s.to_icon().to_string())
                } else {
                    None
                }
            })
            .unwrap_or_default()
    });
    let visible = sources.signal_ref(|sources| sources.iter().any(|s| s.active));

    label()
        .class(vec!["source"])
        .text(Dynamic(format))
        .visible(Dynamic(visible))
}

pub fn sink_indicator(sinks: Mutable<Vec<Sink>>) -> impl Into<Node> {
    let format = sinks.signal_ref(|sinks| {
        sinks
            .iter()
            .find_map(|s| {
                if s.active {
                    Some(s.to_icon().to_string())
                } else {
                    None
                }
            })
            .unwrap_or_default()
    });

    label().class(vec!["sink"]).text(Dynamic(format))
}

pub fn sinks_settings(
    submenu: Mutable<Option<SubMenuType>>,
    sinks: Mutable<Vec<Sink>>,
) -> impl Into<Node> {
    let volume_value = sinks.signal_ref(|sinks| {
        sinks
            .iter()
            .find_map(|s| {
                if s.active {
                    Some(s.volume as f64)
                } else {
                    None
                }
            })
            .unwrap_or_default()
    });
    section(
        submenu.clone(),
        slider(
            (
                Dynamic(sinks.signal_ref(|sinks| {
                    sinks
                        .iter()
                        .find_map(|s| {
                            if s.active {
                                Some(s.to_type_icon().to_string())
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default()
                })),
                vec!["sink-icon-fix"],
            ),
            (0., 100.),
            (Dynamic(volume_value), {
                let sinks = sinks.clone();
                move |value| {
                    tokio::spawn(set_volume(sinks.clone(), value as u32));
                }
            }),
            Some({
                let sinks = sinks.clone();
                move || {
                    tokio::spawn(toggle_volume(sinks.clone()));
                }
            }),
            SliderToggleMenu::Enabled((Dynamic(sinks.signal_ref(|sinks| sinks.len() > 1)), {
                let submenu = submenu.clone();
                move || {
                    submenu.set(Some(SubMenuType::Sinks));
                }
            })),
        ),
        vec![sinks_submenu(sinks.clone())],
        true,
    )
}

pub fn sources_settings(
    submenu: Mutable<Option<SubMenuType>>,
    sources: Mutable<Vec<Source>>,
) -> impl Into<Node> {
    let volume_value = sources.signal_ref(|sources| {
        sources
            .iter()
            .find_map(|s| {
                if s.active {
                    Some(s.volume as f64)
                } else {
                    None
                }
            })
            .unwrap_or_default()
    });
    section(
        submenu.clone(),
        slider(
            (
                Dynamic(sources.signal_ref(|sources| {
                    sources
                        .iter()
                        .find_map(|s| {
                            if s.active {
                                Some(s.to_icon().to_string())
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default()
                })),
                vec![],
            ),
            (0., 100.),
            (Dynamic(volume_value), {
                let sources = sources.clone();
                move |value| {
                    tokio::spawn(set_microphone(sources.clone(), value as u32));
                }
            }),
            Some({
                let sources = sources.clone();
                move || {
                    tokio::spawn(toggle_microphone(sources.clone()));
                }
            }),
            SliderToggleMenu::Enabled((
                Dynamic(sources.signal_ref(|sources| sources.len() > 1)),
                {
                    let submenu = submenu.clone();
                    move || {
                        submenu.set(Some(SubMenuType::Sources));
                    }
                },
            )),
        ),
        vec![sources_submenu(sources.clone())],
        Dynamic(sources.signal_ref(|sources| sources.iter().any(|s| s.active))),
    )
}

pub fn sinks_submenu(sinks: Mutable<Vec<Sink>>) -> (SubMenuType, Node) {
    (
        SubMenuType::Sinks,
        container()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .children(Dynamic(sinks.clone().signal_ref({
                move |sinks_ref| {
                    let sinks = sinks.clone();
                    sinks_ref
                        .iter()
                        .map({
                            |s| {
                                let sinks = sinks.clone();
                                let index = s.index;
                                let name = s.name.clone();

                                container()
                                    .class(vec!["menu-voice"])
                                    .spacing(8)
                                    .on_click({
                                        let sinks = sinks.clone();
                                        move || {
                                            tokio::spawn(set_sink(
                                                sinks.clone(),
                                                index,
                                                name.clone(),
                                            ));
                                        }
                                    })
                                    .children(nodes!(
                                        label().text("".to_string()).visible(s.active),
                                        label()
                                            .text_halign(TextAlign::Start)
                                            .ellipsize(EllipsizeMode::End)
                                            .text(s.description.to_string())
                                    ))
                                    .into()
                            }
                        })
                        .collect()
                }
            })))
            .into(),
    )
}

pub fn sources_submenu(sources: Mutable<Vec<Source>>) -> (SubMenuType, Node) {
    (
        SubMenuType::Sources,
        container()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .children(Dynamic(sources.clone().signal_ref({
                move |sources_ref| {
                    let sources = sources.clone();
                    sources_ref
                        .iter()
                        .map({
                            |s| {
                                let sources = sources.clone();
                                let index = s.index;
                                let name = s.name.clone();
                                container()
                                    .class(vec!["menu-voice"])
                                    .spacing(8)
                                    .on_click({
                                        let sources = sources.clone();
                                        move || {
                                            tokio::spawn(set_source(
                                                sources.clone(),
                                                index,
                                                name.clone(),
                                            ));
                                        }
                                    })
                                    .children(nodes!(
                                        label().text("".to_string()).visible(s.active),
                                        label()
                                            .text_halign(TextAlign::Start)
                                            .ellipsize(EllipsizeMode::End)
                                            .text(s.description.to_string())
                                    ))
                                    .into()
                            }
                        })
                        .collect()
                }
            })))
            .into(),
    )
}
