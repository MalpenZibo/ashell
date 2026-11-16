+++
title = "Islands and background"
weight = 3
+++

Two things can be behind the zones, and they are kept apart: the bar's
background, and the islands.

## `background`

| Value | Bar |
| ----- | --- |
| `"transparent"` (default) | Nothing behind what is drawn. |
| `"solid"` | A flat fill in the island's colour and material, from edge to edge of the screen, with no corners, no border and no shadow. |

A solid background is as tall as a group and the padding on both sides. The
groups with no island round them open their panels and details out of it, as
out of an island: with no island anywhere, it is the classic strip along the
edge, panels and all.

## `shape`

The island is styletto's: a shape with the theme's colour, material, corners,
border and shadow, the same at every depth. `shape` says which nodes have
one:

- **A group** has one, unless a node round it has one or says
  `shape = false`.
- **A row or a placed node** has one only with `shape = true`, and the nodes
  inside then have none unless they say so.

| Written | What is drawn |
| ------- | ------------- |
| nothing | One island round each group. |
| `shape = true` on the bar | One island as long as the bar, `padding` off the screen's edges. |
| `{ nodes = [...], shape = true }` in each place | One island round each place's groups. |
| `shape = false` on the bar | No island anywhere: the zones alone. |

The bar's island is an island like the others: as tall as a group, with its
corners and border all round, against the screen's edges with
`padding = "s0"`. A strip along the edge is the background's.

A node inside an island can still have its own: with `shape = true` on the
bar,

```toml
end = [{ zones = ["status"], shape = true }]
```

is an island of its own on the bar's.

## The two together

- `background = "solid"` with the groups' islands is islands on a flat bar.
- `background = "solid"` with `shape = false` on the bar is a flat bar with
  nothing on it but the zones: the stable ashell's solid bar.
- `solid` under the bar's island is allowed, though the island covers it but
  for the padding round it.

```toml
[bar.main]
background = "solid"
shape      = false
panels     = "dropdown"

start  = ["workspaces"]
center = ["window"]
end    = [["clock", "privacy", "status"]]
```

## The island's look

It is the theme's, and the same everywhere, so a group, the bar and a panel
merging with them never differ: its corners follow
[`[appearance] shape`](@/docs/rolling/configuration/appearance.md)
(`square`, `soft`, `round`), and its colour, translucency and blur follow
`material`.
