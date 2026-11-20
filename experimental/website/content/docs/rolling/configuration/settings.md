+++
title = "Settings"
weight = 8
+++

Settings is the module of the machine's state: the `battery`,
`peripheral_battery`, `power_profile`, `bluetooth`, `idle_inhibitor` and
`brightness` items. The built-in `status` zone shows them, beside items still
to come:

```toml
[zone.status]
items = [
    "bluetooth",
    { item = "brightness", show = "always" },
    { item = "battery", notable_below = 30 },
]
```

{{ <picture name="settings/settings-items" alt="The settings items: the idle inhibitor, the power profile, Bluetooth with a device connected, the brightness, the headphones' battery and the laptop's" width="306" height="95" /> }}

## Items

| Item | Shows | Empty when | `notable` when | Opens |
| ---- | ----- | ---------- | -------------- | ----- |
| `battery` | the battery: how full, and on the charger or not | no battery | below `notable_below`, or charging | `settings` |
| `peripheral_battery` | the battery of each device that reports one: mice, keyboards, headphones, gamepads | no device reports one | one is below `notable_below`; only those show | `settings` |
| `power_profile` | the power profile | no profiles daemon | it is not balanced | `settings` |
| `bluetooth` | Bluetooth, on or off, and how many devices are connected | no adapter | a device is connected | `settings` |
| `idle_inhibitor` | whether the session is kept from going idle | the compositor has no idle inhibitor | it is on | `settings` |
| `brightness` | the screen's brightness | no backlight | always | `settings` |

An empty item never shows, whatever its `show`. The battery's colour is the
charger's when plugged in, a warning at 30% or less, or while saving power,
and a danger at 15% or less.

The idle inhibitor is ashell's own: another program keeping the session awake
does not show. It reaches the compositor the first time it is turned on.

### Options

| Item | Option | Values | Default |
| ---- | ------ | ------ | ------- |
| `battery` | `style` | `"ring"`, `"percentage"`, `"ring-percentage"` | `"ring-percentage"` |
| `battery` | `notable_below` | a percentage, 1 to 100 | 20 |
| `peripheral_battery` | `notable_below` | a percentage, 1 to 100 | 20 |

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
