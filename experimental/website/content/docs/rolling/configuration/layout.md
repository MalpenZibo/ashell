+++
title = "Layout"
weight = 2
+++

A bar's content is one node, the root, with three places: `start`, `center`
and `end`. Each place holds a list of nodes, side by side, and a node holds
one of three things:

- **zones**, side by side: a *group*, the node that shows the zones' rows and
  opens their panels;
- **nodes**, side by side: a *row*;
- **nodes placed** at its own `start`, `center` and `end`, as the bar places
  its own.

`start` and `end` rather than left and right, so the same names will fit a
bar on a side edge.

## Short and long

| Node | Short | Long |
| ---- | ----- | ---- |
| a group | `"clock"` (one zone), or `["clock", "status"]` | `{ zones = ["clock", "status"], panels = "morph" }` |
| a row | | `{ nodes = [ … ], shape = true }` |
| placed | | `{ start = [ … ], center = [ … ], end = [ … ] }` |

A place, and `nodes`, take a list of nodes or one node on its own:
`center = "clock"` is `center = ["clock"]`. A list inside the list is a
group:

```toml
[bar.main]
end = ["system", ["clock", "status"]]   # two groups: system alone, and clock with status
```

### The long form's keys

| Key | Values | Default | What it sets |
| --- | ------ | ------- | ------------ |
| `zones` | zones | | A group: its zones, in order. |
| `nodes` | nodes | | A row: its nodes, in order. |
| `start`, `center`, `end` | nodes | | Placed: the nodes at each place. |
| `shape` | `true`, `false` | see [Islands](@/docs/rolling/configuration/islands.md) | An island round this node, or none. |
| `hover` | `true`, `false` | the outer node's | Its zones show their details on hover. |
| `panels` | `dropdown`, `attached`, `detached`, `morph` | the outer node's | How its groups open their panels. |

A node holds one of `zones`, `nodes` or the places. One with more than one is
reported, and the first in that order is kept.

## How wide

- A group and a row are as wide as what they hold, and grow and shrink with
  it.
- A placed node fills the room it has, since it has ends to put things at.
  In `center`, where the room is its content's, it puts its three places side
  by side at their content's width. Two placed nodes in the same place share
  its room.

## Where

`start` and `end` keep to their edge. The `center` stays in the middle of the
bar as long as the ends leave it room, and moves over only as far as an end
too long for its half pushes it; when it fits nowhere, it is in the middle of
the gap between the ends. A group that grows in the centre stays centred.

## What shows

{% <note kind="info" title="Not drawn yet"> %}
Items are placeholders for now and always show: what follows is read from
the file, and is how the bar will behave once items have their data.
{% </note> %}

A zone with no visible items takes no room. A group with no visible zones
disappears, island and all, and so does a node with nothing visible inside.
That is how a group shows only when something happens: give it items that
are visible only then.

```toml
[bar.main]
center = [["window", "clock"], "hot"]

[zone.hot]
items = [{ item = "system", show = "notable" }]  # only when the system runs hot
open  = "system"
```

## Examples

One island round each place, holding its groups:

```toml
[bar.main]
start  = { nodes = ["workspaces"], shape = true }
center = { nodes = ["window", "clock"], shape = true }
end    = { nodes = [["privacy", "status"]], shape = true }
```

The bar in two halves, each an island with its own ends:

```toml
[bar.main]
start = { shape = true, start = ["workspaces"], end = ["window"] }
end   = { shape = true, start = ["media"], end = [["clock", "status"]] }
```
