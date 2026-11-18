# The bar model

Status: in progress, started 2026-10-23.

This document fixes the structure of the configuration for a GUIdo-based
ashell built on styletto, one that can express both today's ashell (islands
along a bar, menus that drop down) and the island prototype (one island that morphs).
Parts not decided yet are listed, with the proposals to start from, under
[Not decided yet](#not-decided-yet).

It is implemented step by step in `ashell-experimental`: see
[Implementation plan](#implementation-plan).

## Principles

- **One grammar, two experiences.** Classic ashell and the morphing island are two
  configuration files, not two modes in the code. Every middle ground is a
  combination of the same options.
- **Three separate levels.** How it looks (`[appearance]`), what is there and
  where (`[bar.*]`, `[monitor.*]`, `[zone.*]`, items), and how it works (the
  modules' own tables, such as `[power]`; see [Modules](#modules)) never mix
  in the same table.
- **Short when it can be, long when it must.** Every level of the layout has a
  short form and a long one (see [Syntax](#syntax)).
- **Options flow down.** An option written on an outer level is the default for
  everything inside it; written on an inner level it applies only there.
- **Sensible defaults.** An empty file is a complete bar. Users write only the
  differences.

## Levels

```
bar                  an edge of the screen: where, and how things open
└── node             the bar's content is one node, the root
      ├── node       a group: zones side by side
      ├── node       a row: nodes side by side
      └── node       placed: nodes at the start, the centre and the end
            └── zone     a part of the bar with interactions
                  └── item   one piece of information
```

| Level | What it is | In ashell today | In the prototype |
| --- | --- | --- | --- |
| bar | an edge of the screen: position, outputs, how panels and details open; its content is the root node | the bar | the per-output surface |
| node | zones, or other nodes, with an island round them or none | `modules.left`, `center`, `right`, and each entry of them | the island; the satellite is a second node |
| zone | a part of the bar with a resting row, optional details and a click | a module (`Tempo`, `Settings`…) | a `Zone` |
| item | one indicator or piece of data | an indicator inside `Settings` | `battery()`, `unread()`… |

`start` and `end` rather than `left` and `right`, so the same names work for a
vertical bar on the left or right edge.

### Node

One element, at every depth, holding one of three things:

- **zones**, side by side: a *group*, the node that has the zones' rows and
  opens their panels;
- **nodes**, side by side: a *row*;
- **nodes placed** at its `start`, `center` and `end`, as ashell places its
  modules: the `center` ones stay in the middle of the node as long as the
  others leave room, and move over only when a side would overlap them. The
  bar's root node is always placed.

Each place holds a list of nodes, side by side. Any node can have an island
round it (`shape`); a node with no island is still a node, and its options
still flow down.

How wide a node is follows from what it holds: a group and a row are as wide
as their content, and grow and shrink with it, as the prototype's island does; a
placed node fills the room it has, since it has ends to place things at. In
`center`, where the room is the content's, a placed node puts its three places
side by side at their content's width. Two placed nodes in the same place
share its room.

### Zone

The interactive unit. It has:

- `items`: the resting row, in the order written;
- `details`: a second row shown while the pointer rests on it (empty: no
  details), inside the group's island or split off what holds it (see
  [Details](#details-hover));
- `open`: what a click on it does, usually a panel.

Built-in zones (`workspaces`, `window`, `media`, `clock`, `status`, `tray`,
`updates`, `system`…) come with default items, details and `open`; naming one
uses its defaults, and a `[zone.<name>]` table overrides only the keys written.
A new name is a zone built from scratch.

### Item

One piece of information to show: the battery, the time, the window title.
An item may carry its own `open`, which wins over its zone's for a click on the
item itself (the unread dot opens the notifications even inside the clock's
zone, which opens the calendar).

Instances of the same item share the service's data, never their options.

### Modules

A zone uses data; a module provides it. A module offers:

- **items**, to put in any zone's rows, at rest or in the details;
- **panels**, one or more, for a zone or an item to open;
- **actions**, by name, for its items' gestures to run (see
  [Interactions](#interactions)): `next_timezone`, `toggle_dnd`.

One module can feed many zones, and one zone can hold items of many modules:
the built-in `clock` zone holds the time and the unread dot of the
notifications, which come from two of them.

A module is not a service. The settings module offers the volume, the
network and the battery, each from a service of its own, and one service can
feed more than one module.

A module's own options, about what it does and where its data comes from, go
in a table of its own, never in the layout. Panels are the modules' own: a
panel the user puts together is not planned.

## Syntax

Every level has a short form and a long form:

| Level | Short | Long |
| --- | --- | --- |
| node, group | `"clock"` (one zone) or `["clock", "status"]` | `{ zones = ["clock", "status"], panels = "morph" }` |
| node, row | | `{ nodes = [ …nodes… ], shape = true }` |
| node, placed | | `{ start = [ … ], center = [ … ], end = [ … ] }`; the bar's own keys for the root |
| zone | `"status"` (its defaults) | `[zone.status]` with `items`, `details`, `open` |
| item | `"battery"` | `{ item = "battery", show = "notable", open = "settings" }` |

A place (`start`, `center`, `end`) and `nodes` take a list of nodes, or one
node on its own: `center = "clock"` is `center = ["clock"]`, and `center = {
zones = ["clock"] }` is one group with options. A list inside the list is a
group: `end = ["system", ["clock", "status"]]` is two nodes, `system` alone
and `clock` with `status`.

Options flow down: `hover`, `panels` or `shape` written on a node are
the default for every node inside it; written on an inner node they apply
there and below.

### Examples

Today's ashell default (`modules.left = ["Workspaces"]`, `center =
["WindowTitle"]`, `right = [["Tempo", "Privacy", "Settings"]]`), an island
round each group:

```toml
[bar.main]
panels = "dropdown"

start  = ["workspaces"]
center = ["window"]
end    = [["clock", "privacy", "status"]]
```

A place holds as many islands as it has groups: `end = ["system", ["clock",
"privacy", "status"]]` is two islands at the end.

The island prototype, one island in the centre and, beside it, a group of
its own for what runs hot, shown only then (as a satellite it would split
off the island instead, see [Not decided yet](#not-decided-yet)):

```toml
[bar.main]
hover  = true
panels = "merge"

center = [
  { zones = ["workspaces", "window", "tray", "media", "clock", "updates", "status"], panels = "morph" },
  "hot",
]

[zone.hot]
items = [{ item = "system", show = "notable" }]
open  = "system"
```

A middle ground: the morphing island, with the tray at the end opening its menu
as a panel that stands apart:

```toml
[bar.main]
hover  = true
panels = "merge"
gap    = "s4"

center = [{ zones = ["workspaces", "window", "media", "clock", "status"], panels = "morph" }]
end    = ["tray"]
```

One island as long as the bar, the root's, floating off the screen's edges,
its panels splitting off it; the groups on it have none of their own:

```toml
[bar.main]
shape  = true
padding = "s2"
panels = "merge"

start  = ["workspaces"]
center = ["window"]
end    = [["clock", "privacy", "status"]]
```

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

## Keys

Only the keys decided so far. Others (`layer`, `reserve`, `backdrop`…) are
expected to come but are not settled: see [Not decided yet](#not-decided-yet).

### `[bar.<name>]`

Every bar has a name, used by the monitors to say which bars they have. A
file with none has one, `main`, with the defaults.

| Key | Values | Default | Meaning |
| --- | --- | --- | --- |
| `position` | `top`, `bottom` (later `left`, `right`) | `top` | the edge |
| `start`, `center`, `end` | nodes | `[]` | the root node's places |
| `shape` | `true`, `false` | unset | `true`: an island as long as the bar, the root's; `false`: no island anywhere, unless a node inside says so; unset: an island round each group (see [Islands and background](#islands-and-background)) |
| `background` | `transparent`, `solid` | `transparent` | the surface's own background, in the island's colour and material: flat, edge to edge, with no corners and no border. The panels and details of the groups with no island round them come out of it |
| `padding` | styletto space steps, `s0`…`s10`: one for every side, `[vertical, horizontal]`, or `[top, right, bottom, left]`, as CSS writes them | `s2` | between the screen's edges and the outermost shapes, and on the side away from the edge between them and the background's. `s0` puts them against the edges. The background always runs from edge to edge; there is no margin outside it |
| `gap` | a styletto space step, `s0`…`s10` | `s2` | how far the panels and details stand off what they come out of, a dropdown's included. For those that merge, below `s4` a neck joins them; from `s4` it breaks as they arrive (see [Panels](#panels-panels)) |
| `hover`, `panels` | as on a node | | the defaults for every node |

### `[monitor."<monitor>"]`

| Key | Values | Meaning |
| --- | --- | --- |
| `bars` | list of bar names | the bars on this monitor, at most one per edge |

The monitor is named by its connector (`"DP-2"`) or by any part of its
description (`"Dell U2720Q"`), which stays the same through a dock. See
[Bars and monitors](#bars-and-monitors).

### Node, long form

| Key | Values | Default | Meaning |
| --- | --- | --- | --- |
| `zones` | zones | | a group: the zones, in order |
| `nodes` | nodes | | a row: the nodes, in order |
| `start`, `center`, `end` | nodes | | placed: the nodes at each place |
| `shape` | `true`, `false` | see [Islands and background](#islands-and-background) | an island round this node or not; the default for the nodes inside |
| `hover` | `true`, `false` | `false` | zones show their details on hover |
| `panels` | `morph`, `merge`, `dropdown` | `dropdown` | how the groups inside open panels (see below) |

A node holds one of `zones`, `nodes` or the places; one with more than one is
reported, and the first in that order is kept. `hover`, `panels` and
`shape` flow down.

### Zone (`[zone.<name>]`)

| Key | Values | Meaning |
| --- | --- | --- |
| `items` | list of items | the resting row |
| `details` | list of items | the row shown on hover; empty means none |
| `open` | a panel name, `none` | what a click on the zone does |

### Item, long form

| Key | Values | Meaning |
| --- | --- | --- |
| `item` | an id from the catalogue | which item; required |
| `show` | `always`, `notable`, `never` | when it is visible (semantics not settled, see below) |
| `open` | a panel name, `none` | a click on this item, over its zone's |
| `right_click`, `middle_click`, `scroll_up`, `scroll_down` | an action | see [Interactions](#interactions) |
| other keys | the item's own options | presentation details |

## Behaviour

### Visibility rolls up

An item with nothing to show takes no space, whatever its `show`: its data
has not come yet, its service or device is missing (no battery, no Bluetooth
adapter, no weather configured), or what it tells of is not there (no window
focused, nothing using the microphone). This is not something `show` decides:
`always` means always while there is something to show.

A zone with no visible items takes no space. A group with no visible zones
disappears, shape included, and so does a node with no visible nodes inside.
This is what makes a group appear only when something happens: give it items
that are visible only then.

### Islands and background

Two things can be behind the zones, and they are kept apart:

- **The background** (`background = "solid"`) is the surface's: a flat fill
  in the island's colour and material, from edge to edge of the bar, with no
  corners, no border and no shadow. A group with no island round it opens its
  `merge` panels and its details out of it, as out of an
  island: with no island anywhere, it is the classic strip along the edge,
  panels and all. `padding` moves what is drawn on it away from its edges;
  the background itself stays at the screen's edges, and is as tall as the
  padding on both sides and a group.
- **The island** is styletto's: a shape with the theme's colour, material,
  corners, border and shadow, the same at every depth. `shape` says which
  nodes have one:

  - A group has one, unless a node round it has one or says `shape = false`:
    an island on an island is asked for, and a `false` is the default for
    what is inside.
  - A row or a placed node has one only with `shape = true`, and the nodes
    inside then have none unless they say so.

  | Written | What is drawn |
  | --- | --- |
  | nothing | one island round each group |
  | `shape = true` on the bar | one island as long as the bar, `padding` off the screen's edges |
  | `{ nodes = [...], shape = true }` in each place | one island round each place's groups |
  | `shape = false` on the bar | no island anywhere: the zones alone |

  The root's island is an island like the others: as tall as a group, with
  its corners and border all round, against the screen's edges too with
  `padding = "s0"`. A strip along the edge is the background's.

Nodes inside an island can have their own: with `shape = true` on the bar,
`end = [{ zones = ["status"], shape = true }]` is an island of its own on the
bar's.

The two combine: `background = "solid"` with groups' islands is islands on a
flat bar, and with `shape = false` on the bar a flat bar with nothing on it
but the zones. `solid` under the root's island is allowed, though the island
covers it but for the padding round it.

The island's look is the theme's, and the same everywhere, so a group, the
bar and a panel that merges with them never differ: its corners follow
`[appearance] shape` (`square`, `soft`, `round`), and its border is
styletto's island border (see [Not decided yet](#not-decided-yet)). The
`[bar]` keys `radius` and `border` go: the background has neither, and the
island takes them from the theme.

### Groups appearing

A group that appears or disappears fades, and the nodes of its list slide to
make room or close the gap. `center` is one row centred as a whole, as in
ashell: the row moves as it grows or shrinks. `start` and `end` keep to their
edge.

Satellites, groups that split off a node and merge back, are not decided yet
(see [Not decided yet](#not-decided-yet)).

### Panels (`panels`)

| Value | What a click does | Today |
| --- | --- | --- |
| `morph` | the group itself turns into the panel and back | the prototype |
| `merge` | the panel is a shape of its own that splits off the group, merged with it, and merges back on close. With the bar's `gap` below `s4` it stays joined by a neck; from `s4` the neck breaks as it arrives and it stands apart once out | ashell's menus, merging instead of appearing; apart, the prototype's satellite |
| `dropdown` | the panel drops down from the group's edge, merging with nothing | ashell's menus |

Whether a merged panel stays joined is the bar's `gap`, one for the whole bar
rather than a value per panel: the shapes that merge in one island reach for
each other as far as one distance, so one bar has one way of coming out.

`morph`:

- In `center` the group grows symmetrically, as the prototype's island.
- In `start` and `end` it grows away from the bar's edge: a group in `start`
  keeps its start edge still and grows towards the end.
- Its neighbours in the list step aside while it is open (fade and slide
  away, then come back).

`merge` and `dropdown` both open the panel as a shape of its own:

- It is centred under the zone that opened it, wherever the group is: a group
  with several zones that open panels (`["clock", "privacy", "status"]`) opens
  each under its own zone. It is pushed back inside where centring would take
  it past the ends of the nearest island round the group, when that island
  is wide enough to hold it, or else past the bar's (the screen's less the
  padding, and at least `S2`): a panel of a group in one of two halves stays
  in its half.
  This is what ashell does today (its menus are centred on the button
  pressed and kept 8 px inside the screen), and what an item with its own
  `open`, such as a tray icon, will need: centred under the item.
- The group's neighbours do not move.

`merge`:

- The panel comes out of the nearest island round the zone that opened it:
  the group's, or else that of the nearest node round it that has one, or
  else the background. It comes out of that
  island's edge facing the screen (under it for a top bar), starting as a
  sliver as wide as the group and growing to its own size, joined to the
  island while it grows; on close it goes the same way back. It hangs from
  that edge as long as it is out, and follows it as the island changes size:
  opened from a zone whose details were out, it comes out of the island as
  the details go back. The border runs
  round the island and the panel as one outline. The background has no
  border: out of it only the panel has one, fading out along the neck. A
  border comes only with a shape.
- Closing, the panel's content fades first and its shape then sinks back,
  easing.

`dropdown`:

- The panel drops down from the edge of the nearest island, or else of the
  bar, as ashell's menus do today: as wide as it is from the start,
  its height growing from the edge with its content cut, not squeezed, and
  the gap to the bar opening as it drops. Narrowing would only read as a box
  going flat, with no shape for it to become part of; and it never crosses the
  bar, which a translucent or glass bar would show. GNOME, Plasma, Windows and
  macOS keep a popup's width as well.
- Closing, it goes back up at once, easing.

A panel needs an island for what it does, and drops down as with `dropdown`
when there is none, the file being told so:

- `merge` needs an island round the zone to come out of,
  the group's or a node's round it, or else the background. With `shape =
  false` on the bar and no background there is none.
- `morph` needs the group's own island to turn into: a node's island round it
  does not grow into a panel. A morph with an island of its own
  only while it is open is a direction for later (see
  [Not decided yet](#not-decided-yet)).

A panel stands `gap` past what it comes out of. Two shapes that far apart
keep a neck while the gap is less than half of how far they reach for each
other, and swell towards each other until the gap is as far as they reach.
With a gap below `S4` they reach the merge distance, `S8` (32 px): the panel
stays joined by a neck, thick for the default `s2`. From `S4` they reach a
third more than the gap: the panel comes out joined, the neck breaks as it
arrives, and once apart they hardly swell.
The sliver it starts and ends as is kept a merge distance inside the group's
ends, so letting it go on close leaves no bulge to snap away.

Panels are the existing ones (`settings`, `calendar`, `notifications`,
`media`, `system`, `tray`, `updates`, `launcher`): this document decides only
how they open, not what they contain.

### Details (`hover`)

With `hover = true`, the pointer resting on a zone shows its `details`, a
second row of items. Where it shows depends on the shape round the group:

- **The group has its own island**:
  the row is added to the island, away from the bar's edge (under the zones
  on a top bar, over them on a bottom one), and the island grows to hold it.
  The zones stay where they are.
- **It has none**: the row is a small island of its own that splits off what
  holds the group, the nearest node's island round it, the background, or
  the group's place when there is none, as a `merge` panel does, and goes back
  the same way. The island round the group keeps its height, and nothing is
  drawn without a shape under it. With nothing round the group it is an
  island alone, with the island's border and shadow, as a dropdown has.

The split-off row is drawn as a group's island would be: as tall as a group,
with pill corners. It comes out of what holds it and goes back into it as a
`merge` panel does, from and to a sliver under the zone, so nothing is left
to snap away when it is gone; its row fades in once it is out, without
squeezing while the island grows, and fades out first. There is one per
list of nodes: going from one zone to
another of the same list, it stays out, slides under the new zone and
changes its row, as an island does, rather than going back and coming out
again. It comes out from under the zone. In `start` its
start edge is on the zone's, in `end` its end edge, so it stays on the screen
whatever its width; in `center` it is centred on the zone. The place is the
one the group is in, the nearest round it. It stands half the bar's `gap`
off while that keeps a neck, for a thick one, whatever the group's panels;
with a `gap` that breaks the neck it stands as far off as a panel, apart too.

The pointer on the split-off row counts as on the group: the row stays while
the pointer goes from the zone to it.

The row comes once the pointer stays on a zone a moment, and goes a moment
after it has left the group. Between zones of the group it changes sooner.
On a zone with no details the row shown goes, whether it is that group's or
another's, and the pointer reaching a zone straight from another group
brings that zone's row without waiting for one more move.

### Interactions

A click opens a panel, with `open`. The other gestures run an action, and
are written on an item, the same for every item:

| Key | Gesture |
| --- | --- |
| `right_click` | the right button |
| `middle_click` | the middle button |
| `scroll_up`, `scroll_down` | the wheel, or the touchpad gathered into steps: an action runs once a step, not once an event |

An action is one of:

- an action of the item's module, by name: `"cycle_format"`,
  `"next_timezone"`, `"toggle_dnd"`, `"volume_up"`;
- a panel to open, `{ open = "calendar" }`;
- a command to run, `{ run = "pavucontrol" }`, which covers the custom
  modules and the "more" commands of today's ashell;
- `"none"`, to switch off a default.

Every item comes with its own defaults, today's ashell's, and the file
writes only what differs:

```toml
[zone.clock]
items = [
  { item = "clock", formats = ["%R", "%T"] },           # right click cycles them, the wheel the time zones
  { item = "date", scroll_up = "none", scroll_down = "none" },
]
```

- A gesture with nothing bound does nothing: only `open` falls back to the
  zone's. Zones have `open` and the details on hover, no other gestures.
- An item made of parts, the workspaces or the tray's icons, handles a click
  on a part itself (it goes to that workspace, it activates that app); the
  gestures written on it are for the item as a whole.
- Turning the wheel the other way is binding the actions the other way
  round, `scroll_up = "previous_workspace"`: no option of its own.
- Hovering shows a zone's details (see [Details](#details-hover)); there are
  no tooltips on items: what ashell's tooltips tell goes in the details.

### Bars and monitors

A bar is a copy, one to one, on every monitor that has it, and a monitor has
at most one bar per edge:

- `[monitor."<monitor>"] bars = [...]` gives a monitor its bars; `bars = []`
  gives it none. The first entry that matches a monitor decides.
- `[monitor."*"]` is every monitor no entry matches: a laptop on its own, a
  projector. Without it, those monitors have every bar, the first on each
  edge. So a file with no `[monitor]` puts every bar on every monitor, and a
  monitor that is not connected never leaves the others without a bar.
- A list with two bars on the same edge, or a name with no bar, is reported
  and the bar left out.

```toml
[bar.main]
center = ["clock"]

[bar.media]
position = "bottom"
center   = ["media"]

[monitor."Dell U2720Q"]
bars = ["main", "media"]

[monitor."DP-2"]
bars = []

[monitor."*"]
bars = ["main"]
```

Plugging a monitor in or out, or saving the file, applies the rules again.
A bar only on the monitor that has the focus (`active`) and one that comes
only when called (`elsewhere = "on-demand"`) wait for the compositor's
services.

### Surfaces

Each bar is one layer-shell surface per output, transparent, as big as the
compositor gives, for good: a panel needs room up to the far edge, and a size
asked for anew may be more than the compositor gives, which cuts the bar off
for a frame. Its buffer goes only where something is drawn, and it takes
input only on its shapes. While a panel is open the whole surface takes
input, so a click anywhere outside closes the panel, and it takes the
keyboard, so <kbd>Esc</kbd> closes it too. The reserved space is the bar's
padding and a group, and never changes.

## Appearance

`[appearance]` is styletto's `UiConfig` unchanged (theme, seed or wallpaper,
mode, fonts, density, shape, material, motion). The migration of ashell's
appearance to it is complete and deliberately lossy. The bar's padding is a
bar key in styletto's steps rather than in pixels, written as ashell writes
its `padding` (see [`[bar.<name>]`](#barname)); its radius and border are the island's, the theme's
(see [Islands and background](#islands-and-background)). These keys of today's
ashell have no counterpart and go away:

- `appearance.bar.margin`, and its `radius` and `border` in pixels;
- `appearance.modules.*` (grouping, spacing, colours, font size per module;
  parsed today but not rendered);
- `font_weight`, `scale_factor`;
- the per-surface `opacity` (`material` decides opacity and blur);
- the individual colours, replaced by the theme or a seed.

## Not decided yet

- **Visibility.** What `always`, `notable` and `never` mean exactly; whether
  `notable` stays one rule per item or becomes a named condition the user
  picks (`bluetooth` is visible while on in ashell, while connected in the prototype;
  `network` always in ashell, only on trouble in the prototype). An empty
  item never shows (see [Visibility rolls up](#visibility-rolls-up)), so
  `notable` is only about an item that has something to show and says
  whether it is worth showing. The proposal to start from, to be checked
  item by item against ashell's `modules/settings` and the prototype's
  `compact::status`; a `notable` left blank is still to be thought out, or
  makes no sense for the item and is then the same as `always`:

  | Item | Shows | Empty when | `notable` when | Parameters |
  | --- | --- | --- | --- | --- |
  | `workspaces`, `workspace_list` | compact workspaces; the list with names and windows | never | | |
  | `window` | the focused window | no window is focused | | |
  | `window_list` | the windows of the workspace | there are none | | |
  | `launcher_button` | the launcher's button | never | | |
  | `clock`, `date` | the time; the date | never | | |
  | `weather` | sky and temperature | no weather yet, or none configured | it rains, snows or storms | |
  | `forecast` | the next hours | no weather yet, or none configured | rain, snow or a storm is coming in them | |
  | `notifications` | an unread dot | nothing is unread | | |
  | `notices` | the latest notifications | there are none | | |
  | `media` | the track | no player | a player plays | |
  | `media_details` | track, progress and controls | no player | | |
  | `tray` | the tray's icons | no icons | | |
  | `updates` | the number of updates | not checked yet | above a threshold | `notable_above` (40) |
  | `system`, `cpu`, `memory`, `temperature`, `disk` | a summary; one metric | not read yet, or no such sensor | past a threshold | `notable_above`, per metric for `system` |
  | `privacy` | microphone, camera, screen share in use | nothing is in use | | |
  | `airplane`, `dnd`, `idle_inhibitor`, `vpn` | each one's state | its service is missing | it is on | |
  | `power_profile` | the power profile | no profiles daemon | it is not balanced | |
  | `bluetooth` | Bluetooth | no adapter | a device is connected | |
  | `microphone`, `volume` | each one's state | no device | it is muted | |
  | `brightness` | the brightness | no backlight | | |
  | `network` | the connection | never | there is none, or it is limited | |
  | `battery` | the battery | no battery | below a threshold, or charging | `notable_below` (20) |
  | `peripheral_battery` | the devices' batteries | no device reports one | below a threshold | `notable_below` (20) |
  | `keyboard_layout` | the layout | never | there is more than one | |
  | `keyboard_submap` | the active submap | none is active | | |

- **Item options.** Beyond `item`, `show` and `open`, read today and kept for
  the item: a `style` of the item's own (`battery` as `ring` or
  `icon-percentage`), the `notable_*` parameters, options such as `format`,
  and an `[item.<id>]` table for the options shared by every instance, which
  those written in a list win over. `open` could also take `{ run =
  "<command>" }`. Presentation formats of today's ashell
  (`battery_format`, the `*_indicator_format`, the workspaces'
  `indicator_format`, the media player's `indicator_fields`) would become
  styles or options of their items. Custom modules would be
  `custom:<name>` items.
- **Zone options.** A `hover` of its own; `extends`, to start from another
  zone's values; `{ from = "<zone>" }` in a row, to insert another zone's
  items with the options written beside it, as the prototype's status
  details show the status items, all visible. Items too wide for the row
  they land in (`window_list`, `workspace_list`, `media_details`) shrink as
  best they can, for example to icons; how is not decided.
- **Modules.** Which modules there are, which items and panels each offers,
  and what goes in each one's table. Whether modules also offer ready-made
  zones, as shorthands, in place of the built-in ones; if they do, whether
  such a zone holds only its module's items, since today's `clock` holds the
  unread dot and `status` the privacy indicator. The starting point is the
  data and the commands of today's ashell, gathered by what they are about:

  | About | Today |
  | --- | --- |
  | time and date: format, formats to cycle, time zones | `tempo.clock_format`, `tempo.formats`, `tempo.timezones` |
  | weather: location, units | `tempo.weather_location`, `tempo.wind_speed_unit`, the prototype's `[weather]` |
  | updates: `check_cmd`, `update_cmd`, interval | `[updates]` of ashell and the prototype |
  | power: `lock`, `suspend`, `hibernate`, `reboot`, `shutdown`, `logout` | `settings.*_cmd`, the prototype's `[power]` |
  | audio: volume step, maximum volume, extra commands | `settings.volume_step`, `settings.max_volume`, `settings.audio_*_cmd` |
  | network and Bluetooth: extra commands | `settings.wifi_more_cmd`, `vpn_more_cmd`, `bluetooth_more_cmd` |
  | workspaces: which ones, names, maximum, inverted scrolling | `[workspaces]` |
  | window: title or class, truncation | `[window_title]` |
  | system: interval, sensors, disks | `[system_info]` |
  | notifications: history, blocklist, grouping | `[notifications]` |
  | launcher: terminal | the prototype's `[launcher]` |
  | custom: command, interval, icons | `[[CustomModule]]` |

- **Actions.** Each module's list of actions, and the items' default
  gestures. Whether an action's name is also its IPC command
  (`msg tempo next_timezone`).
- **Vertical details.** Details as a column rather than a row, for what
  needs more room than a row has, as a list of connected devices.
- **Transients.** Where OSD, toasts, alerts and messages go. The proposal:
  each kind of event has a destination, decided on the event's side so two
  groups cannot compete for the same kind.

  ```toml
  [transients]
  osd       = { to = "group:main" }
  workspace = { to = "group:main" }
  alert     = { to = "group:main" }
  toast     = { to = "screen", position = "top-right", stack = 5 }
  message   = { to = "off" }
  ```

  | Kind | What | In the prototype |
  | --- | --- | --- |
  | `osd` | volume, brightness, microphone | `Transient::Volume`, `Brightness`, `Microphone` |
  | `workspace` | a workspace switch | `Transient::Workspace` |
  | `toast` | a notification that just arrived | `Transient::Toast` |
  | `alert` | a critical battery and the like | `Transient::Alert` |
  | `message` | short confirmations (an IPC command, a copy) | `Transient::Message` |

  `to` is `group:<name>`, `screen` or `off`; `position` (the four corners,
  `top-center`, `bottom-center`) and `stack` (how many at once, then queued)
  go with `screen` only, and `duration_ms` overrides the default duration.
  Inside a group the event takes the resting row's place for its duration,
  with the prototype's priorities (`Transient::priority`): alert, urgent
  toast, toast, message, OSD. On the screen it is an island of its own, with
  no zones: today's ashell OSD and toasts. With several bars or monitors it
  goes to the group of that name on the active monitor, as the prototype's
  `Islands::focused()`; whether groups of the same name on different bars
  need telling apart is open. A group that does not exist is reported, and
  the event goes to the top centre of the screen.
- **Satellites.** A temporary node that grows out of a node it belongs to,
  as the prototype's bubble of what runs hot out of its island. What is settled:
  - It is written on the node it comes out of, under the side it takes, so
    there is nothing to name: `{ zones = ["clock"], satellites = { end =
    ["hot"] } }`.
  - That node does not move; the nodes on the satellite's side are pushed
    out. Before the first node of `start` or after the last of `end` the node
    is at the edge, and it moves to make room.
  - In `center` the row stays centred on its own nodes: a satellite leaves it
    lopsided while it is out.
  - With shapes round them it splits off its node, as panels do: joined by a
    neck, as the prototype's bubble, or apart, as the bar's `gap` says. With
    no shapes it fades.
  - It comes out when it has something to show, by the visibility rules: at
    least one of its items visible, so a `show = "notable"` item brings it out
    when there is something to note, and one with every item `always` is out
    whenever its node is at rest.
  - It is out only while its node is at rest: when the node shows anything
    else, a `morph` panel or a transient, it merges back into it, as in
    the prototype.

  It waits on the visibility rules (step 7).
- **The island's border in the theme.** styletto draws every island with a
  hairline border and has no `[appearance]` key for it. A theme-wide
  `border` (none, hairline, bold…) is the likely home, so the bar, its groups
  and its panels keep one outline.
- **Islands round nodes.** A row's or a placed node's island holds groups,
  and the panels of its groups split off it. A group in it morphs only with
  an island of its own.
- **A morph with no island.** With no island round a group a morph panel drops down
  today. It could instead grow an island out of the group's row while it is
  open and give it back on close, keeping the prototype's gesture without an island
  at rest.
- **Shapes.** Whether a shape per zone (`per-zone`) or per item (`per-item`,
  ashell's unrendered `grouping = "individual"`) is worth supporting.
- **Distance from the edge.** The prototype's `island.seed.top` sets where the island
  rests and the reserved space; a bar's `padding` may be its home.
- **Names for transients.** A group needs a `name` only to be addressed by a
  transient; whether it is optional, defaulting to the place it is in.
- **Workspace colours.** ashell colours each workspace by its monitor
  (`workspace_colors`). Two parts, for later: a categorical series in styletto
  (`tints`, from the theme's accents or the seed's roles), and a `color_by =
  "monitor" | "workspace" | "none"` option on the `workspaces` item.
- **Bar keys** still to come: `layer` (`bottom`, `top`, `overlay`; `top`
  today), `reserve` (the space taken from windows, `auto` for the bar's
  padding and a group, or pixels, if floating shapes on some compositors
  need it), `backdrop` (0 to 1, how much the screen dims while a panel is
  open), `elsewhere` (`none`, or `on-demand` for a bar that comes to the
  other monitors only when called, as the launcher), and `active` monitors.
- **Appearance per surface.** Partial `[appearance]` tables over the whole,
  for the bar, the panels, the on-screen transients or one group by name
  (`[appearance.surfaces.panel]`, `[appearance.surfaces."group:main"]`), as
  styletto lets each surface `provide_ui` its own `Ui`. Only the `Ui` choices
  (`density`, `shape`, `material`, `motion`, `fonts` and the colours of each
  side); the colours' source stays one.

## From today's configuration

The configuration is new and deliberately incompatible, so nothing migrates
by itself; an `ashell migrate` that writes the new file from the old one
could follow this table.

| ashell today | Here |
| --- | --- |
| `position` | the bar's `position` |
| `outputs` | `[monitor."<monitor>"] bars` |
| `modules.left`, `center`, `right` | the bar's `start`, `center`, `end` |
| a group, `[["Tempo", "Privacy", "Settings"]]` | a group, `[["clock", "privacy", "status"]]` |
| a module (`Tempo`, `Settings`…) | a built-in zone (`clock`, `status`…), and the module its items come from |
| `appearance.bar.surface` | the bar's `background` |
| `appearance.modules.*.grouping` | `shape` on the nodes |
| `settings.indicators` | `items` of `[zone.status]` |
| `settings.battery_hide_when_full` | `show = "notable"` on `battery` |
| `system_info.indicators` | `items` of `[zone.system]` |
| `settings.remove_*_btn` | the `settings` panel's content: none |
| `appearance.*_color`, `font_name`, `opacity`, `blur` | styletto's `[appearance]` (see [Appearance](#appearance)) |
| `animations.enabled` | `[appearance] motion` |
| `layer`, `appearance.menu.backdrop` | the bar's `layer` and `backdrop`, not decided |
| `osd.enabled`, `notifications.toast`, `toast_position`, `toast_limit` | transients, not decided |
| the formats of the indicators | the items' styles and options, not decided |
| the modules' data and commands (`tempo`, `updates`, `settings.*_cmd`…) | the modules' tables, not decided |
| `[[CustomModule]]` | `custom:<name>` items, not decided |

## Implementation plan

Each step ends with something that runs, and with tests for what does not
need a screen.

1. **Config, parsed.** Done. Types for bar, node, zone and item with their
   short and long forms, hand-written readers with problems that name where
   they are, options flowing down resolved into one plain tree. Built-in zone
   defaults. Tests on the examples in [`examples`](examples).
2. **A bar on screen.** Done but for the items: one surface per output, the
   nodes laid out, the islands where they are asked for and the background,
   `[appearance]` through `provide_ui`. Zones and panels are placeholders
   sized like the real ones.
3. **Panels, `dropdown` and `merge`.** Done. A click on a zone
   opens its panel under that zone; a click outside or <kbd>Esc</kbd>
   closes it.
4. **Panels, `morph`.** Done. The group turns into the panel; growth away
   from the edge in `start` and `end`; neighbours stepping aside.
5. **Details on hover.** Done. The zone's second row: in the group's island,
   or split off what holds the group (see [Details](#details-hover)).
6. **The first modules.** Items and panels with real data, from the services
   of `ashell-services`, in place of the placeholders, translated from the
   start through `ashell-i18n`.
7. **Visibility rolling up.** Items that hide, zones and groups that take no
   space, with a fade, as the visibility rules are settled.
8. **The other modules**, as their services move to `ashell-services`.

Interactions come in with the modules whose actions they run. Satellites,
transients and the rest of the "not decided" list come in once they are
decided here.
