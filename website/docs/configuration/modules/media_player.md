---
sidebar_position: 4
---

# Media Player

This module displays the current media playback status in the status bar. It only appears when at least one media player is active.

You can configure the max media title length after which the title will be truncated
using the `max_title_length` field (default: `100`).

### Indicator Format

The tray indicator can show configured metadata text, the media icon together
with that text, or only the media icon. Configure this with the
`indicator_format` field:

| Value          | Description                                                |
| -------------- | ---------------------------------------------------------- |
| `Text`         | Displays only the configured text in the status bar.       |
| `IconAndText`  | Displays the icon followed by the text (default).          |
| `Icon`         | Displays only the media icon in the status bar.            |

Use `Icon` if you want a compact indicator or have limited space.

### Indicator Fields

When `indicator_format` is `Text` or `IconAndText`, configure the metadata
fields shown in the status bar with `indicator_fields`
(default: `["Artist", "Title"]`).

Available fields:

- `Artist`
- `Title`
- `Album`

Fields are joined with ` - `. For example, use `["Title"]` to show only the
current song title. An empty list uses the default fields.

### Visualizer

The module can render an optional audio visualizer powered by
[CAVA](https://github.com/karlstav/cava). It shows animated bars next to the
indicator, but only while a player is actively playing. Enable it with the
`show_visualizer` field (default: `false`):

```toml
[media_player]
show_visualizer = true
```

The bars are coloured with a gradient built from the active theme palette
(`primary` at the bottom, `warning` in the middle, `danger` at the peak). In the
media menu, the visualizer also fills the background of the card that is
currently playing.

CAVA visualizes the system audio output, not the individual stream of the
active player (MPRIS carries only metadata and playback controls, not audio
samples). The visualizer is therefore shown only while the active player is
playing, and `cava` is started on demand and stopped again while playback is
paused.

> **Requires** `cava` to be installed and available on your `$PATH`. If `cava`
> is missing the visualizer stays hidden.

## Menu

The menu shows all active media players with playback controls:

- Previous, Play/Pause, and Next buttons
- Volume slider (if supported by the player)

## Example

```toml
[media_player]
max_title_length = 50
indicator_format = "Text"
indicator_fields = ["Title"]
show_visualizer = true
```
