+++
title = "Appearance"
weight = 6
+++

`[appearance]` is [styletto](https://github.com/MalpenZibo/styletto)'s own
table: the colours, fonts and feel of everything the bar draws. Theme,
flavour, accent and colour names match in any case, and one that matches
nothing is logged and left out. The other values are written in lower case,
as listed: one that is not among them is logged, and the whole
`[appearance]` table falls back to its defaults.

```toml
[appearance]
theme        = "catppuccin"
dark_flavor  = "mocha"
light_flavor = "latte"
accent       = "mauve"
mode         = "auto"

density  = "default"
shape    = "soft"
material = "translucent"
motion   = "full"

[appearance.fonts]
sans = "Inter"
mono = "JetBrains Mono"
```

The same keys can live in a file of their own, shared with other programs
that use styletto, and be read with `import = "path"`; see
[imports](@/docs/rolling/configuration/_index.md#imports).

## Colours

One source of colours, and the colours you want set over it:

| Key | What it sets |
| --- | ------------ |
| `theme` | A theme by name: `tokyo night` (default), `catppuccin`, `nord`, `gruvbox`, `dracula`, `rosé pine`, `everforest`, `kanagawa`. |
| `dark_flavor`, `light_flavor` | The theme's flavour for each side, as `mocha` and `latte` for Catppuccin. |
| `accent` | The accent, on each side whose flavour has it. |
| `seed` | Both sides worked out from one colour, `"#6750a4"`. It wins over `theme`. |
| `wallpaper` | Both sides worked out from an image, `"~/Pictures/wall.jpg"`. It wins over `seed`. |
| `variant` | How a seed or a wallpaper spreads over the roles: `tonal-spot` (default), `vibrant`, `expressive`, `content`, `fidelity`, `neutral`, `monochrome`, `rainbow`, `fruit-salad`. |
| `contrast` | For a seed or a wallpaper: `standard` (default), `medium`, `high`. |
| `mode` | Which side: `auto` (default), `light`, `dark`. The bar does not read the system's preference yet, so `auto` is dark for now. |

Any colour of either side can be set over the source, by name, `_` for a
space:

```toml
[appearance.dark]
accent    = "#ff79c6"
on_accent = "#1a1b26"

[appearance.light]
background = "#fdf6e3"
```

## Feel

| Key | Values | What it does |
| --- | ------ | ------------ |
| `density` | `compact`, `default`, `comfortable` | Moves type, controls, icons and space one step down or up; the panels grow or shrink with them. |
| `shape` | `square`, `soft` (default), `round` | Moves every corner one step; a pill stays a pill. The islands' corners follow it. |
| `material` | `solid` (default), `translucent`, `glass` | The islands' opacity (1, 0.86, 0.75) and blur behind them. |
| `motion` | `full` (default), `reduced`, `none` | Springs, fades only, or instant changes. |

## Fonts

| Key | What it sets |
| --- | ------------ |
| `fonts.sans` | The family of the text; the system's when unset. |
| `fonts.mono` | The monospaced family; the system's when unset. |
