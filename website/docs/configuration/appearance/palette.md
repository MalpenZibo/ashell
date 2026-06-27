---
sidebar_position: 2
---

# Palette

With these configuration options, you can customize
the color palette of your status bar.

## Color Syntax

Each color can be specified as a simple hex color or as an object with variants.

### Simple Syntax

```toml
primary_color = "#7aa2f7"
```

### Advanced Syntax

```toml
[appearance.primary_color]
base = "#7aa2f7"
strong = "#8aacff"
weak = "#6988e6"
text = "#1a1b26"
```

All fields except `base` are optional. If not provided, they are
auto-generated from the base color:

- **`weak`**: The base color faded toward the background. Used for subtle,
  de-emphasized states (e.g. hover backgrounds, inactive elements).
- **`strong`**: The base color pushed away from the background for more
  contrast. Used for emphasized, active states.
- **`text`**: The text color to use on top of this color. If not provided,
  the default text color is used (with automatic contrast adjustment).

## Palette Colors

| Color | Description |
|-------|-------------|
| `background_color` | Background color for all status bar components |
| `primary_color` | Accent color for interactive elements (buttons, slider handles, active states) |
| `success_color` | Positive feedback indicators (e.g. connected, active) |
| `warning_color` | Cautionary indicators (e.g. alerts that need attention) |
| `danger_color` | Error or destructive state indicators |
| `text_color` | Default text color |

### Background Color

The background color supports additional granularity. Beyond `base`, `weak`,
`strong`, and `text`, you can also specify intermediate levels:

```toml
[appearance.background_color]
base = "#1e1e2e"
weakest = "#1a1a28"
weaker = "#1c1c2c"
weak = "#313244"
neutral = "#3a3a50"
strong = "#45475a"
stronger = "#505268"
strongest = "#5a5c76"
text = "#cdd6f4"
```

All levels are optional. When omitted, they are auto-generated as
gradual steps between the base color and the text color, providing
a range of surface tones from subtle to prominent.

## Workspace Colors

You can customize the color of workspace indicators 
based on the monitor they are attached to.

| Option | Description |
| -------------- | --------------- |
| `workspace_colors` | The default colors of regular, inactive workspaces (falls back to `primary_color` if undefined) |
| `active_workspace_colors` | The colors used for currently active workspaces (falls back to `workspace_colors` if undefined) |
| `special_workspace_colors` | The colors used for special workspaces (falls back to `workspace_colors` if undefined) |

Each option accepts a list of colors. 
The colors are assigned to monitors sequentially based on the order your monitors are defined/detected.
This means the first color in each list only applies to workspaces on monitor 1, the second color to workspaces on monitor 2, and so on.
Example: If workspace 1 and 3 are both on `monitorA`, the first
color will be used for both of them; if workspace 2 is attached to `monitorB`, the second color will be used.

## Complete Examples

For complete theme examples with full palette configurations, see the [Theme documentation](./theme.md).
