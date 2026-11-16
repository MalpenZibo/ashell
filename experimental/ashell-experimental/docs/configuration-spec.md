# Unified ashell configuration

Status: draft, 2026-10-22.

The new configuration describes the bar with four nested concepts (bar, group,
module, item) and, with the same file, produces either today's ashell or
the island prototype. It is a new configuration, incompatible with the current one,
meant for an ashell written in GUIdo on top of styletto.

## Goal and principles

- **One grammar, two experiences.** Classic ashell and the morphing island are two
  configuration files, not two modes in the code. Every middle ground is a
  combination of the same switches.
- **Three separate levels.** How it looks (appearance), what is there and where
  (layout and interaction), and how it works (features) never mix in the same
  table.
- **The item is the unit.** A module is only a shorthand for its default items.
  Per-module indicator lists and scattered format fields go away.
- **No condition language.** Visibility has three values; what counts as
  "notable" is decided by the item, with declared and documented parameters.
- **Sensible defaults.** An empty file yields a complete preset. Users write only
  the differences.

Out of scope for this spec: allock's configuration, the content of the panels
(they stay as they are) and interaction timings (hover delays and the like,
already solved in the prototype).

## The three levels

| Level | Tables | Decides | Never decides |
| --- | --- | --- | --- |
| Appearance | `[appearance]` | colours, density, shapes, material, motion, fonts | what is there and where |
| Layout and interaction | `[[bar]]`, `[[bar.group]]`, `[module.*]`, `[item.*]`, `[transients]` | which items, in what order, where, when they are visible, what happens on hover and click | colours and commands |
| Features | `[clock]`, `[weather]`, `[updates]`, `[power]`, `[launcher]`, `[workspaces]`, `[audio]`, `[network]`, `[system]`, `[notifications]`, `[custom.*]` | commands, services, intervals, data formats | where something appears |

A rule of thumb for where an option belongs: if changing it changes **what is
shown or where**, it is layout; if it changes **how it looks**, it is
appearance; if it changes **what it does or where its data comes from**, it is a
feature.

## Vocabulary

Four nested levels: a bar contains groups, a group contains modules, a module
contains items in two rows.

```
bar · top                         background, hover, panels, outputs
├── group · start                 workspaces
├── group · center                shape = island
│   ├── module · window
│   │   └── items: window
│   └── module · status
│       ├── items (at rest):      privacy, battery
│       ├── details (on hover):   system, updates
│       └── open = settings
└── group · end                   clock, status
```

Clicking an item uses its own `open`; clicking the module's background uses the
module's `open`.

| Level | What it is | In ashell today | In the prototype |
| --- | --- | --- | --- |
| `bar` | a screen edge, with background, hover, panels | the bar | the per-output surface |
| `group` | an anchored container, with or without a shape | a group in `left`, `center`, `right` | the island |
| `module` | a unit with a resting row, details and a click | a module (settings, tempo…) | a `Zone` |
| `item` | an indicator, a piece of data or a view | a settings indicator | `battery()`, `trouble()`, `unread()`… |

"Island" stays the name of a styletto shape: a group draws it or not.

## Item

An item is the smallest piece: an indicator (the battery), a piece of data (the
time) or a view (the window list). Every time it appears in a list it is an
**instance** with its own options; instances of the same item share only the
service's data.

It is written as a name (`"battery"`) or as a table with options:

```toml
{ item = "battery", show = "notable", notable_below = 20, style = "ring", open = "settings" }
```

| Key | Values | Default | Meaning |
| --- | --- | --- | --- |
| `item` | an id from the catalogue, or `custom:<name>` | required | which item |
| `show` | `always`, `notable`, `never` | per item | when it is visible |
| `notable_*` | the item's own parameters | per item | the `notable` threshold |
| `style` | the item's own styles | per item | how it is presented |
| `open` | a panel, `none`, or `{ run = "<command>" }` | per item | what a click does |
| other keys | the item's own options (`format`, `fields`…) | per item | presentation details |

Options shared by all instances of an item go in `[item.<id>]`; those written in
the list win. An unknown key is logged and ignored.

### Visibility

- `always`: always visible.
- `notable`: visible only when the item has something to say. The rule belongs
  to the item; its parameters are declared in the catalogue.
- `never`: never. Useful to switch off an item that a default module brings
  along.

### Initial catalogue

| Item | Shows | `notable` when | Parameters | Default `open` |
| --- | --- | --- | --- | --- |
| `workspaces` | compact workspaces | always | | none (click on a workspace) |
| `workspace_list` | detailed list: number, name, windows | always | | none |
| `window` | title and icon of the focused window | a window is focused | | none |
| `window_list` | windows of the workspace or of all | there are windows | | none |
| `launcher_button` | launcher button | always | | `launcher` |
| `clock` | the time | always | | `calendar` |
| `date` | the date | always | | `calendar` |
| `weather` | sky icon and temperature | data available | | `calendar` |
| `forecast` | forecast for the next hours | data available | | `calendar` |
| `notifications` | dot for unread notifications | there are unread ones | | `notifications` |
| `notices` | summary of the latest notifications | there are notifications | | `notifications` |
| `media` | current track, compact | a player is playing | | `media` |
| `media_details` | track, progress, controls | there is a player | | `media` |
| `tray` | tray icons | there are icons | | the icon's menu |
| `updates` | number of updates | above the threshold | `notable_above` (40) | `updates` |
| `system` | CPU, memory, temperature summary | past the thresholds | per-metric thresholds | `system` |
| `cpu`, `memory`, `temperature`, `disk` | one metric | past the threshold | `notable_above` | `system` |
| `privacy` | microphone, camera, screen share in use | something is in use | | none |
| `airplane` | airplane mode | on | | `settings` |
| `dnd` | do not disturb | on | | `settings` |
| `idle_inhibitor` | idle inhibitor | on | | `settings` |
| `power_profile` | power profile | not balanced | | `settings` |
| `bluetooth` | Bluetooth | a device connected | | `settings` |
| `microphone` | microphone | muted | | `settings` |
| `volume` | volume | muted | | `settings` |
| `brightness` | brightness | never on its own | | `settings` |
| `network` | network | no connection or limited connectivity | | `settings` |
| `vpn` | VPN | connected | | `settings` |
| `battery` | battery | below the threshold or charging | `notable_below` (20) | `settings` |
| `peripheral_battery` | device batteries | below the threshold | `notable_below` (20) | `settings` |
| `keyboard_layout` | keyboard layout | more than one layout | | cycle the layout |
| `keyboard_submap` | active submap | a submap is active | | none |
| `custom:<name>` | a custom module | per its command | | per its command |

The thresholds in brackets are the defaults; the one for `updates` is the prototype's
`WORTH_SAYING = 40`. The catalogue is a starting point: the `notable` rules need
checking item by item against the current behaviour of ashell and the prototype.

## Module

A module is the interactive unit of the bar: a row of items at rest, a row of
details that appears on hover, a panel for clicks on its background. It matches
a module of today's ashell and a `Zone` of the prototype.

```toml
[module.status]
items = ["privacy", { item = "idle_inhibitor", show = "notable" }, "battery"]
details = ["system", "updates", { from = "status", show = "always" }]
open = "settings"
hover = true
```

| Key | Values | Meaning |
| --- | --- | --- |
| `items` | list of items | the resting row, in the order written |
| `details` | list of items | the row below, on hover; empty = no details |
| `open` | a panel, `none`, `{ run = "…" }` | click on the module's background; a clicked item uses its own `open` |
| `hover` | `true`, `false` | overrides `bar.hover` for this module |
| `extends` | a module name | starts from another module's values |

- **No placement constraints.** Any item can go in `items` or in `details`. If an
  item does not fit (the window list in a narrow row) it shrinks as best it can,
  for example to icons only, and the log says so.
- **`{ from = "<module>" }`** inserts all the items of another module, with the
  options written next to it. It covers the prototype's case where the status details
  show the same items, all visible.
- **Default modules are shorthands.** Naming `"status"` in a group means the
  `status` module with its defaults; `[module.status]` overrides only the keys
  written. A new name (`[module.mine]`) is a module built from scratch, or from
  `extends`.

A module with no visible items takes no space. A group with no visible modules
disappears, shape included.

### Default modules

| Module | Default `items` | Default `details` | `open` |
| --- | --- | --- | --- |
| `workspaces` | `workspaces` | `workspace_list` | none |
| `window` | `window` | `launcher_button`, `window_list` | none |
| `media` | `media` | `media_details` | `media` |
| `clock` | `weather`, `date`, `clock`, `notifications` | `date`, `forecast`, `notices` | `calendar` |
| `status` | `privacy`, `airplane`, `dnd`, `idle_inhibitor`, `bluetooth`, `microphone`, `volume`, `network`, `battery` (all `notable` except `battery`) | `system`, `updates`, all `status` items with `show = "always"` | `settings` |
| `system` | `cpu`, `memory`, `temperature` | the same with values | `system` |
| `tray` | `tray` | extended `tray` | none |
| `updates` | `updates` | none | `updates` |
| `keyboard` | `keyboard_layout`, `keyboard_submap` | none | none |

These defaults follow the prototype. The classic ashell preset overrides them where
needed, with no details and with status indicators always visible.

## Bar and group

A bar is a screen edge; its groups are the positioned containers that hold the
modules. The bar's background and the groups' shape are independent, as they are
in ashell today (`bar.surface` and `grouping`).

```toml
[[bar]]
position = "top"
outputs = "all"
background = "transparent"
panels = "popup"
hover = false

[[bar.group]]
name = "left"
anchor = "start"
shape = "island"
modules = ["workspaces"]
```

### `[[bar]]` keys

| Key | Values | Default | Meaning |
| --- | --- | --- | --- |
| `position` | `top`, `bottom` | `top` | the edge |
| `outputs` | `all`, `active`, list of names | `all` | the outputs it always sits on |
| `elsewhere` | `none`, `on-demand` | `none` | on other outputs: absent, or only when called (the launcher) |
| `layer` | `bottom`, `top`, `overlay` | `top` | the layer-shell layer |
| `background` | `transparent`, `solid` | `transparent` | the full-width strip behind the groups |
| `reserve` | `auto`, a number of pixels | `auto` | space taken from windows; `auto` = the bar's height |
| `hover` | `true`, `false` | `false` | module details on hover |
| `panels` | `popup`, `morph` | `popup` | how a panel opens (see Panels) |
| `backdrop` | 0 to 1 | 0 | how much the screen dims while a panel is open |
| `group` | list of `[[bar.group]]` | | the containers |

More than one `[[bar]]` is allowed, for example one at the top and one at the
bottom.

### `[[bar.group]]` keys

| Key | Values | Default | Meaning |
| --- | --- | --- | --- |
| `name` | text | required | used to address transients and appearance overrides |
| `anchor` | `start`, `center`, `end` | required | where it sits in the bar |
| `shape` | `island`, `per-module`, `none` | `island` | one shape around the group, one per module, none |
| `modules` | list of module names | required | the content, in the order written |

Several groups with the same `anchor` line up in the order they are written.

### Fixed behaviour

- **One surface.** Each bar is one layer-shell surface per output, full width and
  transparent, taking input only on the shapes. It is the architecture the prototype
  already uses (`AREA_HEIGHT` = 700).
- **Neighbours step aside.** When a group grows (details, a `morph` panel, a
  transient) and would touch a neighbour, the neighbour fades and slides away,
  then comes back. Not configurable for now.
- **Reserved space.** With `reserve = "auto"` it is the bar's height. With
  floating groups the user can write it in pixels; it replaces the prototype's temporary
  `GAP_OVERLAP`.

## Transients

Each kind of event has a destination: a group, a point on the screen, or
nothing. The decision sits on the event's side, so two groups cannot compete for
the same kind.

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
| `workspace` | workspace switch | `Transient::Workspace` |
| `toast` | a notification that just arrived | `Transient::Toast` |
| `alert` | critical battery and the like | `Transient::Alert` |
| `message` | short confirmations (an IPC command, a copy) | `Transient::Message` |

| Key | Values | Meaning |
| --- | --- | --- |
| `to` | `group:<name>`, `screen`, `off` | where it goes |
| `position` | `top-left`, `top-center`, `top-right`, `bottom-left`, `bottom-center`, `bottom-right` | only with `screen` |
| `stack` | number | only with `screen`: how many at once, then queued |
| `duration_ms` | number | overrides the default duration |

- **Inside a group** the event takes the place of the resting row for its
  duration, with the prototype's priorities and queue (`Transient::priority`): alert,
  then urgent toast, then toast, then message, then OSD.
- **On the screen** the event is an ephemeral island: the same styletto shape,
  with no modules, appearing at the given position. It is the equivalent of
  today's ashell OSD and toasts.
- **With several bars or outputs** the event goes to the group with that name on
  the active output, as `Islands::focused()` does in the prototype.
- **A destination group that does not exist** is a configuration error: the log
  reports it and the event goes to the top centre of the screen.

## Panels

The panels are the existing ones; the configuration decides only the container
they open in, with `bar.panels`.

| Value | What a click does | Equivalent to |
| --- | --- | --- |
| `popup` | the panel opens as a separate shape under the clicked module | ashell's menus |
| `morph` | the group holding the module turns into the panel | the prototype's island |

Panel names usable in `open`: `settings`, `calendar`, `notifications`, `media`,
`system`, `tray`, `updates`, `launcher`. `popup` needs no new surfaces: it is
drawn in the bar's surface, under the module.

`bar.backdrop` dims the rest of the screen while a panel is open, in both modes.
Since the bar's surface is only 700 px tall, the backdrop needs either extending
it to the whole screen when the panel opens, or a second dedicated surface.

## Appearance

`[appearance]` is styletto's `UiConfig` unchanged: the same keys as in the
styletto README and the storybook's Theme page.

```toml
[appearance]
theme = "catppuccin"          # or seed = "#6750a4", or wallpaper = "~/…"
dark_flavor = "mocha"
light_flavor = "latte"
accent = "mauve"
variant = "tonal-spot"        # with a seed or a wallpaper
contrast = "standard"
mode = "auto"                 # light, dark
density = "default"           # compact, comfortable
shape = "soft"                # square, round
material = "translucent"      # solid, glass
motion = "full"               # reduced, none

[appearance.fonts]
sans = "Inter"
mono = "JetBrains Mono"

[appearance.dark]
accent = "#ff79c6"
```

### Per-surface overrides

In styletto each surface declares its own `Ui` with `provide_ui`. The
configuration exposes it through partial tables applied over `[appearance]`:

```toml
[appearance.surfaces.bar]          # the strip and the groups
material = "solid"

[appearance.surfaces.panel]        # panels, in popup or morph
material = "glass"

[appearance.surfaces.transient]    # on-screen transients
density = "compact"

[appearance.surfaces."group:main"] # a single group, by name
shape = "round"
```

Only the `Ui` choices can be overridden: `density`, `shape`, `material`,
`motion`, `fonts` and the `dark` and `light` colours. The colour source (theme,
seed, wallpaper) stays one for the whole shell.

No separate `opacity` and `blur` as in the prototype: `material` decides them.

## Features

The tables at this level configure services and commands. They never decide
where something appears: the same service feeds every item that uses it.

| Table | Content | Where it comes from today |
| --- | --- | --- |
| `[clock]` | time and date `format`, `formats` to cycle, time zones | `tempo.clock_format`, `tempo.formats`, `tempo.timezones` |
| `[weather]` | location, units | `tempo.weather_location`, the prototype's `[weather]` |
| `[updates]` | `check_cmd`, `update_cmd`, `interval` | `[updates]` of ashell and the prototype |
| `[power]` | `lock`, `suspend`, `hibernate`, `reboot`, `shutdown`, `logout` | `settings.*_cmd`, the prototype's `[power]` |
| `[audio]` | `volume_step`, `max_volume`, extra commands | `settings.volume_step`, `settings.max_volume`, `audio_*_cmd` |
| `[network]` | extra commands for Wi-Fi and VPN | `settings.wifi_more_cmd`, `vpn_more_cmd` |
| `[bluetooth]` | extra command | `settings.bluetooth_more_cmd` |
| `[workspaces]` | which workspaces, names, maximum, inverted scrolling | ashell's `[workspaces]` |
| `[window]` | mode (title, class…) and truncation | ashell's `[window_title]` |
| `[system]` | interval, sensors, disks | ashell's `[system_info]` |
| `[notifications]` | history, blocklist, grouping | ashell's `[notifications]` |
| `[launcher]` | terminal | the prototype's `[launcher]` |
| `[custom.<name>]` | command, interval, icons | ashell's `[[CustomModule]]` |

Formats that are about presentation today (`battery_format`,
`*_indicator_format`, the workspaces' `indicator_format`, the media player's
`indicator_fields`) move to the layout level as the item's `style` or options.

## Test configuration: today's ashell

This file reproduces the features of ashell's current default (`main` on
22 October 2026): three islands, no hover details, separate menus, no OSD,
toasts at the top right. The values come from the `Default`s in
`src/config.rs`.

```toml
# Classic ashell

[appearance]
material = "solid"            # today's opacity 1.0
motion = "none"               # today's animations.enabled = false

[[bar]]
position = "top"
outputs = "all"
layer = "top"
background = "transparent"    # bar.surface = transparent
hover = false
panels = "popup"
backdrop = 0.0                # menu.backdrop

[[bar.group]]                 # modules.left = ["Workspaces"]
name = "left"
anchor = "start"
modules = ["workspaces"]

[[bar.group]]                 # modules.center = ["WindowTitle"]
name = "center"
anchor = "center"
modules = ["window"]

[[bar.group]]                 # modules.right = [["Tempo", "Privacy", "Settings"]]
name = "right"
anchor = "end"
modules = ["clock", "privacy", "status"]

[module.clock]                # tempo: the time only, with the date in its format
items = ["clock"]

[module.privacy]
items = ["privacy"]
open = "none"

[module.status]               # today's settings.indicators, in the same order
items = [
  { item = "idle_inhibitor", show = "notable" },
  { item = "power_profile", show = "notable" },
  { item = "volume", show = "always" },
  { item = "microphone", show = "always" },
  { item = "bluetooth", show = "always" },
  { item = "network", show = "always" },
  { item = "vpn", show = "notable" },
  { item = "battery", show = "always", style = "icon-percentage" },
]

[transients]
osd       = { to = "off" }    # osd.enabled = false
workspace = { to = "off" }
alert     = { to = "off" }
message   = { to = "off" }
toast     = { to = "screen", position = "top-right", stack = 5 }   # toast_position, toast_limit

[clock]
format = "%a %d %b %R"        # tempo.clock_format
```

Notes:

- With `hover = true` this same bar shows the modules' default details, the
  prototype's. That is the two experiences merged with one line.
- The `notable` visibility of `idle_inhibitor`, `power_profile` and `vpn` follows
  the indicators that today appear in ashell only while active. They need
  checking one by one against `modules/settings`.
- The right-hand group matches `grouping = "combined"`: one island around the
  three modules. `shape = "per-module"` would give three separate islands.

## Test configuration: the island prototype

This file reproduces the prototype at commit `1aa3b34`: one island in the centre with
the zones of `compact/mod.rs` in the same order, the details of
`compact/details.rs`, panels growing out of the island and every transient
inside the island. Since the module defaults follow the prototype, the module part is
almost empty.

```toml
# The island prototype

[appearance]
material = "solid"            # the prototype's appearance.opacity = 1.0
motion = "full"

[[bar]]
position = "top"
outputs = "all"               # island.outputs
elsewhere = "on-demand"       # on other outputs it comes only when called
layer = "top"
background = "transparent"
hover = true
panels = "morph"

[[bar.group]]
name = "main"
anchor = "center"
shape = "island"
modules = ["workspaces", "window", "tray", "media", "clock", "updates", "status"]

# Already the defaults, written out for clarity:
# [module.status]  items: privacy, airplane, dnd, idle_inhibitor, bluetooth,
#                  microphone, volume, network (notable), battery (always, ring)
#                  details: system, updates, all status items visible
# [module.window]  details: launcher_button, window_list
# [module.clock]   items: weather, date, clock, notifications
#                  details: date, forecast, notices
# [item.updates]   notable_above = 40

[item.battery]
style = "ring"

[transients]
osd       = { to = "group:main" }
workspace = { to = "group:main" }
toast     = { to = "group:main" }
alert     = { to = "group:main" }
message   = { to = "group:main" }

[weather]
city = "Milano"               # the prototype's [weather], optional

[launcher]
terminal = "kitty"
```

Notes:

- In the prototype `tray` and `media` disappear when they have nothing to show. In the
  new model this is a general rule: **a module with no visible items takes no
  space**.
- The updates chip in the prototype's row is not a `Zone` (it has no details): it is the
  `updates` module, with no `details`.
- The launcher button can move into the row by writing
  `[module.workspaces] items = ["launcher_button", "workspaces"]`, without
  touching anything else.
- The prototype's `[island.seed]` stays out: it belongs to allock.

## Mapping from the current configuration

| ashell today | New configuration |
| --- | --- |
| `position`, `layer`, `outputs` | `[[bar]]` keys |
| `modules.left`, `center`, `right` | `[[bar.group]]` with `anchor` |
| a group `[["A", "B"]]` | one `[[bar.group]]` with several modules |
| `appearance.modules.*.grouping` | the group's `shape` (`island`, `per-module`, `none`) |
| `appearance.bar.surface` | `bar.background` |
| `appearance.menu.backdrop` | `bar.backdrop` |
| `settings.indicators` | `items` of `[module.status]` |
| `settings.*_indicator_format`, `battery_format` | the items' `style` |
| `settings.battery_hide_when_full` | `show = "notable"` on `battery` |
| `settings.remove_*_btn` | content of the `settings` panel: out of scope |
| `system_info.indicators` | `items` of `[module.system]` |
| `workspaces.indicator_format` | `style` of `workspaces` |
| `media_player.indicator_*` | options of the `media` item |
| `osd.enabled` | `transients.osd.to` |
| `notifications.toast`, `toast_position`, `toast_limit` | `transients.toast` |
| `appearance.*_color`, `font_name`, `opacity`, `blur` | styletto's `[appearance]` |
| `animations.enabled` | `appearance.motion` |
| `[[CustomModule]]` | `[custom.<name>]` + item `custom:<name>` |
| `settings`, `tempo`, `updates` commands | the Features level |

No automatic migration is needed, since the new configuration is deliberately
incompatible. An `ashell migrate` command that reads the old file and writes the
new one using this table could still be useful.

## Open questions

- [ ] **Names.** `group` and `module` are provisional. `module` clashes with the
      `modules/` folder in the code, which also covers the services.
- [ ] **`notable` rules.** The catalogue needs checking item by item against the
      current behaviour of ashell (`modules/settings`) and the prototype
      (`compact::status`).
- [ ] **Backdrop.** Extend the surface to the whole screen when a panel opens,
      or a separate surface just for the backdrop.
- [ ] **Reserved space with floating groups.** Whether `auto` is enough or a
      pixel value is always needed for different compositors.
- [ ] **Neighbours stepping aside.** How two groups with the same `anchor`
      behave when one of them grows.
- [ ] **Narrow items.** How wide items (`window_list`, `workspace_list`,
      `media_details`) shrink when they land in a row.
- [ ] **Several bars.** Whether transients sent to `group:<name>` must tell
      apart different bars with groups of the same name.
- [ ] **Validation.** Whether an error blocks loading or falls back to the
      preset with a warning, as the prototype does today for the whole file.
