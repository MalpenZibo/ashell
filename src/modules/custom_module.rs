use guido::prelude::*;
use serde::Deserialize;
use std::process::Stdio;
use tokio::io::AsyncBufReadExt;

use crate::components::icons::DynamicIcon;
use crate::components::{IconKind, icon};
use crate::config::{CustomModuleDef, CustomModuleType};
use crate::theme::ThemeColors;
use crate::utils::launcher::execute_command;

#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
pub struct CustomListenData {
    pub alt: String,
    pub text: Option<String>,
}

#[derive(Clone)]
pub struct CustomHandle {
    pub def: CustomModuleDef,
    pub data: RwSignal<CustomListenData>,
}

pub fn create(def: CustomModuleDef) -> CustomHandle {
    let data = create_signal(CustomListenData::default());

    if let Some(listen_cmd) = def.listen_cmd.clone() {
        let writer = data.writer();
        let name = def.name.clone();
        let _ = create_service::<(), _, _>(move |_rx, _ctx| async move {
            let command = tokio::process::Command::new("bash")
                .arg("-c")
                .arg(&listen_cmd)
                .stdout(Stdio::piped())
                .kill_on_drop(true)
                .spawn();

            match command {
                Ok(mut child) => {
                    if let Some(stdout) = child.stdout.take() {
                        let mut reader = tokio::io::BufReader::new(stdout).lines();
                        let mut buf = String::new();

                        // The child makes progress on its own while we await
                        // output lines
                        tokio::spawn(async move {
                            match child.wait().await {
                                Ok(status) => log::info!("child status was: {status}"),
                                Err(e) => log::warn!("child process encountered an error: {e}"),
                            }
                        });

                        // Newline-delimited JSON, buffering multi-line
                        // payloads with a 1 MiB guard (upstream logic)
                        while let Some(line) = reader.next_line().await.ok().flatten() {
                            buf.push_str(&line);
                            buf.push('\n');
                            match serde_json::from_str::<CustomListenData>(&buf) {
                                Ok(event) => {
                                    buf.clear();
                                    writer.set(event);
                                }
                                Err(e) if e.is_eof() => {
                                    if buf.len() > 1 << 20 {
                                        log::warn!(
                                            "custom module '{name}': dropping {} bytes of unterminated JSON",
                                            buf.len()
                                        );
                                        buf.clear();
                                    }
                                }
                                Err(e) => {
                                    log::error!(
                                        "Failed to parse JSON for custom module '{name}': {e} (payload: {buf})"
                                    );
                                    buf.clear();
                                }
                            }
                        }
                    } else {
                        log::error!("Failed to capture stdout for command: {listen_cmd}");
                    }
                }
                Err(error) => {
                    log::error!("Failed to execute command: {error}");
                }
            }
        });
    }

    CustomHandle { def, data }
}

pub fn view(handle: CustomHandle) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let CustomHandle { def, data } = handle;

    match def.r#type {
        CustomModuleType::Text => container()
            .child(move || {
                data.with(|d| {
                    d.text
                        .clone()
                        .filter(|t| !t.is_empty())
                        .map(|t| text(t).color(theme.text).font_size(13))
                })
            })
            .into_any(),
        CustomModuleType::Button => {
            let icons_map = def.icons.clone();
            let base_icon = def.icon.clone();
            let alert_re = def.alert.clone();

            // Icon (config regex overrides) + top-right alert dot
            let icon_kind = create_memo(move || -> IconKind {
                let alt = data.with(|d| d.alt.clone());
                if let Some(map) = &icons_map {
                    for (re, icon_str) in map {
                        if re.0.is_match(&alt) {
                            return DynamicIcon(icon_str.clone()).into();
                        }
                    }
                }
                match &base_icon {
                    Some(s) => DynamicIcon(s.clone()).into(),
                    None => DynamicIcon(String::new()).into(),
                }
            });
            let show_alert = create_memo(move || {
                alert_re
                    .as_ref()
                    .is_some_and(|re| data.with(|d| re.0.is_match(&d.alt)))
            });

            let icon_wr = create_widget_ref();
            let icon_stack = container()
                .layout(Overlay::new())
                .child(
                    container()
                        .widget_ref(icon_wr)
                        .padding([0, 1])
                        .child(icon().kind(move || icon_kind.get()).color(theme.text)),
                )
                .child(container().child(move || {
                    show_alert.get().then(|| {
                        let w = icon_wr.rect().get().width;
                        container()
                            .width(w.max(4.0))
                            .layout(Flex::row().main_alignment(MainAlignment::End))
                            .child(
                                container()
                                    .width(4)
                                    .height(4)
                                    .corner_radius(2)
                                    .background(theme.danger),
                            )
                    })
                }));

            let mut row = container()
                .height(fill())
                .layout(
                    Flex::row()
                        .spacing(4)
                        .cross_alignment(CrossAlignment::Center),
                )
                .child(icon_stack)
                .child(container().child(move || {
                    data.with(|d| {
                        d.text
                            .clone()
                            .filter(|t| !t.is_empty())
                            .map(|t| text(t).color(theme.text).font_size(13))
                    })
                }));

            if let Some(cmd) = def.command.clone() {
                row = row.on_click(move || execute_command(&cmd));
            }
            let (up, down) = (def.on_scroll_up.clone(), def.on_scroll_down.clone());
            if up.is_some() || down.is_some() {
                let accum = std::cell::Cell::new(0.0f32);
                row = row.on_scroll(move |_dx, dy, _src| {
                    let acc = accum.get() + dy;
                    if acc.abs() < 3.0 {
                        accum.set(acc);
                        return;
                    }
                    accum.set(0.0);
                    let cmd = if acc < 0.0 { &up } else { &down };
                    if let Some(cmd) = cmd {
                        execute_command(cmd);
                    }
                });
            }
            // on_right_click / on_middle_click need right/middle button
            // support in guido containers; unwired until then.

            row.into_any()
        }
    }
}
