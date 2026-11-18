+++
title = "Panels and details"
weight = 4
+++

A click on a zone opens its panel; with `hover`, the pointer resting on a
zone shows its details. Both are set on a node, or on the bar for all of
them, and flow down.

{% <note kind="info" title="Placeholders"> %}
The panels are placeholders for now, sized like the real ones: how they open
and close is what is in place.
{% </note> %}

## `panels`

| Value | What a click does |
| ----- | ----------------- |
| `"dropdown"` (default) | The panel drops down from the edge of the nearest island, or of the bar, as wide as it is from the start: the stable ashell's menus. |
| `"merge"` | The panel splits off the nearest island, starting as a sliver and growing to its size, merged with it. It merges back on close. Whether it stays joined by a neck is the bar's `gap`: see below. |
| `"morph"` | The group's own island turns into the panel, and back. |

### Joined or apart: `gap`

The bar's `gap` (a space step, `s2` by default) is how far panels and
split-off details stand off what they come out of, a dropdown's from the
edge included. For a `merge` panel and the details it also says whether
they stay joined:

- below `s4` they stay joined to it by a neck;
- from `s4` they come out joined, the neck breaks as they arrive, and they
  stand apart once out.

It is one for the whole bar, not one per panel: the shapes that merge in one
island reach for each other as far as one distance.

```toml
[bar.main]
panels = "merge"
gap    = "s4"   # panels and details stand apart
```

### Where a panel opens

`dropdown` and `merge` open the panel as a shape of its own,
centred under the zone that opened it: a group with several zones that open
panels opens each under its own zone. It is pushed back inside where
centring would take it past the ends of the nearest island round the group,
when that island is wide enough to hold it, or else past the bar's. The
group's neighbours do not move.

`merge` panels come out of the nearest island round the zone: the
group's, or else that of the nearest node round it that has one, or else the
background. The border runs round the island and the panel as one outline;
the background has none, so out of it only the panel has a border, fading
out along the neck. While it is out the panel hangs from the island's edge
and follows it as the island changes size.
Closing, the panel's content fades first, then its shape sinks back.

`morph` grows the group itself: in `center` it grows on both sides, in `start`
and `end` away from the bar's edge, and its neighbours in the list step aside
while it is open.

### When there is no island

A panel needs an island for what it does. Where there is none it drops down
as with `dropdown`, and the log says so:

- `merge` needs an island round the zone to come out of, or
  the background. With `shape = false` on the bar and no background there is
  none.
- `morph` needs the group's own island: a node's island round it does not
  turn into a panel.

A click anywhere outside an open panel, or <kbd>Esc</kbd>, closes it.

## `hover`

With `hover = true`, the pointer resting on a zone shows its
[`details`](@/docs/rolling/configuration/zones.md), a second row of items.
Where it shows depends on the shape round the group:

- **The group has its own island**: the row is added to the island, away
  from the bar's edge, under the zones on a top bar and over them on a bottom
  one. The island grows to hold it, and the zones stay where they are.
- **It has none**: the row is a small island of its own that splits off what
  holds the group, the nearest island round it or the background, as a
  `merge` panel does, and goes back the same way. With nothing round the
  group it is an island of its own under the zone, with the island's border
  and shadow.

Going from one zone to another of the same list, the split-off row stays out,
slides under the new zone and changes its row. The pointer on it counts as
on the group, so the row stays while the pointer goes from the zone to it.

The row comes once the pointer stays on a zone a moment, and goes a moment
after the pointer has left the group. On a zone with no details the row
shown goes.

```toml
[bar.main]
shape  = true        # the groups have no island of their own...
hover  = true        # ...so their details split off the bar's
panels = "merge"
gap    = "s4"       # panels and details stand apart

start  = ["workspaces"]
center = [["window", "clock"]]
end    = [["privacy", "status"]]
```
