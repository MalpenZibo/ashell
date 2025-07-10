---
sidebar_position: 2
---

# Palette

With these configuration options, you can customize
the color palette of your status bar.

Each color could be a simple hex color like `#228800` or an
object that defines:

- a base hex color
- two optional variant of that color (a strong one and a weak one)
- an optional text color that should be used with that base color

Without providing the two variant of the base color the strong and weak variant
will be auto-generated.

Without providing a text color the default text color will be used.

## Example

```toml
[appearance.background_color]
base = "#448877"
strong = "#448888"
weak = "#448855"
text = "#ffffff"
```

## Palette color

The following are the colors that compose a palette:

- `background_color`: It's used as a background color for every status bar component
- `primary_color`: It's used for things like button or slider handle
- `secondary_color`: It's used for things like border color or slider track color
- `success_color`: It's used for every success indicator
- `danger_color`: It's used for danger message or danger state
  (the weak version is used for the warning state
- `text_color`: It's used as default text color

## Workspaces color

The following color are used for the workspaces module.

With this list of color you can specify which color to use for the workspace indicator
based on the attached monitor.

So for example if the workspace 1 is attached to the monitorA then
the first color will be used, if the workspace 2 is attached to
the monitorB then the second color will be used and so on.

You can specify a list for normal workspaces using the `workspace_colors` field and
one for the special workspaces using the field `special_workspace_colors`.

If the `special_workspace_colors` is not defined then the `workspace_colors`
will be used.

If the `workspace_colors` is not defined or doesn't exist a color for a
particular monitor then the `primary_color` will be used.
