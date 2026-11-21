+++
title = "Zones and items"
weight = 5
+++

A zone is a part of the bar with interactions: a resting row of items, a
second row of details shown on hover, and what a click on it opens. An item
is one piece of information: the battery, the time, the window's title.

{% <note kind="info" title="Placeholders"> %}
The items of [`tempo`](@/docs/rolling/configuration/tempo.md) are drawn,
with their `show`, `open` and gestures. The others show placeholders for
now, sized like the real ones; a zone none of whose items is drawn yet is
one placeholder with its name.
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
| `config` | `config` | | the config window |

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
| `open` | a panel, `"config"`, or `"none"` | What a click on the zone does: `"config"` opens the config window. |

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
| `open` | a panel, `"config"`, or `"none"` | What a click on this item does, over its zone's. |
| `right_click`, `middle_click`, `scroll_up`, `scroll_down` | an action | What the gesture does: see [Gestures](#gestures). |
| other keys | | The item's own options, as its module documents them. |

The items, with when they show and what they open by default:

| Item | Shows | Opens |
| ---- | ----- | ----- |
| `workspaces`, `workspace_list` | always | the zone's |
| `window`, `window_list` | notable | the zone's |
| `launcher_button` | always | `launcher` |
| `clock`, `date` | always | `calendar` |
| `weather`, `forecast` | always | `calendar` |
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
| `config` | always | the config window |

An item with nothing to show never shows, whatever its `show`: no window
focused, no battery, the weather not fetched yet. `notable` is about an item
that has something to show, and shows it only when it is worth seeing: a
player playing, many updates waiting. What it means item by item is still
being settled.

## Gestures

A click opens a panel, with `open`. The other gestures run an action:

| Key | Gesture |
| --- | ------- |
| `right_click` | The right button. |
| `middle_click` | The middle button. |
| `scroll_up`, `scroll_down` | The wheel, one action a notch, or the touchpad, one action every so far. |

An action is one of:

- an action of the item's, by name, as its module documents them:
  `"cycle_format"`, `"next_timezone"`;
- a panel to open, `{ open = "calendar" }`, or the config window,
  `{ open = "config" }`;
- a command to run with the shell, `{ run = "pavucontrol" }`;
- `"none"`, to switch off what the item does by default.

Every item comes with its gestures, and the file writes only what differs.
Turning the wheel the other way is binding the actions the other way round:

```toml
[zone.clock]
items = [{ item = "clock", scroll_up = "previous_timezone", scroll_down = "next_timezone" }]
```

Only `open` falls back to the zone's: a gesture with nothing bound does
nothing. Hovering a zone shows its details, with `hover`.

## Panels

`open` takes one of `settings`, `calendar`, `notifications`, `media`,
`system`, `tray`, `updates` and `launcher`, or `none`.
