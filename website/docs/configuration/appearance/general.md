---
sidebar_position: 1
---

# General

These are all the appearance options not related to the color palette.

## Font

You can change the font used by setting the `font_name` field. This configuration
is optional—if not set, the `iced` library will use the default font.

```toml
[appearance]
font_name = "Comic Sans MS"
```

:::warning

Changing the font requires killing and restarting ashell process. The font configuration does not support hot-reloading

:::

## Scaling Factor

You can change the scaling factor of the status bar using the `scale_factor` field.

The value should be a float greater than `0.0` and less than or equal to `2.0`.
The default value is `1.0`.

```toml
[appearance]
scale_factor = 1.5
```

## Status Bar Style

You can change the style of the status bar using the `style` field.

You can choose between:

- `Islands`: This is the default style. Each module or module group is displayed
  in a rounded rectangle using the background color.
- `Solid`: The status bar has a solid background color.
- `Gradient`: The status bar has a gradient background color.

### Example

```toml
[appearance]
style = "Gradient"
```

## Opacity

You can change the opacity of the status bar components using the `opacity` field.

The value should be a float between `0.0` and `1.0`, where `0.0` is fully transparent.
The default value is `1.0`.

It's also possible to define the opacity of status bar menus and whether they should
include a backdrop effect.

The `backdrop` effect adds a blur/transparent background to menus, making them appear
semi-transparent over the content behind them. The value should be a float between `0.0`
and `1.0`, where `0.0` disables the effect and `1.0` applies maximum blur.

**Default values:**

- `opacity`: `1.0` (fully opaque)
- `menu.opacity`: `1.0` (fully opaque)
- `menu.backdrop`: `0.0` (disabled)

## Examples

Setting the opacity of the status bar components:

```toml
[appearance]
opacity = 0.8
```

Also setting the opacity of the status bar menus and adding a backdrop effect:

```toml
[appearance]
opacity = 0.8

[appearance.menu]
opacity = 0.7
backdrop = 0.3
```

## Per-Module Styling

You can override the global appearance settings for individual modules. This allows
you to give specific modules a different look from the rest of the status bar.

Each key under `[appearance.modules]` is the module name. The available module names
are: `Workspaces`, `Tempo`, `SystemInfo`, `Tray`, `Privacy`, `Settings`,
`MediaPlayer`, `Notifications`, `Clipboard`, `Updates`, `WindowTitle`,
`KeyboardLayout`, `KeyboardSubmap`, or a custom module name.

### Available fields

| Field | Description |
|---|---|
| `opacity` | Override the bar component opacity for this module (`0.0`–`1.0`) |
| `background_color` | Override the background color for this module |
| `text_color` | Override the text color for this module |
| `border_radius` | Override the border radius for this module (in pixels) |

### Example

```toml
[appearance.modules.Workspaces]
opacity = 1.0
background_color = "#2ac3de"

[appearance.modules.SystemInfo]
text_color = "#f7768e"
border_radius = 8

[appearance.modules."my-custom-module"]
opacity = 0.9
```

## Per-Popup Styling

You can override the popup (menu) appearance for specific menu types. This lets you
style individual popups differently from the global menu settings.

Each key under `[appearance.popups]` is the popup name. The available popup names
are: `Updates`, `Settings`, `Notifications`, `Tray`, `MediaPlayer`, `SystemInfo`,
`Tempo`, `Clipboard`.

### Available fields

| Field | Description |
|---|---|
| `opacity` | Override the popup opacity for this menu (`0.0`–`1.0`) |
| `backdrop` | Override the backdrop effect for this menu (`0.0`–`1.0`) |
| `background_color` | Override the popup background color for this menu |
| `border_radius` | Override the popup border radius (in pixels) |
| `width` | Override the popup width. Can be `Small`, `Medium`, `Large`, or `XLarge` |

### Example

```toml
[appearance.popups.Notifications]
opacity = 0.9
backdrop = 0.2
width = "Large"

[appearance.popups.Settings]
background_color = "#1a1b26"
border_radius = 16
width = "Medium"

[appearance.popups.Clipboard]
opacity = 0.85
width = "Small"
```
