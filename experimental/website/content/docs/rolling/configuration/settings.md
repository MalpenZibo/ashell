+++
title = "Settings"
weight = 8
+++

Settings is the module of the machine's state: the `battery`,
`battery_time`, `peripheral_battery`, `power_profile`, `bluetooth`,
`idle_inhibitor`, `brightness`, `volume` and `microphone` items, and the
`settings` panel. The built-in `status` zone shows them, beside items still to
come, with how long the battery has left in its details:

```toml
[zone.status]
items = [
    "bluetooth",
    { item = "brightness", show = "always", style = "icon-value" },
    { item = "battery", warn_threshold = 40, alert_threshold = 15 },
]
```

{{ <picture name="settings/settings-items" alt="The settings items: the idle inhibitor, the power profile, Bluetooth with a device connected, the brightness, the headphones' battery, the laptop's with the time it has left, the volume and the muted microphone" width="348" height="95" /> }}

## `[settings]`

The commands of the panel's power buttons, and how far and by how much the
volume moves.

A power button set to `""`, or to spaces only, has no button.

| Key | Default | Button |
| --- | ------- | ------ |
| `lock` | none | Lock, also beside the battery |
| `suspend` | `"systemctl suspend"` | Suspend |
| `hibernate` | none | Hibernate |
| `reboot` | `"systemctl reboot"` | Reboot |
| `shutdown` | `"shutdown now"` | Shutdown |
| `logout` | `"loginctl kill-user $(whoami)"` | Logout |

| Key | Values | Default | What |
| --- | ------ | ------- | ---- |
| `max_volume` | a percentage, 1 to 200 | 100 | The output's highest volume, on its slider and its steps; above 100 it is amplified. The input stops at 100. |
| `volume_step` | a percentage, 1 to 50 | 5 | How much the volume's and the microphone's actions, and a notch of the wheel over their sliders, move them. |

```toml
[settings]
lock       = "hyprlock"
hibernate  = "systemctl hibernate"
max_volume = 150
```

## Items

| Item | Shows | Empty when | States | `notable` when | Opens |
| ---- | ----- | ---------- | ------ | -------------- | ----- |
| `battery` | the battery: how full, and charging or not | no battery | `charging`, `discharging`, `not_charging`, `full`, `warn`, `alert` | `warn`, `alert` or `charging` | `settings` |
| `battery_time` | how long until the battery is full or empty: "1h 12m" | no estimate, or on the charger without charging | `charging`, `discharging` | `discharging` | `settings` |
| `peripheral_battery` | the battery of each device that reports one: mice, keyboards, headphones, gamepads | no device reports one | each device: `charging`, `discharging`, `full`, `warn`, `alert` | a device is `warn`, `alert` or `charging`; only those show | `settings` |
| `power_profile` | the power profile | no profiles daemon | `balanced`, `performance`, `power_saver` | it is not balanced | `settings` |
| `bluetooth` | Bluetooth, on or off; its value is how many devices are connected | no adapter | `on`, `off`, `connected` | `on` | `settings` |
| `idle_inhibitor` | whether the session is kept from going idle | the compositor has no idle inhibitor | `on`, `off` | `on` | `settings` |
| `brightness` | the screen's brightness | no backlight | none | always | `settings` |
| `volume` | the output's volume, or that it is muted | no output | `muted`, `unmuted` | always | `settings` |
| `microphone` | the input's volume, or that it is muted | no input | `muted`, `unmuted` | always | `settings` |

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
| `battery`, `peripheral_battery`, `bluetooth`, `brightness`, `volume`, `microphone` | `style` | `"icon"`, `"value"`, `"icon-value"` | `"icon-value"` for `battery`, `"icon"` for the others, as ashell's |
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
| `volume` | `toggle_mute` | Mutes the output, or unmutes it. | none |
| `volume` | `volume_up` | `volume_step` louder, up to `max_volume` or where it already is above. | `scroll_up` |
| `volume` | `volume_down` | `volume_step` quieter. | `scroll_down` |
| `microphone` | `toggle_microphone_mute` | Mutes the input, or unmutes it. | none |
| `microphone` | `microphone_up` | `volume_step` louder, up to 100% or where it already is above. | `scroll_up` |
| `microphone` | `microphone_down` | `volume_step` quieter. | `scroll_down` |

```toml
[zone.status]
items = [
    { item = "idle_inhibitor", show = "always", right_click = "toggle_idle_inhibitor" },
    "battery",
]
```

## The panel

{{ <picture name="settings/settings-panel" alt="The settings panel: the battery with the time left, the config, lock and power buttons, the volume, microphone and brightness sliders, the Bluetooth, power profile and idle inhibitor tiles, and the headphones' battery" width="447" height="509" /> }}

The `settings` panel has the battery, how full and how long until full or
empty, with the button to the config window, the lock button and the power
button, which opens the power page while it has more than the lock. Then the
volume, the microphone and the brightness, each a slider; a click on the
volume's or the microphone's icon mutes it, and a muted one rests at zero,
as ashell's, until moved. Then the toggles, each while its service is there, the next
taking the place of one missing: Bluetooth, with the devices connected; the
power profile, a click on its icon going to the next one and anywhere else on
the tile opening the page of the three; and the idle inhibitor. Last, the
battery of each device that reports one.

Each of the panel's pages opens with the same head: back, the page's name
with what stands now under it, and the feature's icon. Where the page can
turn the feature on and off, the icon is the switch.

- **Power**, from the power button: the lock and the other commands set in
  `[settings]`, a row each, those that end the session after a line and in
  red. The panel closes once one has run. Its head says how full the battery
  is.
- **Output** and **Input**, from the chevron beside the volume or the
  microphone when there is more than one to choose: each port of a device a
  row of its own, named under it, the one in use checked; a click on another
  makes it the one in use. The icon at the head mutes it.
- **Power mode**, from the power profile tile: the three profiles, each said
  as GNOME says it, the one in use checked.
- **Bluetooth**, from the Bluetooth tile: the devices known, a click
  connecting or disconnecting one, and those found nearby, a click pairing
  one. The icon at the head turns Bluetooth on or off; opening the page with
  it on looks for devices for a while.

A page's list scrolls once it is long.

The volume and the microphone are the default output and input's, through
PulseAudio or PipeWire.
