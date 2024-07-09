use crate::modules::settings::brightness::BrightnessMessage;
use iced::{
    futures::{self, FutureExt, SinkExt, StreamExt},
    subscription, Subscription,
};
use inotify::{Inotify, WatchMask};
use std::fs;
use zbus::{proxy, Connection, Result};

const DEVICES_FOLDER: &str = "/sys/class/backlight";

#[proxy(
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1/session/auto",
    interface = "org.freedesktop.login1.Session"
)]
trait BrightnessCtrl {
    fn set_brightness(&self, subsystem: &str, name: &str, value: u32) -> Result<()>;
}

pub fn subscription(
    rx: Option<tokio::sync::mpsc::UnboundedReceiver<f64>>,
) -> Subscription<BrightnessMessage> {
    subscription::channel("brightness", 100, move |mut output| async move {
        let mut rx = rx.unwrap();

        let device_folder = fs::read_dir(DEVICES_FOLDER)
            .ok()
            .and_then(|mut d| d.next().and_then(|entry| entry.ok()));

        if let Some(device_folder) = device_folder {
            let device_name = device_folder.file_name().into_string().unwrap();

            let conn = Connection::system().await.unwrap();
            let brightness_ctrl = BrightnessCtrlProxy::new(&conn).await.unwrap();

            let device_folder = device_folder.path();

            let max_brightness = fs::read_to_string(device_folder.join("max_brightness"))
                .ok()
                .and_then(|v| v.trim().parse::<u32>().ok())
                .unwrap_or(0);

            let actual_brightness_file = device_folder.join("actual_brightness");

            let get_actual_brightness = || {
                fs::read_to_string(actual_brightness_file.as_path())
                    .ok()
                    .and_then(|v| v.trim().parse::<u32>().ok())
                    .unwrap_or(0)
            };

            let mut current_brightness = get_actual_brightness();

            let _ = output
                .send(BrightnessMessage::Changed(
                    current_brightness as f64 / max_brightness as f64,
                    true,
                ))
                .await;

            let inotify = Inotify::init().expect("Failed to initialize inotify");

            inotify
                .watches()
                .add(&actual_brightness_file, WatchMask::MODIFY)
                .expect("Failed to add file watch");

            let buffer = [0; 1024];
            let mut watcher_stream = inotify
                .into_event_stream(buffer)
                .expect("Failed to create a brightness file event stream");

            loop {
                futures::select! {
                    v = watcher_stream.next().fuse() => {
                        if let Some(Ok(_)) = v {
                            let v = get_actual_brightness();
                            if v != current_brightness {
                                current_brightness = v;
                                let _ = output.send(BrightnessMessage::Changed(
                                    current_brightness as f64 / max_brightness as f64,
                                    true
                                )).await;
                            }
                        }
                    }
                    v = rx.recv().fuse() => {
                        if let Some(brightness_value) = v {
                            let brightness_value = brightness_value.clamp(0., 1.0);
                            let _ = brightness_ctrl.set_brightness(
                                "backlight",
                                &device_name,
                                (brightness_value * max_brightness as f64).round() as u32
                            ).await;
                        }
                    }
                }
            }
        } else {
            loop {
                rx.recv().await;
            }
        }
    })
}
