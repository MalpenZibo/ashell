use std::{any::TypeId, process::Stdio, time::Duration};

use super::{Module, OnModulePress};
use crate::{
    app,
    components::icons::{icon, Icons},
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

async fn get_current_song() -> String {
    let get_current_song_cmd = process::Command::new("bash")
        .arg("-c")
        .arg("playerctl metadata --format \"{{ artist }} - {{ title }}\"")
        .stdout(Stdio::piped())
        .output()
        .await;

    match get_current_song_cmd {
        Ok(get_current_song_cmd) => String::from_utf8_lossy(&get_current_song_cmd.stdout)
            .trim()
            .into(),
        Err(e) => {
            error!("Error: {:?}", e);
            String::new()
        }
    }
}

fn get_volume() -> f64 {
    let get_current_song_cmd = std::process::Command::new("bash")
        .arg("-c")
        .arg("playerctl volume")
        .stdout(Stdio::piped())
        .output();

    match get_current_song_cmd {
        Ok(check_update_cmd) => {
            String::from_utf8_lossy(&check_update_cmd.stdout)
                .trim()
                .parse::<f64>()
                .unwrap()
                * 100.0
        }
        Err(e) => {
            error!("Error: {:?}", e);
            100.0
        }
    }
}

pub struct Playerctl {
    song: String,
    volume: f64,
}

impl Default for Playerctl {
    fn default() -> Self {
        Self {
            song: String::default(),
            volume: get_volume(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    SetSong(String),
    Prev,
    Play,
    Next,
    SetVolume(f64),
}

impl Playerctl {
    pub fn update(&mut self, message: Message) -> Task<crate::app::Message> {
        match message {
            Message::SetSong(song) => {
                self.song = song;
                Task::none()
            }
            Message::Prev => {
                execute_command("playerctl previous".to_string());
                Task::perform(async move { get_current_song().await }, move |song| {
                    app::Message::Playerctl(Message::SetSong(song))
                })
            }
            Message::Play => {
                execute_command("playerctl play-pause".to_string());
                Task::perform(async move { get_current_song().await }, move |song| {
                    app::Message::Playerctl(Message::SetSong(song))
                })
            }
            Message::Next => {
                execute_command("playerctl next".to_string());
                Task::perform(async move { get_current_song().await }, move |song| {
                    app::Message::Playerctl(Message::SetSong(song))
                })
            }
            Message::SetVolume(v) => {
                execute_command(format!("playerctl volume {}", v / 100.0));
                self.volume = v;
                Task::none()
            }
        }
    }

    pub fn menu_view(&self) -> Element<Message> {
        column![
            slider(0.0..=100.0, self.volume, |new_v| {
                Message::SetVolume(new_v)
            }),
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
            .spacing(8)
        ]
        .spacing(8)
        .align_x(Center)
        .into()
    }
}

impl Module for Playerctl {
    type ViewData<'a> = ();
    type SubscriptionData<'a> = ();

    fn view(
        &self,
        (): Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        Some((
            text(self.song.clone()).size(12).into(),
            Some(OnModulePress::ToggleMenu(MenuType::Playerctl)),
        ))
    }

    fn subscription(&self, (): Self::SubscriptionData<'_>) -> Option<Subscription<app::Message>> {
        let id = TypeId::of::<Self>();

        Some(Subscription::batch(vec![Subscription::run_with_id(
            id,
            channel(10, |mut output| async move {
                loop {
                    let song = get_current_song().await;
                    let _ = output.try_send(Message::SetSong(song));
                    sleep(Duration::from_secs(1)).await;
                }
            }),
        )
        .map(app::Message::Playerctl)]))
    }
}
