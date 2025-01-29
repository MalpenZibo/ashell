use std::{any::TypeId, ops::Not, process::Stdio, time::Duration};

use super::{Module, OnModulePress};
use crate::{
    app,
    components::icons::{icon, Icons},
    config::MediaPlayerModuleConfig,
    menu::MenuType,
    style::SettingsButtonStyle,
    utils::launcher::execute_command,
};
use iced::{
    stream::channel,
    widget::{button, column, row, slider, text},
    Alignment::Center,
    Element, Subscription, Task,
};
use log::error;
use tokio::{process, time::sleep};

async fn get_current_song() -> Option<String> {
    let get_current_song_cmd = process::Command::new("bash")
        .arg("-c")
        .arg("playerctl metadata --format \"{{ artist }} - {{ title }}\"")
        .stdout(Stdio::piped())
        .output()
        .await;

    match get_current_song_cmd {
        Ok(get_current_song_cmd) => {
            if !get_current_song_cmd.status.success() {
                return None;
            }
            let s = String::from_utf8_lossy(&get_current_song_cmd.stdout);
            let trimmed = s.trim();
            trimmed.is_empty().not().then(|| trimmed.into())
        }
        Err(e) => {
            error!("Error: {:?}", e);
            None
        }
    }
}

async fn get_volume() -> Option<f64> {
    let get_volume_cmd = process::Command::new("bash")
        .arg("-c")
        .arg("playerctl volume")
        .stdout(Stdio::piped())
        .output()
        .await;

    match get_volume_cmd {
        Ok(get_volume_cmd) => {
            if !get_volume_cmd.status.success() {
                return None;
            }
            let v = String::from_utf8_lossy(&get_volume_cmd.stdout);
            let trimmed = v.trim();
            if trimmed.is_empty() {
                return None;
            }
            match trimmed.parse::<f64>() {
                Ok(v) => Some(v * 100.0),
                Err(e) => {
                    error!("Error: {:?}", e);
                    None
                }
            }
        }
        Err(e) => {
            error!("Error: {:?}", e);
            None
        }
    }
}

#[derive(Default)]
pub struct MediaPlayer {
    song: Option<String>,
    volume: Option<f64>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SetSong(Option<String>),
    Prev,
    Play,
    Next,
    SetVolume(Option<f64>),
    SyncVolume(Option<f64>),
}

impl MediaPlayer {
    pub fn update(
        &mut self,
        message: Message,
        config: &MediaPlayerModuleConfig,
    ) -> Task<crate::app::Message> {
        match message {
            Message::SetSong(song) => {
                if let Some(song) = song {
                    let length = song.len();

                    self.song = Some(if length > config.max_title_length as usize {
                        let split = config.max_title_length as usize / 2;
                        let first_part = song.chars().take(split).collect::<String>();
                        let last_part = song.chars().skip(length - split).collect::<String>();
                        format!("{}...{}", first_part, last_part)
                    } else {
                        song
                    });
                } else {
                    self.song = None;
                }

                Task::none()
            }
            Message::Prev => {
                execute_command("playerctl previous".to_string());
                Task::perform(async move { get_current_song().await }, move |song| {
                    app::Message::MediaPlayer(Message::SetSong(song))
                })
            }
            Message::Play => {
                execute_command("playerctl play-pause".to_string());
                Task::perform(async move { get_current_song().await }, move |song| {
                    app::Message::MediaPlayer(Message::SetSong(song))
                })
            }
            Message::Next => {
                execute_command("playerctl next".to_string());
                Task::perform(async move { get_current_song().await }, move |song| {
                    app::Message::MediaPlayer(Message::SetSong(song))
                })
            }
            Message::SetVolume(v) => {
                if let Some(v) = v {
                    execute_command(format!("playerctl volume {}", v / 100.0));
                }
                self.volume = v;
                Task::none()
            }
            Message::SyncVolume(v) => {
                self.volume = v;
                Task::none()
            }
        }
    }

    pub fn menu_view(&self) -> Element<Message> {
        column![]
            .push_maybe(
                self.volume
                    .map(|v| slider(0.0..=100.0, v, |new_v| Message::SetVolume(Some(new_v)))),
            )
            .push(
                row![
                    button(icon(Icons::SkipPrevious))
                        .on_press(Message::Prev)
                        .padding([5, 12])
                        .style(SettingsButtonStyle.into_style()),
                    button(icon(Icons::PlayPause))
                        .on_press(Message::Play)
                        .style(SettingsButtonStyle.into_style()),
                    button(icon(Icons::SkipNext))
                        .on_press(Message::Next)
                        .padding([5, 12])
                        .style(SettingsButtonStyle.into_style())
                ]
                .spacing(8),
            )
            .spacing(8)
            .align_x(Center)
            .into()
    }
}

impl Module for MediaPlayer {
    type ViewData<'a> = ();
    type SubscriptionData<'a> = ();

    fn view(
        &self,
        (): Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        self.song.clone().and_then(|s| {
            Some((
                text(s).size(12).into(),
                Some(OnModulePress::ToggleMenu(MenuType::MediaPlayer)),
            ))
        })
    }

    fn subscription(&self, (): Self::SubscriptionData<'_>) -> Option<Subscription<app::Message>> {
        let id = TypeId::of::<Self>();

        Some(
            Subscription::run_with_id(
                id,
                channel(10, |mut output| async move {
                    loop {
                        let song = get_current_song().await;
                        let _ = output.try_send(Message::SetSong(song));
                        let volume = get_volume().await;
                        let _ = output.try_send(Message::SyncVolume(volume));
                        sleep(Duration::from_secs(1)).await;
                    }
                }),
            )
            .map(app::Message::MediaPlayer),
        )
    }
}
