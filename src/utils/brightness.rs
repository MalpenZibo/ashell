use crate::modules::settings::Message;
use iced::{
    futures::{FutureExt, SinkExt},
    Subscription,
};
use std::{fs, time::Duration};
use tokio::time::sleep;
use zbus::{dbus_proxy, Connection, Result};

const DEVICES_FOLDER: &str = "/sys/class/backlight";

#[dbus_proxy(
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1/session/auto",
    interface = "org.freedesktop.login1.Session"
)]
trait BrightnessCtrl {
    fn set_brightness(&self, subsystem: &str, name: &str, value: u32) -> Result<()>;
}

pub fn subscription(
    rx: Option<tokio::sync::mpsc::UnboundedReceiver<f64>>,
) -> Subscription<Message> {
    iced::subscription::channel("brightness", 100, move |mut output| async move {
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
                .send(Message::BrightnessChanged(
                    current_brightness as f64 / max_brightness as f64,
                ))
                .await;

            let actual_brightness_delayed = || async move {
                sleep(Duration::from_millis(250)).await;
                get_actual_brightness()
            };

            loop {
                iced::futures::select! {
                    v = actual_brightness_delayed().fuse() => {
                        if v != current_brightness {
                            current_brightness = v;
                            let _ = output.send(Message::BrightnessChanged(
                                current_brightness as f64 / max_brightness as f64,
                            )).await;
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
