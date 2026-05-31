//! Clipboard History Module for ashell
//!
//! This module provides a clipboard history feature using **cliphist** as the
//! backend. When the user clicks the clipboard icon on the panel, the module
//! reads the last N entries from cliphist and displays them in a popup menu.
//!
//! # How it works
//!
//! - **On popup open**: `cliphist list | head -N` loads the last N entries.
//!   Text entries are shown directly. Image entries (detected by the
//!   `[[ binary data` prefix) are loaded via `cliphist decode <id>`.
//! - **On entry click**: `cliphist decode <id> | wl-copy` copies the entry back.
//! - **On clear**: `cliphist wipe` clears the entire history.
//!
//! # cliphist list output format
//!
//! Each line: `<id>\t<preview>`
//! - Text: `42\thello world`
//! - Image: `42\t[[ binary data 150 KiB png 800x600 ]]`
//!
//! # Dependencies
//!
//! - `cliphist` — clipboard history manager
//! - `wl-copy` (from wl-clipboard) — for writing content back to the clipboard

use crate::{
    components::icons::{StaticIcon, icon, icon_button},
    components::{ButtonHierarchy, ButtonKind, MenuSize},
    config::ClipboardModuleConfig,
    t,
    theme::use_theme,
};
use iced::{
    Alignment, Element, Length, Subscription, Task,
    widget::{
        button, column, container, image, row, scrollable, text,
    },
};
use log::{debug, error, warn};
use std::collections::VecDeque;

/// Maximum image size in bytes (2 MB). Larger images are skipped.
const MAX_IMAGE_SIZE: usize = 2_000_000;

/// Height of image thumbnails in the popup, in logical pixels.
const THUMBNAIL_HEIGHT: f32 = 48.0;

/// Width of image thumbnails in the popup, in logical pixels.
const THUMBNAIL_WIDTH: f32 = 48.0;

/// Maximum number of characters shown in a text entry preview.
const PREVIEW_TEXT_LENGTH: usize = 80;

/// Pattern that cliphist uses to mark image entries in `list` output.
const BINARY_DATA_PREFIX: &str = "[[ binary data";

// ── Content types ──────────────────────────────────────────────────────────

/// The content of a single clipboard entry.
#[derive(Debug, Clone)]
pub enum ClipboardContent {
    /// Plain-text content with preview.
    Text(String),
    /// PNG-encoded image data (loaded via `cliphist decode`).
    Image(Vec<u8>),
}

impl ClipboardContent {
    /// Returns a short preview string suitable for the popup list.
    ///
    /// Uses char-based truncation to avoid panicking on multi-byte UTF-8
    /// characters (e.g. Cyrillic, CJK) where a byte index would fall
    /// inside a character boundary.
    fn preview_text(&self) -> String {
        match self {
            ClipboardContent::Text(t) => {
                let first_line = t.lines().next().unwrap_or("");
                if first_line.chars().count() > PREVIEW_TEXT_LENGTH {
                    format!(
                        "{}…",
                        first_line.chars().take(PREVIEW_TEXT_LENGTH).collect::<String>()
                    )
                } else if first_line.is_empty() && t.chars().count() > PREVIEW_TEXT_LENGTH {
                    format!(
                        "{}…",
                        t.chars().take(PREVIEW_TEXT_LENGTH).collect::<String>()
                    )
                } else {
                    first_line.to_string()
                }
            }
            ClipboardContent::Image(_) => t!("clipboard-image-entry").to_string(),
        }
    }
}

// ── Clipboard entry ───────────────────────────────────────────────────────

/// A single entry in the clipboard history.
#[derive(Debug, Clone)]
pub struct ClipboardEntry {
    /// cliphist entry ID (numeric, used for `cliphist decode <id>`).
    id: u64,
    /// The clipboard content (text or image).
    content: ClipboardContent,
}

// ── Messages ──────────────────────────────────────────────────────────────

/// Messages handled by the clipboard module.
#[derive(Debug, Clone)]
pub enum Message {
    /// Popup was opened — load clipboard history from cliphist.
    MenuOpened,
    /// Clipboard history was loaded from cliphist (list phase).
    HistoryListed(Vec<(u64, String)>),
    /// An image entry was decoded from cliphist.
    ImageDecoded { index: usize, id: u64, data: Vec<u8> },
    /// Failed to load history from cliphist.
    HistoryLoadFailed(String),
    /// User clicked on a history entry to copy it back to the clipboard.
    CopyEntry(u64),
    /// User clicked the "clear history" button.
    ClearHistory,
    /// History was cleared.
    HistoryCleared,
    /// Copy-to-clipboard task completed.
    EntryCopied,
}

// ── Actions ───────────────────────────────────────────────────────────────

/// Actions returned by `update()`, similar to other ashell modules.
pub enum Action {
    None,
    Command(Task<Message>),
}

// ── Module state ──────────────────────────────────────────────────────────

/// The clipboard history module.
pub struct Clipboard {
    /// Module configuration.
    config: ClipboardModuleConfig,
    /// History of clipboard entries, most recent first.
    history: VecDeque<ClipboardEntry>,
    /// Whether we are currently loading history from cliphist.
    loading: bool,
    /// Pending image entries to decode (index in history, id).
    pending_images: Vec<(usize, u64)>,
}

impl Clipboard {
    /// Creates a new `Clipboard` module with the given configuration.
    pub fn new(config: ClipboardModuleConfig) -> Self {
        debug!("Module created, max_entries={}", config.max_entries);
        Self {
            config,
            history: VecDeque::new(),
            loading: false,
            pending_images: Vec::new(),
        }
    }

    // ── Update ────────────────────────────────────────────────────────────

    /// Processes a message and returns an action.
    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::MenuOpened => {
                debug!("MenuOpened received — loading history...");
                self.loading = true;
                self.pending_images.clear();
                let max_entries = self.config.max_entries;
                Action::Command(Task::perform(
                    list_clipboard_history(max_entries),
                    |result| match result {
                        Ok(entries) => Message::HistoryListed(entries),
                        Err(e) => Message::HistoryLoadFailed(e),
                    },
                ))
            }

            Message::HistoryListed(entries) => {
                debug!("HistoryListed: received {} entries", entries.len());
                self.history.clear();
                self.pending_images.clear();

                for (id, preview) in entries {
                    debug!("  id={}, preview={:?}", id, &preview.chars().take(60).collect::<String>());
                    if preview.starts_with(BINARY_DATA_PREFIX) {
                        // Image entry — store as text placeholder, schedule decode
                        let index = self.history.len();
                        self.history.push_back(ClipboardEntry {
                            id,
                            content: ClipboardContent::Text(preview),
                        });
                        self.pending_images.push((index, id));
                    } else {
                        // Text entry — use directly
                        self.history.push_back(ClipboardEntry {
                            id,
                            content: ClipboardContent::Text(preview),
                        });
                    }
                }

                debug!("Parsed {} entries, {} are images",
                    self.history.len(), self.pending_images.len());

                // Decode image entries in parallel
                if self.pending_images.is_empty() {
                    self.loading = false;
                    debug!("No images to decode, loading complete");
                    Action::None
                } else {
                    let pending = self.pending_images.clone();
                    let tasks: Vec<Task<Message>> = pending
                        .into_iter()
                        .map(|(index, id)| {
                            Task::perform(
                                decode_clipboard_image(id, index),
                                |result| match result {
                                    Ok((index, id, data)) => Message::ImageDecoded { index, id, data },
                                    Err(e) => {
                                        debug!("Failed to decode image: {e}");
                                        // Return a no-op message
                                        Message::EntryCopied
                                    }
                                },
                            )
                        })
                        .collect();
                    debug!("Starting {} image decode tasks", tasks.len());
                    Action::Command(Task::batch(tasks))
                }
            }

            Message::ImageDecoded { index, id, data } => {
                debug!("ImageDecoded: id={}, index={}, {} bytes", id, index, data.len());
                // Replace the text placeholder with actual image data
                if let Some(entry) = self.history.get_mut(index) {
                    if entry.id == id {
                        entry.content = ClipboardContent::Image(data);
                    } else {
                        warn!("id mismatch at index {} (expected {}, got {})", index, entry.id, id);
                    }
                } else {
                    warn!("index {} out of bounds (history len={})", index, self.history.len());
                }

                // Check if all images are decoded
                self.pending_images.retain(|(_, pid)| *pid != id);
                if self.pending_images.is_empty() {
                    self.loading = false;
                    debug!("All images decoded, loading complete");
                }
                Action::None
            }

            Message::HistoryLoadFailed(e) => {
                self.loading = false;
                self.history.clear();
                error!("Failed to load clipboard history: {e}");
                Action::None
            }

            Message::CopyEntry(id) => {
                debug!("CopyEntry: id={}", id);
                Action::Command(Task::perform(
                    copy_entry_to_clipboard(id),
                    |_| Message::EntryCopied,
                ))
            }

            Message::ClearHistory => {
                debug!("ClearHistory");
                Action::Command(Task::perform(
                    clear_clipboard_history(),
                    |result| match result {
                        Ok(()) => Message::HistoryCleared,
                        Err(e) => {
                            error!("Failed to clear clipboard history: {e}");
                            Message::HistoryCleared
                        }
                    },
                ))
            }

            Message::HistoryCleared => {
                self.history.clear();
                debug!("HistoryCleared");
                Action::None
            }

            Message::EntryCopied => Action::None,
        }
    }

    // ── Bar indicator view ────────────────────────────────────────────────

    /// Renders the bar indicator (clipboard icon).
    pub fn view(&self) -> Element<'_, Message> {
        icon(StaticIcon::Copy).into()
    }

    // ── Popup menu view ──────────────────────────────────────────────────

    /// Renders the popup menu with the clipboard history.
    pub fn menu_view(&self) -> Element<'_, Message> {
        let (space, font_size, menu_opacity, radius) =
            use_theme(|t| (t.space, t.font_size, t.menu.opacity, t.radius));

        let has_entries = !self.history.is_empty();

        let content: Element<'_, Message> = if self.loading && !has_entries {
            // Loading state (no entries yet)
            container(text(t!("clipboard-loading")).size(font_size.md))
                .width(Length::Fill)
                .center_x(Length::Fill)
                .padding(space.xxl)
                .into()
        } else if !has_entries {
            // Empty state
            container(text(t!("clipboard-empty")).size(font_size.md))
                .width(Length::Fill)
                .center_x(Length::Fill)
                .padding(space.xxl)
                .into()
        } else {
            // Has entries — render them
            let entries: Vec<Element<'_, Message>> = self
                .history
                .iter()
                .map(|entry| {
                    let preview = entry.content.preview_text();
                    let entry_id = entry.id;

                    let content_row: Element<'_, Message> = match &entry.content {
                        ClipboardContent::Text(t) => {
                            // Check if this is an image placeholder that hasn't been decoded yet
                            if t.starts_with(BINARY_DATA_PREFIX) {
                                // Show as "image" with generic icon (still loading or failed)
                                row!(
                                    container(icon(StaticIcon::Copy).size(font_size.sm))
                                        .center_x(Length::Fixed(space.xl))
                                        .center_y(Length::Fixed(THUMBNAIL_HEIGHT)),
                                    text(t!("clipboard-image-entry"))
                                        .size(font_size.sm)
                                        .width(Length::Fill),
                                )
                                .align_y(Alignment::Center)
                                .spacing(space.xs)
                                .into()
                            } else {
                                row!(
                                    container(icon(StaticIcon::Copy).size(font_size.sm))
                                        .center_x(Length::Fixed(space.xl))
                                        .center_y(Length::Fixed(THUMBNAIL_HEIGHT)),
                                    text(preview)
                                        .size(font_size.sm)
                                        .wrapping(text::Wrapping::WordOrGlyph)
                                        .width(Length::Fill),
                                )
                                .align_y(Alignment::Center)
                                .spacing(space.xs)
                                .into()
                            }
                        }
                        ClipboardContent::Image(data) => {
                            let handle = image::Handle::from_bytes(data.clone());
                            row!(
                                container(
                                    image(handle)
                                        .width(Length::Fixed(THUMBNAIL_WIDTH))
                                        .height(Length::Fixed(THUMBNAIL_HEIGHT))
                                )
                                .center_x(Length::Fixed(space.xl))
                                .center_y(Length::Fixed(THUMBNAIL_HEIGHT)),
                                text(preview)
                                    .size(font_size.sm)
                                    .wrapping(text::Wrapping::WordOrGlyph)
                                    .width(Length::Fill),
                            )
                            .align_y(Alignment::Center)
                            .spacing(space.xs)
                            .into()
                        }
                    };

                    button(container(content_row).padding([space.xxs, space.xs]))
                        .on_press(Message::CopyEntry(entry_id))
                        .width(Length::Fill)
                        .style(move |theme: &iced::Theme, status| {
                            let background = match status {
                                button::Status::Hovered => {
                                    theme.extended_palette().background.strong.color.scale_alpha(menu_opacity)
                                }
                                _ => {
                                    theme.extended_palette().background.weak.color.scale_alpha(menu_opacity)
                                }
                            };
                            button::Style {
                                background: Some(background.into()),
                                text_color: theme.palette().text,
                                border: iced::Border::default().rounded(radius.lg),
                                ..Default::default()
                            }
                        })
                        .into()
                })
                .collect();

            column(entries)
                .spacing(space.xxs)
                .padding(iced::Padding::default().right(space.md).left(space.xs))
                .into()
        };

        column!(
            row!(
                text(t!("clipboard-heading"))
                    .width(Length::Fill)
                    .size(font_size.lg),
                has_entries.then(|| {
                    icon_button(StaticIcon::Delete)
                        .on_press(Message::ClearHistory)
                        .kind(ButtonKind::Transparent)
                        .hierarchy(ButtonHierarchy::Danger)
                }),
            ),
            container(scrollable(content)).max_height(400),
        )
        .width(MenuSize::Medium)
        .spacing(space.sm)
        .into()
    }

    // ── Subscription ─────────────────────────────────────────────────────

    /// Returns a no-op subscription. This module loads history on demand
    /// when the popup is opened — no background monitoring needed.
    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }
}

// ── cliphist interaction ──────────────────────────────────────────────────

/// Runs `cliphist list` and parses the output.
///
/// Returns a list of (id, preview) tuples. Image entries have previews
/// starting with `[[ binary data`.
///
/// Uses `std::process::Command` via `tokio::task::spawn_blocking` to avoid
/// any issues with the async process spawning.
async fn list_clipboard_history(max_entries: usize) -> Result<Vec<(u64, String)>, String> {
    debug!("list_clipboard_history: starting (max_entries={})", max_entries);

    let result = tokio::task::spawn_blocking(move || {
        debug!("spawn_blocking: running 'cliphist list'...");

        let output = match std::process::Command::new("cliphist")
            .arg("list")
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                let msg = format!("failed to spawn 'cliphist list': {e}");
                error!("{msg}");
                return Err(msg);
            }
        };

        debug!("cliphist list: exit={:?}, stdout={} bytes, stderr={} bytes",
            output.status.code(), output.stdout.len(), output.stderr.len());

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let msg = format!("cliphist list failed (exit {:?}): {}", output.status.code(), stderr.trim());
            error!("{msg}");
            return Err(msg);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        debug!("cliphist list raw output (first 500 chars):\n{}", stdout.chars().take(500).collect::<String>());

        let mut entries = Vec::new();

        for (line_num, line) in stdout.lines().enumerate() {
            // max_entries == 0 means "unlimited"
            if max_entries > 0 && line_num >= max_entries {
                break;
            }
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // cliphist format: "<id>\t<preview>"
            // Try tab first, then spaces as fallback
            if let Some((id_str, preview)) = line.split_once('\t') {
                if let Ok(id) = id_str.trim().parse::<u64>() {
                    debug!("  parsed entry: id={}, preview={:?}", id, &preview.chars().take(40).collect::<String>());
                    entries.push((id, preview.to_string()));
                } else {
                    debug!("  SKIPPING line {} (non-numeric id): {:?}", line_num, line);
                }
            } else {
                // Fallback: no tab separator found.
                // Try splitting by whitespace: "<number> <text>"
                debug!("  SKIPPING line {} (no tab separator): {:?}", line_num, line);

                // Try finding the first space and treating everything before as ID
                if let Some(space_pos) = line.find(|c: char| c.is_whitespace()) {
                    let id_str = &line[..space_pos];
                    let rest = line[space_pos..].trim_start();
                    if let Ok(id) = id_str.parse::<u64>() {
                        if !rest.is_empty() {
                            debug!("  FALLBACK parsed: id={}, rest={:?}", id, rest.chars().take(40).collect::<String>());
                            entries.push((id, rest.to_string()));
                        }
                    }
                }
            }
        }

        debug!("list_clipboard_history: returning {} entries", entries.len());
        Ok(entries)
    })
    .await
    .map_err(|e| {
        let msg = format!("spawn_blocking panicked: {e}");
        error!("{msg}");
        msg
    })?;

    result
}

/// Decodes an image entry from cliphist by running `cliphist decode <id>`.
///
/// Returns the raw image bytes (PNG, JPEG, etc.).
async fn decode_clipboard_image(id: u64, index: usize) -> Result<(usize, u64, Vec<u8>), String> {
    debug!("decode_clipboard_image: id={}, index={}", id, index);

    let result = tokio::task::spawn_blocking(move || {
        let id_str = id.to_string();

        let output = match std::process::Command::new("cliphist")
            .args(["decode", &id_str])
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                let msg = format!("failed to spawn 'cliphist decode {id}': {e}");
                error!("{msg}");
                return Err(msg);
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let msg = format!("cliphist decode {id} failed (exit {:?}): {}", output.status.code(), stderr.trim());
            error!("{msg}");
            return Err(msg);
        }

        let data = output.stdout;
        if data.is_empty() {
            let msg = format!("cliphist decode {id} returned empty data");
            error!("{msg}");
            return Err(msg);
        }

        if data.len() > MAX_IMAGE_SIZE {
            let msg = format!("cliphist decode {id} returned {} bytes (max {})", data.len(), MAX_IMAGE_SIZE);
            warn!("{msg}");
            return Err(msg);
        }

        debug!("Decoded image for id {id}: {} bytes", data.len());
        Ok((index, id, data))
    })
    .await
    .map_err(|e| {
        let msg = format!("spawn_blocking panicked for decode: {e}");
        error!("{msg}");
        msg
    })?;

    result
}

/// Copies a clipboard entry back to the active clipboard using
/// `cliphist decode <id>` piped into `wl-copy`.
async fn copy_entry_to_clipboard(id: u64) {
    debug!("copy_entry_to_clipboard: id={}", id);

    let result = tokio::task::spawn_blocking(move || {
        let id_str = id.to_string();

        // Decode the entry from cliphist
        let decode_output = match std::process::Command::new("cliphist")
            .args(["decode", &id_str])
            .output()
        {
            Ok(o) if o.status.success() && !o.stdout.is_empty() => o,
            Ok(o) => {
                debug!("cliphist decode {} failed (exit {:?})",
                    id, o.status.code());
                return;
            }
            Err(e) => {
                debug!("cliphist decode error: {e}");
                return;
            }
        };

        let data = decode_output.stdout;

        // Detect content type from the data
        let mime_type = detect_mime_type(&data);

        // Write to wl-copy
        let mut cmd = std::process::Command::new("wl-copy");
        if let Some(ref mt) = mime_type {
            cmd.args(["-t", mt]);
        }

        let mut child = match cmd
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                debug!("failed to spawn wl-copy: {e}");
                return;
            }
        };

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            if let Err(e) = stdin.write_all(&data) {
                debug!("failed to write to wl-copy stdin: {e}");
                return;
            }
            drop(stdin); // Close pipe to signal EOF
        }

        match child.wait() {
            Ok(status) => {
                debug!("wl-copy exited with {:?}", status.code());
            }
            Err(e) => {
                debug!("wl-copy wait error: {e}");
            }
        }
    })
    .await;

    if let Err(e) = result {
        debug!("copy_entry_to_clipboard spawn_blocking error: {e}");
    }
}

/// Detects the MIME type of binary data by checking magic bytes.
fn detect_mime_type(data: &[u8]) -> Option<String> {
    if data.len() < 4 {
        return None;
    }

    // PNG: 89 50 4E 47
    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        return Some("image/png".to_string());
    }
    // JPEG: FF D8 FF
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg".to_string());
    }
    // GIF: GIF87a or GIF89a
    if data.starts_with(b"GIF8") {
        return Some("image/gif".to_string());
    }
    // WebP: RIFF....WEBP
    if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        return Some("image/webp".to_string());
    }
    // BMP: BM
    if data.starts_with(b"BM") {
        return Some("image/bmp".to_string());
    }
    // Default: treat as text (no special MIME type needed for wl-copy)
    None
}

/// Clears the entire clipboard history using `cliphist wipe`.
async fn clear_clipboard_history() -> Result<(), String> {
    debug!("clear_clipboard_history: running 'cliphist wipe'...");

    let result = tokio::task::spawn_blocking(move || {
        let output = match std::process::Command::new("cliphist")
            .arg("wipe")
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                let msg = format!("failed to spawn 'cliphist wipe': {e}");
                error!("{msg}");
                return Err(msg);
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let msg = format!("cliphist wipe failed: {}", stderr.trim());
            error!("{msg}");
            return Err(msg);
        }

        debug!("cliphist wipe succeeded");
        Ok(())
    })
    .await
    .map_err(|e| {
        let msg = format!("spawn_blocking panicked for wipe: {e}");
        error!("{msg}");
        msg
    })?;

    result
}
