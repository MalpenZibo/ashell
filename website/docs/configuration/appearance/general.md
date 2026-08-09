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

:::tip Finding the exact font name

The `font_name` must match the font's family name exactly (e.g. `"Terminus (TTF)"`,
not `"Terminus"`). To list available fonts and their exact names, run:

```bash
fc-list | cut -d: -f2 | sort -u
```

:::

:::info Font weight

ashell picks the face whose declared weight is closest to Normal (400). If the
font has no face with weight 400 (for example, Terminus TTF's Regular face reports
weight 500/Medium), ashell uses the closest available face. Text that requests a
different weight (e.g. Bold) will then look the same as regular text.

This is also why ashell **cannot use bitmap fonts** (`.bdf`/`.pcf`), which are
the format of the `terminus-font` package on Arch Linux — only TrueType (`.ttf`)
and OpenType (`.otf`/`.otc`) fonts are supported.

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

The `backdrop` effect darkens whatever is behind an open menu, so the menu stands
out from the content around it. It is drawn by ashell and involves no blur — see
[Blur](#blur) for that. The value should be a float between `0.0` (disabled) and
`1.0` (fully dark).

**Default values:**

- `menu.opacity`: `1.0` (fully opaque)
- `menu.backdrop`: `0.0` (disabled)

```toml
[appearance.menu]
opacity = 0.7
backdrop = 0.3
```

## Blur

The `blur` field asks the compositor to blur the wallpaper behind ashell's
translucent surfaces — the bar (the island pills when `bar.surface` is
`transparent`, the whole bar when it is `solid`), menus, the OSD and toast
notifications — using the `ext-background-effect-v1` Wayland protocol. It is a
no-op on compositors that do not support that protocol.

| Value | Behaviour |
| --- | --- |
| `"auto"` (default) | Ask for blur when `bar.opacity` or `menu.opacity` is below `1.0` |
| `"always"` | Ask for blur regardless of opacity |
| `"never"` | Never ask |

`"auto"` exists because blurring a fully opaque surface cannot be seen: it asks
for the effect exactly when the effect can show. Use `"never"` if you want
translucent surfaces without blur.

This is different from `menu.backdrop`, which is an ashell-drawn darkening
applied only behind open menus. `blur` affects the wallpaper behind the surface
itself and requires compositor support.

### Example

```toml
[appearance]
blur = "auto"

[appearance.bar]
opacity = 0.8
```

### Compositor setup

Supporting the protocol is not enough on its own — most compositors also want
blur turned on somewhere in their own config before they will draw it.

On niri, add a layer rule matching ashell's namespaces:

```kdl
layer-rule {
    match namespace="^ashell-"
    background-effect {
        blur true
    }
}
```

On Hyprland, enable blur globally:

```conf
decoration {
    blur {
        enabled = true
    }
}
```

No `layerrule = blur` is needed: once a surface uses the protocol Hyprland
follows the region ashell publishes and ignores the layer rule. But with
`decoration:blur:enabled = false` nothing is drawn even though the protocol is
advertised, so `blur` will look like it does nothing.

Note that `"never"` means "blur nothing", not "leave it to the compositor": on a
compositor that supports the protocol ashell always publishes a region, and an
empty one takes precedence over a rule like the ones above. If you configured
blur in your compositor and want to keep it, use `"auto"` or `"always"`.
