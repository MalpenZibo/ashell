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
| `"attached"` | The panel splits off the nearest island, starting as a sliver and growing to its size, and stays joined to it by a neck. It merges back on close. |
| `"detached"` | The same, but it stands far enough off for the neck to break as it arrives: it stands apart once out. |
| `"morph"` | The group's own island turns into the panel, and back. |

One value says the whole behaviour: how far a panel stands and whether it
merges are not knobs of their own.

### Where a panel opens

`dropdown`, `attached` and `detached` open the panel as a shape of its own,
centred under the zone that opened it: a group with several zones that open
panels opens each under its own zone. It is pushed back inside where
centring would take it past the ends of the nearest island round the group,
when that island is wide enough to hold it, or else past the bar's. The
group's neighbours do not move.

`attached` and `detached` come out of the nearest island round the zone: the
group's, or else that of the nearest node round it that has one, or else the
background. The border runs round the island and the panel as one outline;
the background has none, so out of it only the panel has a border, fading
out along the neck.
Closing, the panel's content fades first, then its shape sinks back.

`morph` grows the group itself: in `center` it grows on both sides, in `start`
and `end` away from the bar's edge, and its neighbours in the list step aside
while it is open.

### When there is no island

A panel needs an island for what it does. Where there is none it drops down
as with `dropdown`, and the log says so:

- `attached` and `detached` need an island round the zone to come out of, or
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
  holds the group, the nearest island round it or the background, as an
  `attached` panel does, and goes back the same way.

Going from one zone to another of the same list, the split-off row stays out,
slides under the new zone and changes its row. The pointer on it counts as
on the group, so the row stays while the pointer goes from the zone to it.

```toml
[bar.main]
shape  = true        # the groups have no island of their own...
hover  = true        # ...so their details split off the bar's
panels = "detached"

start  = ["workspaces"]
center = [["window", "clock"]]
end    = [["privacy", "status"]]
```
