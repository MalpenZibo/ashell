---
sidebar_position: 2
---

# Palette

With these configuration options, you can customize  
the color palette of your status bar.

Each color can be a simple hex color like `#228800` or an  
object that defines:

- A base hex color
- Two optional variants of that color (a strong one and a weak one)
- An optional text color to use with that base color

If the strong and weak variants are not provided, they will be auto-generated.  
If no text color is provided, the default text color will be used.

## Example

```toml
[appearance.background_color]
base = "#448877"
strong = "#448888"
weak = "#448855"
text = "#ffffff"
```

## Palette Colors

The following are the colors that make up the palette:

- `background_color`: Used as the background color for all status bar components
- `primary_color`: Used for elements like buttons or slider handles
- `secondary_color`: Used for borders and slider tracks
- `success_color`: Used for success indicators
- `danger_color`: Used for danger messages or danger states  
  (the weak version is used for warning states)
- `text_color`: Used as the default text color

## Workspace Colors

The following colors are used for the workspaces module.

You can specify which color to use for workspace indicators based on  
the monitor to which a workspace is attached.

For example, if workspace 1 is attached to `monitorA`, the first
color will be used; if workspace 2 is attached to `monitorB`,
the second color will be used, and so on.

Use the `workspace_colors` field for regular workspaces, and  
`special_workspace_colors` for special workspaces.

If `special_workspace_colors` is not defined, `workspace_colors` will be used.  
If neither `workspace_colors` is defined nor a color exists
for a given monitor, the `primary_color` will be used.

## Complete Examples

For complete theme examples with full palette configurations, see the [Theme documentation](./theme.md). These examples show popular color schemes like Catppuccin Mocha, Tokyo Night, and Nord with all colors configured.

## Simple vs Advanced Syntax

You can use simple hex colors for basic customization:

```toml
[appearance]
primary_color = "#7aa2f7"
success_color = "#9ece6a"
text_color = "#a9b1d6"
```

Or use the advanced syntax with variants for more control:

```toml
[appearance.primary_color]
base = "#7aa2f7"
strong = "#8aacff"
weak = "#6988e6"
text = "#1a1b26"
```
