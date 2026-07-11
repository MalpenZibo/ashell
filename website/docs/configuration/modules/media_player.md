---
sidebar_position: 4
---

# Media Player

This module displays the current media playback status in the status bar. It only appears when at least one media player is active.

You can configure the max text length after which the indicator and menu text
will be truncated using the `max_text_length` field (default: `100`).

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
[CAVA](https://github.com/karlstav/cava). It animates only while a player is
actively playing. Two independent settings control it.

`indicator_visualizer` places the visualizer in the status bar indicator. Omit
the field to disable it.

| Value        | Description                                                    |
| ------------ | -------------------------------------------------------------- |
| `Background` | Draws the bars as a background behind the indicator content.  |
| `Before`     | Draws the bars before (to the left of) the content.           |
| `After`      | Draws the bars after (to the right of) the content.           |

`menu_visualizer` (bool, default `false`) draws the bars as the background of
the currently playing card in the media menu.

```toml
[media_player]
indicator_visualizer = "Background"
menu_visualizer = true
```

The bars are coloured with a gradient built from the active theme palette
(`primary` at the bottom, `warning` in the middle, `danger` at the peak).

CAVA visualizes the system audio output, not the individual stream of the
active player (MPRIS carries only metadata and playback controls, not audio
samples). The visualizer is therefore shown only while a player is playing,
and `cava` is started on demand and stopped again while playback is paused.

> **Requires** `cava` to be installed and available on your `$PATH`. If `cava`
> is missing the visualizer stays hidden.

## Menu

The menu shows all active media players with playback controls:

- Previous, Play/Pause, and Next buttons
- Volume slider (if supported by the player)

## Example

```toml
[media_player]
max_text_length = 50
indicator_format = "Text"
indicator_fields = ["Title"]
indicator_visualizer = "Background"
menu_visualizer = true
```
