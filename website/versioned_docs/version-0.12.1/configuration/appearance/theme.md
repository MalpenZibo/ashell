---
sidebar_position: 3
---

# Theme

ashell has no built-in themes: the look is defined entirely by the
[palette](./palette.md). Below are ready-made palettes for popular color
schemes. Copy one into your config file, replacing any color you already set
in the `[appearance]` section.

Only the base colors are set: the `weak` and `strong` variants and the
background shades are generated from them. To fine-tune a theme, switch any
color to the [advanced syntax](./palette.md#advanced-syntax).

## Catppuccin Mocha

```toml
[appearance]

success_color = "#a6e3a1"
warning_color = "#f9e2af"
danger_color = "#f38ba8"
text_color = "#cdd6f4"

workspace_colors = [ "#fab387", "#b4befe", "#cba6f7" ]

[appearance.primary_color]
base = "#fab387"
text = "#1e1e2e"

[appearance.background_color]
base = "#1e1e2e"
weak = "#313244"
strong = "#45475a"
```

## Catppuccin Macchiato

```toml
[appearance]
background_color = "#24273a"
text_color = "#cad3f5"
primary_color = "#8aadf4"
success_color = "#a6da95"
warning_color = "#eed49f"
danger_color = "#ed8796"
```

## Catppuccin Frappé

```toml
[appearance]
background_color = "#303446"
text_color = "#c6d0f5"
primary_color = "#8caaee"
success_color = "#a6d189"
warning_color = "#e5c890"
danger_color = "#e78284"
```

## Catppuccin Latte

```toml
[appearance]
background_color = "#eff1f5"
text_color = "#4c4f69"
primary_color = "#1e66f5"
success_color = "#40a02b"
warning_color = "#df8e1d"
danger_color = "#d20f39"
```

## Tokyo Night - night

```toml
[appearance]

primary_color = "#7aa2f7"
success_color = "#9ece6a"
warning_color = "#e0af68"
danger_color = "#f7768e"
text_color = "#a9b1d6"

workspace_colors = [ "#7aa2f7", "#9ece6a" ]

[appearance.background_color]
base = "#1a1b26"
weak = "#24273a"
strong = "#414868"
```

## Tokyo Night - storm

```toml
[appearance]
background_color = "#24283b"
text_color = "#9aa5ce"
primary_color = "#2ac3de"
success_color = "#9ece6a"
warning_color = "#e0af68"
danger_color = "#f7768e"
```

## Tokyo Night - light

```toml
[appearance]
background_color = "#d5d6db"
text_color = "#565a6e"
primary_color = "#166775"
success_color = "#485e30"
warning_color = "#8f5e15"
danger_color = "#8c4351"
```

## Nord

```toml
[appearance]
success_color = "#a3be8c"
warning_color = "#ebcb8b"
danger_color = "#bf616a"
text_color = "#eceff4"
workspace_colors = [ "#88c0d0", "#81a1c1", "#5e81ac" ]

[appearance.primary_color]
base = "#88c0d0"
text = "#242933"

[appearance.background_color]
base = "#3b4252"
weak = "#434c5e"
strong = "#4c566a"
```

## Dracula

```toml
[appearance]
background_color = "#282a36"
text_color = "#f8f8f2"
primary_color = "#bd93f9"
success_color = "#50fa7b"
warning_color = "#f1fa8c"
danger_color = "#ff5555"
```

## Gruvbox Dark

```toml
[appearance]
background_color = "#282828"
text_color = "#fbf1c7"
primary_color = "#458588"
success_color = "#98971a"
warning_color = "#d79921"
danger_color = "#cc241d"
```

## Gruvbox Light

```toml
[appearance]
background_color = "#fbf1c7"
text_color = "#282828"
primary_color = "#458588"
success_color = "#98971a"
warning_color = "#d79921"
danger_color = "#cc241d"
```

## Solarized Dark

```toml
[appearance]
background_color = "#002b36"
text_color = "#839496"
primary_color = "#2aa198"
success_color = "#859900"
warning_color = "#b58900"
danger_color = "#dc322f"
```

## Solarized Light

```toml
[appearance]
background_color = "#fdf6e3"
text_color = "#657b83"
primary_color = "#2aa198"
success_color = "#859900"
warning_color = "#b58900"
danger_color = "#dc322f"
```

## Kanagawa Wave

```toml
[appearance]
background_color = "#1f1f28"
text_color = "#dcd7ba"
primary_color = "#7fb4ca"
success_color = "#76946a"
warning_color = "#ff9e3b"
danger_color = "#c34043"
```

## Kanagawa Dragon

```toml
[appearance]
background_color = "#181616"
text_color = "#c5c9c5"
primary_color = "#8ba4b0"
success_color = "#8a9a7b"
warning_color = "#ff9e3b"
danger_color = "#c4746e"
```

## Kanagawa Lotus

```toml
[appearance]
background_color = "#f2ecbc"
text_color = "#545464"
primary_color = "#4d699b"
success_color = "#6f894e"
warning_color = "#e98a00"
danger_color = "#c84053"
```

## Moonfly

```toml
[appearance]
background_color = "#080808"
text_color = "#bdbdbd"
primary_color = "#80a0ff"
success_color = "#8cc85f"
warning_color = "#e3c78a"
danger_color = "#ff5454"
```

## Nightfly

```toml
[appearance]
background_color = "#011627"
text_color = "#bdc1c6"
primary_color = "#82aaff"
success_color = "#a1cd5e"
warning_color = "#e3d18a"
danger_color = "#fc514e"
```

## Oxocarbon

```toml
[appearance]
background_color = "#232323"
text_color = "#d0d0d0"
primary_color = "#00b4ff"
success_color = "#00c15a"
warning_color = "#be95ff"
danger_color = "#f62d0f"
```

## Ferra

```toml
[appearance]
background_color = "#2b292d"
text_color = "#fecdb2"
primary_color = "#d1d1e0"
success_color = "#b1b695"
warning_color = "#f5d76e"
danger_color = "#e06b75"
```
