use iced::widget::{button, column, container, image, row, scrollable, text, Row};
use iced::window;
use iced::{Element, Length, Task};
use crate::app::Message as AppMessage;
use crate::components::menu::MenuType;

#[derive(Debug, Clone)]
pub enum ClipboardMessage {
    Toggle,
    HistoryLoaded(Vec<HistoryEntry>),
    EntrySelected { id: String, mime: String },
}

#[derive(Debug, Clone)]
pub enum HistoryEntry {
    Text {
        id: String,
        snippet: String,
        mime: String,
    },
    Image {
        id: String,
        thumbnail: image::Handle,
        mime: String,
    },
}

pub struct ClipboardModule {
    entries: Vec<HistoryEntry>,
    visible: bool,
}

impl ClipboardModule {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            visible: false,
        }
    }

    pub fn update(&mut self, message: ClipboardMessage) -> Task<AppMessage> {
        match message {
            ClipboardMessage::Toggle => {
                if self.visible {
                    self.visible = false;
                    Task::none()
                } else {
                    self.visible = true;
                    // Запускаем асинхронную загрузку истории
                    Task::run(load_history()).map(|entries| {
                        AppMessage::Clipboard(ClipboardMessage::HistoryLoaded(entries))
                    })
                }
            }
            ClipboardMessage::HistoryLoaded(entries) => {
                self.entries = entries;
                Task::none()
            }
            ClipboardMessage::EntrySelected { id, mime } => {
                self.visible = false;
                // Копируем в буфер обмена через cliphist + wl-copy
                Task::run(copy_to_clipboard(id, mime)).map(|_| AppMessage::Clipboard(ClipboardMessage::Nothing))
            }
            _ => Task::none(), // для сообщения Nothing (если добавите)
        }
    }

    pub fn view(&self, is_menu: bool) -> Element<ClipboardMessage> {
        let toggle_button = button("󰅍").on_press(ClipboardMessage::Toggle);

        if is_menu {
            // Вид только для меню (список записей)
            if self.entries.is_empty() {
                return container(text("Нет истории")).into();
            }
            let mut list = column![].spacing(4).padding(8);
            for entry in &self.entries {
                let row = match entry {
                    HistoryEntry::Text { snippet, .. } => {
                        Row::new()
                            .push(text("📄").size(14))
                            .push(text(snippet))
                            .spacing(4)
                    }
                    HistoryEntry::Image { thumbnail, .. } => {
                        Row::new()
                            .push(image(thumbnail.clone()).width(32).height(32))
                            .push(text("Изображение").size(14))
                            .spacing(4)
                    }
                };
                let (id, mime) = match entry {
                    HistoryEntry::Text { id, mime, .. } => (id.clone(), mime.clone()),
                    HistoryEntry::Image { id, mime, .. } => (id.clone(), mime.clone()),
                };
                list = list.push(
                    button(row)
                        .on_press(ClipboardMessage::EntrySelected { id, mime })
                        .style(button::text)
                        .width(Length::Fill),
                );
            }
            container(
                scrollable(list)
                    .width(Length::Fixed(320.0))
                    .max_height(400.0),
            )
            .style(container::rounded_box)
            .into()
        } else {
            // Обычный вид на панели (только кнопка)
            container(toggle_button).into()
        }
    }
}

// Асинхронная загрузка истории (8 последних записей) с миниатюрами
async fn load_history() -> Vec<HistoryEntry> {
    // 1. Получить список: id, mime-тип
    let list_output = tokio::task::spawn_blocking(|| {
        std::process::Command::new("cliphist")
            .arg("list")
            .arg("--max-items")
            .arg("8")
            .output()
    })
    .await
    .ok()
    .and_then(|r| r.ok())
    .and_then(|o| String::from_utf8(o.stdout).ok());

    let Some(list_str) = list_output else {
        return Vec::new();
    };

    let entries_raw: Vec<(String, String)> = list_str
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(2, '\t');
            let id = parts.next()?.to_string();
            let mime = parts.next()?.to_string();
            Some((id, mime))
        })
        .collect();

    let mut entries = Vec::new();

    for (id, mime) in entries_raw {
        if mime.starts_with("image/") {
            // Загружаем изображение и создаём миниатюру
            if let Ok(entry) = load_image_entry(id.clone(), mime.clone()).await {
                entries.push(entry);
            } else {
                // Fallback: показываем как текст-заглушку
                entries.push(HistoryEntry::Text {
                    id,
                    snippet: "[Не удалось загрузить изображение]".to_string(),
                    mime,
                });
            }
        } else {
            // Для текста берём первые 100 символов
            let snippet = match tokio::task::spawn_blocking(move || {
                std::process::Command::new("cliphist")
                    .arg("decode")
                    .arg(&id)
                    .output()
            })
            .await
            .ok()
            .and_then(|r| r.ok())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            {
                Some(full) => {
                    let short: String = full.chars().take(100).collect();
                    if full.len() > 100 { short + "…" } else { short }
                }
                None => "[Ошибка чтения]".to_string(),
            };
            entries.push(HistoryEntry::Text { id, snippet, mime });
        }
    }
    entries
}

async fn load_image_entry(id: String, mime: String) -> Result<HistoryEntry, ()> {
    let bytes = tokio::task::spawn_blocking(move || {
        std::process::Command::new("cliphist")
            .arg("decode")
            .arg(&id)
            .output()
    })
    .await
    .map_err(|_| ())?
    .map_err(|_| ())?;

    let img = image::load_from_memory(&bytes.stdout).map_err(|_| ())?;
    let thumbnail_size = 32;
    let thumbnail = img.thumbnail(thumbnail_size, thumbnail_size);
    let mut rgba_bytes = thumbnail.to_rgba8().into_vec();
    let handle = image::Handle::from_rgba(thumbnail_size, thumbnail_size, rgba_bytes);
    Ok(HistoryEntry::Image {
        id,
        thumbnail: handle,
        mime,
    })
}

async fn copy_to_clipboard(id: String, mime: String) {
    let result = tokio::task::spawn_blocking(move || {
        // Получить содержимое из cliphist
        let decode = std::process::Command::new("cliphist")
            .arg("decode")
            .arg(&id)
            .stdout(std::process::Stdio::piped())
            .output()?;
        // Передать в wl-copy с нужным MIME-типом
        let mut child = std::process::Command::new("wl-copy")
            .arg("--type")
            .arg(&mime)
            .stdin(std::process::Stdio::piped())
            .spawn()?;
        std::io::Write::write_all(&mut child.stdin.take().unwrap(), &decode.stdout)?;
        child.wait()?;
        Ok::<_, std::io::Error>(())
    })
    .await;
    if let Err(e) = result {
        eprintln!("Ошибка копирования в буфер: {e}");
    }
}
