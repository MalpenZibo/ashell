+++
title = "Settings"
weight = 8
+++

Settings is the module of the machine's state: the `battery`,
`battery_time`, `peripheral_battery`, `power_profile`, `bluetooth`,
`idle_inhibitor` and `brightness` items. The built-in `status` zone shows
them, beside items still to come, with how long the battery has left in its
details:

```toml
[zone.status]
items = [
    "bluetooth",
    { item = "brightness", show = "always", style = "icon-value" },
    { item = "battery", warn_threshold = 40, alert_threshold = 15 },
]
```

{{ <picture name="settings/settings-items" alt="The settings items: the idle inhibitor, the power profile, Bluetooth with a device connected, the brightness, the headphones' battery, and the laptop's with the time it has left" width="278" height="95" /> }}

## Items

| Item | Shows | Empty when | States | `notable` when | Opens |
| ---- | ----- | ---------- | ------ | -------------- | ----- |
| `battery` | the battery: how full, and charging or not | no battery | `charging`, `discharging`, `not_charging`, `full`, `warn`, `alert` | `warn`, `alert` or `charging` | `settings` |
| `battery_time` | how long until the battery is full or empty: "1h 12m" | no estimate | `charging`, `discharging` | `discharging` | `settings` |
| `peripheral_battery` | the battery of each device that reports one: mice, keyboards, headphones, gamepads | no device reports one | each device: `charging`, `discharging`, `full`, `warn`, `alert` | a device is `warn`, `alert` or `charging`; only those show | `settings` |
| `power_profile` | the power profile | no profiles daemon | `balanced`, `performance`, `power_saver` | it is not balanced | `settings` |
| `bluetooth` | Bluetooth, on or off; its value is how many devices are connected | no adapter | `on`, `off`, `connected` | `connected` | `settings` |
| `idle_inhibitor` | whether the session is kept from going idle | the compositor has no idle inhibitor | `on`, `off` | `on` | `settings` |
| `brightness` | the screen's brightness | no backlight | none | always | `settings` |

An empty item never shows, whatever its `show`. A battery discharging is
`warn` below its `warn_threshold` and `alert` below its `alert_threshold`; on
the charger it is neither. `not_charging` is on the charger with the charge
held, as some laptops do near full. With
`show` set to a list of states, the item shows in any of them; for
`peripheral_battery`, each device shows in them on its own:

```toml
[zone.status]
items = [
    { item = "battery", show = ["discharging", "charging", "not_charging"] },  # not once full
    { item = "battery_time", show = ["discharging"] },                          # only on the battery
]
```

A battery is coloured a success while charging, or while a charge limit
holds it, and a warning or a danger while `warn` or `alert`, as ashell
colours its loads; its percentage takes its icon's colour.

The idle inhibitor is ashell's own: another program keeping the session awake
does not show. It reaches the compositor the first time it is turned on.

### Options

| Item | Option | Values | Default |
| ---- | ------ | ------ | ------- |
| `battery`, `peripheral_battery`, `bluetooth`, `brightness` | `style` | `"icon"`, `"value"`, `"icon-value"` | `"icon-value"` for `battery`, `"icon"` for the others, as ashell's |
| `battery` | `glyph` | `"icon"`, its icon showing how full; `"ring"`, a ring filled as full, the charging icon while charging | `"icon"` |
| `battery`, `peripheral_battery` | `warn_threshold` | a percentage, 1 to 100, above `alert_threshold` | 30 |
| `battery`, `peripheral_battery` | `alert_threshold` | a percentage, 1 to 100 | 20 |
| `peripheral_battery` | `kinds` | a list of `"keyboard"`, `"mouse"`, `"headphones"`, `"gamepad"` | all of them |

`style` is how an item draws its value: its icon, the value, or both. The
same option is on every item that has a value, the weather's too. How long the battery has left
is an item of its own, `battery_time`: beside `battery`, or in a zone's
details.

```toml
[zone.status]
items = [
    { item = "battery", style = "icon" },
    "battery_time",                                         # 1h 12m beside the icon
    { item = "peripheral_battery", kinds = ["mouse"] },
]
```

### Actions

| Item | Action | Does | Default gesture |
| ---- | ------ | ---- | --------------- |
| `power_profile` | `next_power_profile` | Balanced, then performance, then power saver. | none |
| `bluetooth` | `toggle_bluetooth` | Turns Bluetooth on or off. | none |
| `idle_inhibitor` | `toggle_idle_inhibitor` | Keeps the session from going idle, or lets it. | none |
| `brightness` | `brightness_up` | A twentieth brighter. | `scroll_up` |
| `brightness` | `brightness_down` | A twentieth darker, down to the lowest level that still lights. | `scroll_down` |

```toml
[zone.status]
items = [
    { item = "idle_inhibitor", show = "always", right_click = "toggle_idle_inhibitor" },
    "battery",
]
```

## The panel

The `settings` panel is still a placeholder.
