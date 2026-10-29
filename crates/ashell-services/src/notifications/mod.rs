//! The freedesktop notification daemon. ashell owns
//! `org.freedesktop.Notifications` on the session bus: what applications send
//! there is served from here, and `events()` reports each notification as it
//! arrives and each one that closes.

use crate::{
    stream::channel,
    xdg_icons::{self, IconFile},
};
use futures::{SinkExt, Stream};
use log::{info, warn};
use std::{collections::HashMap, path::PathBuf, time::Duration};
use tokio::sync::broadcast::error::RecvError;
use zbus::{Connection, OwnedVariant};

mod dbus;

use dbus::NotificationDaemon;
pub use dbus::{CloseGuard, DEFAULT_ACTION_KEY, Notification, NotificationEvent, Urgency};

const RETRY_DELAY: Duration = Duration::from_secs(5);

/// Where a notification's icon comes from. Decoding is left to the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationIcon {
    /// Pixels sent along in the `image-data` hint: RGBA, 8 bits per channel,
    /// `width * height * 4` bytes.
    Rgba {
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    },
    /// An SVG file.
    Svg(PathBuf),
    /// Any other image file.
    Image(PathBuf),
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
        match IconFile::from_path(path) {
            IconFile::Svg(path) => Self::Svg(path),
            IconFile::Image(path) => Self::Image(path),
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
    /// Repacks the rows into the tight RGBA buffer a UI expects. The hint is
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
        Some(NotificationIcon::Rgba {
            width: hint_image_data.width as u32,
            height: hint_image_data.height as u32,
            pixels: bytes,
        })
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
    xdg_icons::theme_icon_path(name)
}

#[derive(Debug, Clone)]
pub struct Notifications {
    connection: Connection,
}

impl Notifications {
    /// Serves `org.freedesktop.Notifications` on the session bus.
    pub async fn connect() -> anyhow::Result<Self> {
        let connection = NotificationDaemon::start_server().await?;
        info!("Notifications service initialized");

        Ok(Self { connection })
    }

    /// The connection that serves the notification daemon, for callers that
    /// need it to resolve a sender's pid.
    pub fn connection(&self) -> Connection {
        self.connection.clone()
    }

    /// Yields each notification as it arrives, then every close.
    pub fn events(&self) -> impl Stream<Item = NotificationEvent> + Send + 'static {
        let connection = self.connection.clone();

        channel(100, |mut output| async move {
            loop {
                let mut events = match NotificationDaemon::events(&connection).await {
                    Ok(events) => events,
                    Err(err) => {
                        warn!("Failed to listen for notification events: {err}");
                        tokio::time::sleep(RETRY_DELAY).await;
                        continue;
                    }
                };

                loop {
                    match events.recv().await {
                        Ok(event) => {
                            if output.send(event).await.is_err() {
                                return;
                            }
                        }
                        Err(RecvError::Lagged(skipped)) => {
                            warn!("Notification events lagged by {skipped}");
                        }
                        Err(RecvError::Closed) => {
                            warn!("Notification event stream ended unexpectedly");
                            break;
                        }
                    }
                }

                tokio::time::sleep(RETRY_DELAY).await;
            }
        })
    }

    /// Emits `ActionInvoked` for `action_key`, when there is one, then closes
    /// the notification. Returns whether it was closed, false when `guard` no
    /// longer holds because the notification was replaced since.
    pub async fn invoke_and_close(
        &self,
        id: u32,
        action_key: Option<String>,
        guard: CloseGuard,
    ) -> anyhow::Result<bool> {
        NotificationDaemon::invoke_and_close(
            &self.connection,
            id,
            action_key,
            guard,
            CLOSE_REASON_DISMISSED,
        )
        .await
        .map_err(|e| e.context(format!("Failed to close notification id {id}")))
    }

    /// Closes notification `id` as closed by the caller, whatever holds it now.
    pub async fn close(&self, id: u32) -> anyhow::Result<()> {
        NotificationDaemon::invoke_and_close(
            &self.connection,
            id,
            None,
            CloseGuard::Any,
            CLOSE_REASON_CLOSED,
        )
        .await
        .map(|_| ())
        .map_err(|e| e.context(format!("Failed to close notification id {id}")))
    }

    /// Closes each notification in `evicted`, skipping ids replaced since.
    pub async fn evict_notifications(&self, evicted: &[(u32, u64)]) -> anyhow::Result<()> {
        NotificationDaemon::evict_notifications(&self.connection, evicted).await
    }
}

// Close reasons from the freedesktop Notifications specification.
const CLOSE_REASON_DISMISSED: u32 = 2;
const CLOSE_REASON_CLOSED: u32 = 3;

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
    fn rgba_rows_skip_padding() {
        // 2x2 RGBA, rows padded from 8 to 12 bytes.
        let mut bytes: Vec<u8> = (0..8).collect();
        bytes.extend([0; 4]);
        bytes.extend(8..16);
        let rgba = hint(2, 2, 12, 4, bytes).to_rgba().unwrap();
        assert_eq!(rgba, (0..16).collect::<Vec<u8>>());
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
