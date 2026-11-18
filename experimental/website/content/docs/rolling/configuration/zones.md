+++
title = "Zones and items"
weight = 5
+++

A zone is a part of the bar with interactions: a resting row of items, a
second row of details shown on hover, and what a click on it opens. An item
is one piece of information: the battery, the time, the window's title.

{% <note kind="info" title="Placeholders"> %}
Items show placeholders for now, sized like the real ones. The rows and a
zone's click are in place; the data, and an item's own `show` and `open`, are
read from the file but not acted on yet.
{% </note> %}

## Built-in zones

Naming a zone in a node uses its defaults:

| Zone | Items | Details | Opens |
| ---- | ----- | ------- | ----- |
| `workspaces` | `workspaces` | `workspace_list` | nothing |
| `window` | `window` | `launcher_button`, `window_list` | nothing |
| `media` | `media` | `media_details` | `media` |
| `clock` | `weather`, `date`, `clock`, `notifications` | `date`, `forecast`, `notices` | `calendar` |
| `status` | `privacy`, `airplane`, `dnd`, `idle_inhibitor`, `bluetooth`, `microphone`, `volume`, `network`, `battery` | `system`, `updates` | `settings` |
| `privacy` | `privacy` | | nothing |
| `system` | `cpu`, `memory`, `temperature` | | `system` |
| `tray` | `tray` | | nothing |
| `updates` | `updates` | | `updates` |
| `keyboard` | `keyboard_layout`, `keyboard_submap` | | nothing |

## `[zone.<name>]`

A table named after a built-in zone overrides only the keys written; a new
name is a zone built from scratch.

```toml
[zone.clock]
details = ["forecast"]          # the clock, with only the forecast on hover

[zone.hot]                      # a zone of its own, placed by naming it
                                # in a node, as end = [["hot", "status"]]
items = [{ item = "system", show = "notable" }]
open  = "system"
```

| Key | Values | What it sets |
| --- | ------ | ------------ |
| `items` | a list of items | The resting row, in the order written. |
| `details` | a list of items | The row shown on hover; empty for none. |
| `open` | a panel, or `"none"` | What a click on the zone does. |

A zone that is not placed in any node is reported, and not shown.

## Items

An item is written by its name, or in the long form with its options:

```toml
items = ["battery", { item = "system", show = "notable", open = "system" }]
```

| Key | Values | What it sets |
| --- | ------ | ------------ |
| `item` | an item's name | Which item; required. |
| `show` | `always`, `notable`, `never` | When it is visible. |
| `open` | a panel, or `"none"` | What a click on this item does, over its zone's. |

The items, with when they show and what they open by default:

| Item | Shows | Opens |
| ---- | ----- | ----- |
| `workspaces`, `workspace_list` | always | the zone's |
| `window`, `window_list` | notable | the zone's |
| `launcher_button` | always | `launcher` |
| `clock`, `date` | always | `calendar` |
| `weather`, `forecast` | notable | `calendar` |
| `notifications`, `notices` | notable | `notifications` |
| `media`, `media_details` | notable | `media` |
| `tray` | notable | the zone's |
| `updates` | notable | `updates` |
| `system`, `cpu`, `memory`, `temperature`, `disk` | notable | `system` |
| `privacy` | notable | the zone's |
| `airplane`, `dnd`, `idle_inhibitor`, `power_profile`, `bluetooth`, `microphone`, `volume`, `network`, `vpn`, `peripheral_battery` | notable | `settings` |
| `brightness` | never | `settings` |
| `battery` | always | `settings` |
| `keyboard_layout`, `keyboard_submap` | notable | the zone's |

An item with nothing to show never shows, whatever its `show`: no window
focused, no battery, the weather not fetched yet. `notable` is about an item
that has something to show, and shows it only when it is worth seeing: a
player playing, many updates waiting. What it means item by item is still
being settled.

## Panels

`open` takes one of `settings`, `calendar`, `notifications`, `media`,
`system`, `tray`, `updates` and `launcher`, or `none`.
