# Clipboard History Module

## Overview

The **Clipboard History** module for ashell provides a convenient way to access and manage
recently copied content directly from the Wayland bar. It uses **cliphist** as the backend
to read clipboard history on demand, and displays entries in a popup menu.

## Features

- **On-demand loading** — reads history from cliphist when popup opens (no background polling)
- **Text support** — displays plain-text clipboard entries with a preview
- **Image support** — displays image entries as thumbnails (48×48 px) loaded via `cliphist decode`
- **Click-to-copy** — clicking any history entry copies it back to the clipboard via `wl-copy`
- **Configurable history size** — default 8 entries, adjustable in config
- **Clear history** — one-click button to erase all entries via `cliphist wipe`
- **Bilingual UI** — English and Russian translations included
- **Fallback parsing** — supports both tab and space separators in cliphist output

## Dependencies

| Dependency | Package (Debian) | Purpose |
|---|---|---|
| `cliphist` | `cliphist` | Reading/storing clipboard history |
| `wl-copy` | `wl-clipboard` | Writing content back to clipboard |

### Setting up cliphist

To enable automatic clipboard history storage, run in your compositor's startup:

```bash
wl-paste --watch cliphist store &
```

This watches the Wayland clipboard and stores every copy in cliphist's cache.
Without this, `cliphist list` will return empty results.

## Configuration

Add `Clipboard` to your module list in `~/.config/ashell/config.toml`:

```toml
[modules]
right = ["Clipboard", "Tempo", "Privacy", "Settings"]
```

### Module-specific settings

```toml
[clipboard]
max_entries = 8   # Maximum number of history entries (default: 8)
```

| Option | Type | Default | Description |
|---|---|---|---|
| `max_entries` | `usize` | `8` | Maximum number of clipboard entries to show |

## Usage

### Opening the popup

Click the **clipboard icon** (📋) on the bar to open the clipboard history popup.
The module reads the latest entries from cliphist when the popup opens.

### Copying an entry

Click any entry in the popup to copy its content back to the system clipboard.
For text entries, `cliphist decode <id> | wl-copy` is used.
For image entries, `cliphist decode <id>` is piped to `wl-copy -t image/png`.

### Clearing history

Click the **trash icon** (🗑) in the popup header to clear all stored entries.
This runs `cliphist wipe` to erase the entire cliphist cache.

### Text entries

Text entries display a preview of the first line (up to 80 characters), truncated
with an ellipsis (`…`) if longer. A small copy icon is shown to the left of the text.

### Image entries

Image entries are detected by the `[[ binary data` prefix in `cliphist list` output.
They are decoded via `cliphist decode <id>` and displayed as a 48×48 px thumbnail.
Images larger than 2 MB are not loaded to prevent excessive memory usage.

## Architecture

### File structure

```
src/modules/clipboard.rs    — Main module implementation
src/config.rs               — ClipboardModuleConfig definition
src/modules/mod.rs           — Module routing (view/subscription)
src/app.rs                   — Integration into App struct
src/components/menu.rs       — MenuType::Clipboard variant
i18n/en-US/ashell.ftl        — English translations
i18n/ru-RU/ashell.ftl        — Russian translations
```

### Data flow

```
┌─────────────────┐   click icon   ┌──────────────────┐
│  User           │ ─────────────► │  MenuOpened      │
│                 │                │  message          │
└─────────────────┘                └───────┬──────────┘
                                           │ Task::perform
                                           ▼
                                   ┌──────────────────┐
                                   │  cliphist list   │
                                   │  (spawn_blocking)│
                                   └───────┬──────────┘
                                           │ HistoryListed
                                           ▼
                                   ┌──────────────────┐
                                   │  Clipboard       │
                                   │  (update)        │
                                   │  - parse entries │
                                   │  - detect images │
                                   └───────┬──────────┘
                                           │
                             ┌─────────────┼─────────────┐
                             ▼             ▼             ▼
                       CopyEntry      ClearHistory    view()
                             │             │             │
                             ▼             │             ▼
                    ┌──────────────┐       │     ┌──────────────┐
                    │  cliphist    │       │     │  Popup Menu  │
                    │  decode +    │       │     │  (menu_view) │
                    │  wl-copy     │       │     └──────────────┘
                    └──────────────┘       │
                                           ▼
                                    cliphist wipe
```

### Key types

| Type | Description |
|---|---|
| `ClipboardContent` | Enum: `Text(String)` or `Image(Vec<u8>)` |
| `ClipboardEntry` | An entry with a cliphist `id` and `content` |
| `Message` | Module messages: `MenuOpened`, `HistoryListed`, `ImageDecoded`, `CopyEntry`, `ClearHistory`, etc. |
| `Action` | Update result: `None` or `Command(Task)` |
| `Clipboard` | Main struct holding config, history (`VecDeque`), loading state, and pending image decodes |

### Implementation details

1. **On-demand loading**: Unlike a polling approach, the module only reads cliphist when
   the user opens the popup. This eliminates CPU overhead when the popup is closed.

2. **spawn_blocking**: All external commands (`cliphist list`, `cliphist decode`, `wl-copy`)
   are executed via `std::process::Command` inside `tokio::task::spawn_blocking`. This
   avoids blocking the iced runtime while ensuring reliable process spawning.

3. **Image decoding**: Image entries are initially stored as text placeholders. After
   `HistoryListed`, separate `Task::perform` calls decode each image in parallel via
   `cliphist decode`. When decoded, the placeholder is replaced with `ClipboardContent::Image`.

4. **Fallback parsing**: The `cliphist list` output format is `<id>\t<preview>`. If no tab
   is found, the parser falls back to splitting on the first whitespace character, making
   it resilient to different cliphist versions or output formatting.

## Limitations

1. **cliphist required** — The module requires `cliphist` to be installed and configured
   with `wl-paste --watch cliphist store` running. Without this, the popup will show
   "Clipboard is empty".

2. **Image size limit** — Images larger than 2 MB are skipped during decode. This
   prevents excessive memory usage.

3. **No in-process monitoring** — The module does not monitor the clipboard itself.
   It relies entirely on cliphist's external storage.

4. **Debug logging** — The module uses `eprintln!` for debug output (prefixed with
   `[clipboard]`). This helps diagnose issues but should be removed or converted to
   `log::debug!` for production.
