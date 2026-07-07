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

## Status Bar

The look of the status bar is configured under the `[appearance.bar]` section.

### Surface

The `surface` field controls where the background color is painted:

- `transparent`: This is the default. The bar itself is see-through and each
  module group is painted with the background color, giving the "islands" look.
- `solid`: The bar is painted with the background color as a single continuous
  surface.

```toml
[appearance.bar]
surface = "solid"
```

### Radius

The `radius` field rounds the corners of the bar surface (it only has an effect
with `surface = "solid"`). Values are steps of the radius scale: `none` (square),
`sm`, `md`, `lg`, `xl`.

It uses CSS `border-radius` shorthand: a single value applies to all corners, two
values are `[top-left+bottom-right, top-right+bottom-left]`, and four values are
`[top-left, top-right, bottom-right, bottom-left]`.

```toml
[appearance.bar]
surface = "solid"
radius = "md"                       # all corners
# radius = ["none", "none", "md", "md"]  # square top, rounded bottom
```

### Margin

The `margin` field insets the bar from the screen edges, turning it into a
floating bar. Values are steps of the spacing scale: `none` (default), `xxs`,
`xs`, `sm`, `md`, `lg`, `xl`, `xxl`.

It uses CSS `margin` shorthand: a single value applies to all edges, two values
are `[vertical, horizontal]`, and four values are `[top, right, bottom, left]`.

```toml
[appearance.bar]
margin = "sm"              # all edges
# margin = ["xs", "md"]    # vertical, horizontal
```

### Opacity

The `opacity` field sets the opacity of the status bar components. The value
should be a float between `0.0` (fully transparent) and `1.0` (fully opaque,
the default).

```toml
[appearance.bar]
opacity = 0.8
```

## Menu Opacity

It's also possible to define the opacity of status bar menus and whether they
should include a backdrop effect.

The `backdrop` effect adds a blur/transparent background to menus, making them
appear semi-transparent over the content behind them. The value should be a float
between `0.0` (disabled) and `1.0` (maximum blur).

**Default values:**

- `menu.opacity`: `1.0` (fully opaque)
- `menu.backdrop`: `0.0` (disabled)

```toml
[appearance.menu]
opacity = 0.7
backdrop = 0.3
```
