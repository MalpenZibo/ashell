use crate::services::{ReadOnlyService, ServiceEvent, xdg_icons};
use iced::Subscription;
use iced::futures::{SinkExt, StreamExt, channel::mpsc::Sender, stream::pending};
use iced::stream::channel;
use iced::widget::{image, svg};
use log::{error, info};
use std::any::TypeId;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use zbus::{Connection, OwnedVariant};

pub mod dbus;

use dbus::NotificationEvent;
pub use dbus::{Notification, Urgency};

#[derive(Debug, Clone)]
pub enum NotificationIcon {
    Image(image::Handle),
    Svg(svg::Handle),
}

impl NotificationIcon {
    pub fn resolve(
        app_name: &str,
        app_icon: &str,
        hints: &HashMap<String, OwnedVariant>,
    ) -> Option<Self> {
        try_icon_from_hints(hints).or_else(|| {
            icon_candidates(app_name, app_icon, hints)
                .find_map(resolve_candidate)
                .map(Self::from_path)
        })
    }

    fn from_path(path: PathBuf) -> Self {
        let is_svg = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("svg"));
        if is_svg {
            Self::Svg(svg::Handle::from_path(path))
        } else {
            Self::Image(image::Handle::from_path(path))
        }
    }
}

// freedesktop notification hint image data type
#[derive(zbus::OwnedVariant, Debug)]
struct HintImageData {
    width: i32,
    height: i32,
    rowstride: i32,
    has_alpha: bool,
    bits_per_sample: i32,
    channels: i32,
    image_bytes: Vec<u8>,
}

impl HintImageData {
    /// Repacks the rows into the tight RGBA buffer iced expects. The hint is
    /// GdkPixbuf's memory layout: rows start `rowstride` bytes apart, aligned
    /// to 4 bytes (so RGB rows are padded unless the width is a multiple of 4),
    /// and the last row is unpadded.
    fn to_rgba(&self) -> Option<Vec<u8>> {
        let channels = match (self.has_alpha, self.channels) {
            (true, 4) => 4,
            (false, 3) => 3,
            _ => return None,
        };
        let width = usize::try_from(self.width).ok()?;
        let height = usize::try_from(self.height).ok()?;
        let rowstride = usize::try_from(self.rowstride).ok()?;
        let row_len = width.checked_mul(channels)?;
        if self.bits_per_sample != 8 || width == 0 || height == 0 || rowstride < row_len {
            return None;
        }
        let needed = rowstride.checked_mul(height - 1)?.checked_add(row_len)?;
        if self.image_bytes.len() < needed {
            return None;
        }

        let mut rgba = Vec::with_capacity(width.checked_mul(height)?.checked_mul(4)?);
        for row in self.image_bytes.chunks(rowstride).take(height) {
            let row = &row[..row_len];
            if channels == 4 {
                rgba.extend_from_slice(row);
            } else {
                rgba.extend(
                    row.as_chunks::<3>()
                        .0
                        .iter()
                        .flat_map(|[r, g, b]| [*r, *g, *b, 255_u8]),
                );
            }
        }
        Some(rgba)
    }
}

fn try_icon_from_hints(hints: &HashMap<String, OwnedVariant>) -> Option<NotificationIcon> {
    if let Some(image_data) = hints
        .get("image-data")
        .or(hints.get("image_data"))
        .or(hints.get("icon_data"))
        && let Ok(hint_image_data) = HintImageData::try_from(image_data.clone())
        && let Some(bytes) = hint_image_data.to_rgba()
    {
        Some(NotificationIcon::Image(
            iced::advanced::image::Handle::from_rgba(
                hint_image_data.width as u32,
                hint_image_data.height as u32,
                bytes,
            ),
        ))
    } else {
        None
    }
}

const HINT_KEYS: &[&str] = &[
    "image-path",
    "image_path",
    "icon-name",
    "icon_name",
    "desktop-entry",
];

fn icon_candidates<'a>(
    app_name: &'a str,
    app_icon: &'a str,
    hints: &'a HashMap<String, OwnedVariant>,
) -> impl Iterator<Item = String> + 'a {
    std::iter::once(app_icon.to_string())
        .chain(
            HINT_KEYS
                .iter()
                .filter_map(|k| hints.get(*k).and_then(|v| v.clone().try_into().ok())),
        )
        .chain(std::iter::once(app_name.to_string()))
        .map(|s: String| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn resolve_candidate(candidate: String) -> Option<PathBuf> {
    if let Ok(url) = url::Url::parse(&candidate)
        && url.scheme() == "file"
    {
        return url.to_file_path().ok().filter(|p| p.exists());
    }

    if candidate.contains('/') || candidate.starts_with('.') {
        let path = PathBuf::from(&candidate);
        if path.exists() {
            return Some(path);
        }
    }

    let name = candidate.strip_suffix(".desktop").unwrap_or(&candidate);
    xdg_icons::find_icon_path(name)
}

#[derive(Debug, Clone)]
pub struct NotificationsService {
    pub connection: Connection,
}

impl NotificationsService {
    async fn init_service() -> anyhow::Result<(Connection, broadcast::Sender<NotificationEvent>)> {
        let (connection, event_tx) = dbus::NotificationDaemon::start_server().await?;
        Ok((connection, event_tx))
    }

    async fn start_listening(state: State, output: &mut Sender<ServiceEvent<Self>>) -> State {
        match state {
            State::Init => match Self::init_service().await {
                Ok((connection, event_tx)) => {
                    info!("Notifications service initialized");
                    let _ = output
                        .send(ServiceEvent::Init(NotificationsService {
                            connection: connection.clone(),
                        }))
                        .await;
                    State::Active(connection, event_tx)
                }
                Err(err) => {
                    error!("Failed to initialize notifications service: {err}");
                    State::Error
                }
            },
            State::Active(_connection, event_tx) => {
                let rx = event_tx.subscribe();
                let mut stream = BroadcastStream::new(rx);

                while let Some(result) = stream.next().await {
                    match result {
                        Ok(event) => {
                            let _ = output.send(ServiceEvent::Update(event)).await;
                        }
                        Err(e) => {
                            error!("Error receiving notification event: {e}");
                        }
                    }
                }
                error!("Notification event stream ended unexpectedly");
                State::Error
            }
            State::Error => {
                error!("Notifications service error");
                let _ = pending::<u8>().next().await;
                State::Error
            }
        }
    }
}

enum State {
    Init,
    Active(Connection, broadcast::Sender<NotificationEvent>),
    Error,
}

impl ReadOnlyService for NotificationsService {
    type UpdateEvent = NotificationEvent;
    type Error = ();

    fn update(&mut self, _event: NotificationEvent) {}

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Subscription::run_with(TypeId::of::<Self>(), |_| {
            channel(100, async |mut output| {
                let mut state = State::Init;

                loop {
                    state = NotificationsService::start_listening(state, &mut output).await;
                }
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hint(
        width: i32,
        height: i32,
        rowstride: i32,
        channels: i32,
        bytes: Vec<u8>,
    ) -> HintImageData {
        HintImageData {
            width,
            height,
            rowstride,
            has_alpha: channels == 4,
            bits_per_sample: 8,
            channels,
            image_bytes: bytes,
        }
    }

    #[test]
    fn rgb_rows_skip_padding_and_last_row_may_be_unpadded() {
        // 3x2 RGB, rows padded from 9 to 12 bytes, last row unpadded.
        let mut bytes: Vec<u8> = (1..=9).collect();
        bytes.extend([0, 0, 0]);
        bytes.extend(10..=18);
        let rgba = hint(3, 2, 12, 3, bytes).to_rgba().unwrap();
        assert_eq!(
            rgba,
            [
                1, 2, 3, 255, 4, 5, 6, 255, 7, 8, 9, 255, 10, 11, 12, 255, 13, 14, 15, 255, 16, 17,
                18, 255
            ]
        );
    }

    #[test]
    fn rgba_tight_rows_pass_through() {
        let bytes: Vec<u8> = (0..16).collect();
        assert_eq!(hint(2, 2, 8, 4, bytes.clone()).to_rgba().unwrap(), bytes);
    }

    #[test]
    fn rejects_empty_short_or_inconsistent_images() {
        assert!(hint(0, 0, 0, 4, vec![]).to_rgba().is_none());
        assert!(hint(2, 2, 8, 4, vec![0; 15]).to_rgba().is_none());
        assert!(hint(2, 2, 7, 4, vec![0; 16]).to_rgba().is_none());
        assert!(hint(-1, 2, 8, 4, vec![0; 16]).to_rgba().is_none());
        assert!(
            hint(i32::MAX, i32::MAX, i32::MAX, 4, vec![0; 16])
                .to_rgba()
                .is_none()
        );
    }
}
