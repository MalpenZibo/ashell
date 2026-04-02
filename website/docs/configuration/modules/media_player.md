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

## Visualizer

The media player module includes an optional audio visualizer powered by [CAVA](https://github.com/karlstav/cava). It renders animated bars in the status bar indicator, visible only while a player is actively playing.

> **Requires** `cava` to be installed and available on your `$PATH`.

Enable it with:

```toml
[media_player]
show_visualizer = true
```

### Visualizer Options

| Field                    | Type    | Default    | Description                                              |
| ------------------------ | ------- | ---------- | -------------------------------------------------------- |
| `show_visualizer`        | bool    | `false`    | Enable or disable the visualizer.                        |
| `visualizer_bar_count`   | integer | `8`        | Number of frequency bars to display.                     |
| `visualizer_framerate`   | integer | `60`       | Target framerate for CAVA output.                        |
| `visualizer_padding`     | integer | `3`        | Horizontal padding around the visualizer canvas (px).    |
| `visualizer_color`       | color   | `"Text"`   | Bar colour — see [Colour](#colour) below.                |
| `visualizer_channels`    | string  | `"Stereo"` | `"Stereo"` or `"Mono"`.                                  |
| `visualizer_mono_option` | string  | `"Average"`| Mono mix mode: `"Average"`, `"Left"`, or `"Right"`.      |

### Colour

`visualizer_color` accepts several formats:

**Theme palette names:**

| Value       | Description                          |
| ----------- | ------------------------------------ |
| `"Text"`    | Uses the theme text colour (default).|
| `"Primary"` | Uses the theme primary colour.       |
| `"Success"` | Uses the theme success colour.       |
| `"Danger"`  | Uses the theme danger colour.        |

**Hex code:**

```toml
visualizer_color = "#a6e3a1"
```

**Per-bar gradient** (colour depends on bar height — low frequencies/quiet levels show `low`, loud/high levels show `high`):

```toml
visualizer_color = { low = "#a6e3a1", mid = "#f9e2af", high = "#f38ba8" }
```

`mid` is optional. When omitted the gradient interpolates directly between `low` and `high`.

### Channels

By default CAVA outputs stereo, which mirrors the frequency spectrum (bass in the centre). Switch to mono for the classic left-to-right low-to-high layout:

```toml
visualizer_channels = "Mono"
visualizer_mono_option = "Average"  # "Average" | "Left" | "Right"
```

## Menu

The menu shows all active media players with playback controls:

- Previous, Play/Pause, and Next buttons
- Volume slider (if supported by the player)

## Example

```toml
[media_player]
max_title_length = 50
indicator_format = "IconAndTitle"
show_visualizer = true
visualizer_bar_count = 12
visualizer_framerate = 60
visualizer_padding = 3
visualizer_channels = "Mono"
visualizer_mono_option = "Average"
visualizer_color = { low = "#a6e3a1", mid = "#f9e2af", high = "#f38ba8" }
```
