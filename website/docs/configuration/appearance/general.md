---
sidebar_position: 1
---

# General

These are all the appearance options not related to the color palette.

## Font

You can change the font used by setting the `font_name` field. This configuration
is optional—if not set, the `iced` library will use the default font.

```yaml
[appearance]
font_name = "Comic Sans MS"
```

## Scaling Factor

You can change the scaling factor of the status bar using the `scale_factor` field.

The value should be a float greater than `0.0` and less than `2.0`.
The default value is `1.0`.

## Status Bar Style

You can change the style of the status bar using the `style` field.

You can choose between:

- `Island`: This is the default style. Each module or module group is displayed
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

It's also possible to define the opacity of status bar menus and whether they should
include a backdrop effect.

## Examples

Setting the opacity of the status bar components:

```yaml
[appearance]
opacity = 0.8
```

Also setting the opacity of the status bar menus and adding a backdrop effect:

```yaml
[appearance]
opacity = 0.8

[appearance.menu]
opacity = 0.7
backdrop = 0.3
```
