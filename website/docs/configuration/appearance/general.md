---
sidebar_position: 1
---

# General

This are all the appearance options not related to the color palette.

## Font

You can change the font used using the `font_name` field. This configuration
is optional, and if not set, the iced library will use the default font.

```yaml
[appearance]
font_name = "Comic Sans MS"
```

## Status bar style

You can change the style of the status bar using the `style` field.

You can choose between:

- `Island`: It's the default style. Each module or module group will be displayed
  in a rounded rectangle using the background color.
- `Solid`: The status bar has a solid background color.
- `Gradient`: The status bar has a gradient background color.

### Example

```yaml
[appearance]
style = "Gradient"
```

## Opacity

You can change the opacity of the status bar components using the `opacity` field.

The value should be a float between `0.0` and `1.0`, where `0.0` is fully transparent.
The default value is `1.0`.

It's also passible to define the opacity of the status bar menus and if they should
add a backdrop effect.

## Examples

Setting the opacity of the status bar components

```yaml
[appearance]
opacity = 0.8
```

Setting also the opacity of the status bar menus and adding a backdrop effect

```yaml
[appearance]
opacity = 0.8

[appearance.menu]
opacity = 0.7
backdrop = 0.3
```
