---
sidebar_position: 4
---

# Media Player

This module displays the current media playback status in the status bar. It only appears when at least one media player is active.

You can configure the max media title length after which the title will be truncated
using the `max_title_length` field (default: `100`).

### Indicator Format

The tray indicator can show either the media icon alone or the icon together with
the current title. Configure this with the `indicator_format` field:

| Value          | Description                                                |
| -------------- | ---------------------------------------------------------- |
| `Icon`         | Displays only the media icon in the status bar.            |
| `IconAndTitle` | Displays the icon followed by the current title (default). |

Use `Icon` if you want a compact indicator or have limited space.

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
(`primary` at the bottom, `warning` in the middle, `danger` at the peak).

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
indicator_format = "Icon"
show_visualizer = true
```
