---
sidebar_position: 4
---

# Media Player

This module displays the current media playback status in the status bar. It only appears when at least one media player is active.

You can configure the max media title length after which the title will be truncated
using the `max_title_length` field (default: `100`).

### Indicator Format

The tray indicator can show the current title, the media icon together with the
current title, or only the media icon. Configure this with the
`indicator_format` field:

| Value          | Description                                                |
| -------------- | ---------------------------------------------------------- |
| `Title`        | Displays only the current title in the status bar.         |
| `IconAndTitle` | Displays the icon followed by the current title (default). |
| `Icon`         | Displays only the media icon in the status bar.            |

Use `Icon` if you want a compact indicator or have limited space.

### Indicator Fields

When `indicator_format` is `Title` or `IconAndTitle`, configure the metadata
fields shown in the status bar with `indicator_fields`
(default: `["Artist", "Title"]`).

Available fields:

- `Artist`
- `Title`
- `Album`

Fields are joined with ` - `. For example, use `["Title"]` to show only the
current song title.

## Menu

The menu shows all active media players with playback controls:

- Metadata fields: Title, Artist, and Album
- Cover art
- Previous, Play/Pause, and Next buttons
- Volume slider (if supported by the player)

Configure visible menu sections with `menu_fields`
(default: `["Title", "Artist", "Album", "Cover", "Controls", "Volume"]`).

Available fields:

- `Title`
- `Artist`
- `Album`
- `Cover`
- `Controls`
- `Volume`

## Example

```toml
[media_player]
max_title_length = 50
indicator_format = "Title"
indicator_fields = ["Title"]
menu_fields = ["Title", "Controls"]
```
