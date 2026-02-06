---
sidebar_position: 12
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

## Menu

The menu shows all active media players with playback controls:

- Previous, Play/Pause, and Next buttons
- Volume slider (if supported by the player)

## Example

```toml
[media_player]
max_title_length = 50
indicator_format = "Icon"
```
