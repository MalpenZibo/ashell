+++
title = "Tempo"
weight = 7
+++

Tempo is the module of the time, the weather and the clock: the `clock`,
`date`, `weather`, `forecast`, `timer`, `stopwatch` and `alarm` items, and
the `calendar` panel. The built-in `clock` zone shows the first four:

```toml
[bar.main]
hover  = true
center = [["window", "clock"]]   # the weather, the date and the time; the forecast on hover

[tempo]
timezones        = ["Europe/Rome", "America/New_York", "+09:00"]
weather_location = { city = "Milano" }
```

{{ <picture name="tempo/tempo-items" alt="A bar with the weather, the date and the time, the forecast on hover under the time, and the next hours" width="960" height="115" /> }}

## `[tempo]`

What the module does and where its data comes from. Where its items show,
and how, is written on the items.

| Key | Values | Default | What it sets |
| --- | ------ | ------- | ------------ |
| `timezones` | a list of time zones | `[]` | The time zones the time is told in, in turn. An IANA name (`"Europe/Rome"`, `"UTC"`) or a fixed offset (`"+09:00"`). The time is told in the first; with none, in the system's. |
| `weather_location` | `"current"`, `{ city = "<name>" }`, `{ coordinates = [<latitude>, <longitude>] }` | none | Where the weather is for: where the IP address says, a place by name, or a point. With none there is no weather. |
| `wind_speed_unit` | `"kmh"`, `"mph"`, `"ms"` | the locale's | How the wind is measured. |

The temperatures follow the locale: Fahrenheit where it measures in it,
Celsius elsewhere. The names of days and months, and whether the panel's
times have 12 or 24 hours, follow it as well.

{% <note kind="info" title="Where the weather comes from"> %}
The forecast comes from [Open-Meteo](https://open-meteo.com) every half
hour. `current` asks [ip-api.com](https://ip-api.com) where the IP address
is, a city is found with Open-Meteo's geocoding, and coordinates are named
with [Nominatim](https://nominatim.openstreetmap.org). A fetch that fails
keeps the last forecast and is tried again sooner.
{% </note> %}

## Items

| Item | Shows | Empty when | `notable` when | Opens |
| ---- | ----- | ---------- | -------------- | ----- |
| `clock` | the time | never | always | `calendar` |
| `date` | the date | never | always | `calendar` |
| `weather` | the sky now, and how warm | no weather yet, or none configured | it rains, snows or storms | `calendar` |
| `forecast` | the next hours: when, the sky, how warm | no weather yet, or none configured | rain, snow or a storm is coming in them | `calendar` |
| `timer` | the time left on the timer | no timer has been started | the timer runs or has finished | `calendar` |
| `stopwatch` | the elapsed time | no time has been counted | the stopwatch runs | `calendar` |
| `alarm` | the next alarm's time, or the one ringing | no alarm is set | an alarm is ringing | `calendar` |

The `timer`, `stopwatch` and `alarm` items are `notable`: left out of the
layout, they appear only when there is something to show, and can be added
wherever an item belongs.

An empty item never shows, whatever its `show`.

### Options

| Item | Option | Values | Default |
| ---- | ------ | ------ | ------- |
| `clock` | `format` | a [date format](https://docs.rs/chrono/latest/chrono/format/strftime/index.html) | `"%R"` |
| `clock` | `formats` | a list of date formats, gone through with a right click | the `format` alone |
| `date` | `format` | a date format | `"%a %-d"` |
| `weather` | `style` | `"icon"`, `"icon-temperature"` | `"icon-temperature"` |
| `forecast` | `hours` | 1 to 12 | 6 |

The stable ashell's one line of date and time is a `clock` with that format,
and no `date`:

```toml
[zone.clock]
items = ["weather", { item = "clock", format = "%a %d %b %R" }]
```

### Actions

| Item | Action | Does | Default gesture |
| ---- | ------ | ---- | --------------- |
| `clock` | `cycle_format` | The next of its `formats`. | `right_click` |
| `clock`, `date` | `next_timezone` | The next of `timezones`. | `scroll_up` |
| `clock`, `date` | `previous_timezone` | The one before. | `scroll_down` |
| `timer` | `toggle_timer` | Starts the timer, or stops it and keeps what is left. | `right_click` |
| `timer` | `reset_timer` | Puts the timer back to its length. | |
| `timer` | `add_minute` | Adds a minute to the time left. | |
| `stopwatch` | `toggle_stopwatch` | Starts the stopwatch, or stops it. | `right_click` |
| `stopwatch` | `reset_stopwatch` | Forgets the time and the laps. | |
| `stopwatch` | `lap_stopwatch` | Records a lap. | |
| `alarm` | `snooze_alarm` | Puts a ringing alarm off for five minutes. | `right_click` |
| `alarm` | `dismiss_alarm` | Stops a ringing alarm. | |

The time zone is the module's: changed on one item, every item and the clock
panel tell the time in it. The format is the item's own.

## The calendar panel

{{ <picture name="tempo/tempo-calendar" alt="The calendar panel on its calendar tab: the time, the time zones and the weather beside the month" width="960" height="540" /> }}

The `calendar` panel has four tabs, chosen at the top: **Calendar**,
**Alarms**, **Timer** and **Stopwatch**. It opens on the calendar.

### Calendar

The calendar has the time, large, with its seconds; the time zones, each with
its time, the one chosen marked, and a click on one tells the time in it; and
the weather now, with the next hours.

Beside them is the month, today marked, the days with a forecast showing
their sky. A click on one of those shows its weather and hours instead of
now's; the arrows go through the months, and **Today** comes back. A click on
the place hides it, for a screenshot, and shows it again.

### Alarms

An alarm is a time of day and the days it repeats on; with no days, it rings
once, at its next time, and turns itself off. A ringing alarm sends a
desktop notification and plays a sound, and shows a banner in the panel with
**Snooze** (five more minutes) and **Dismiss**.

Alarms are kept in `$XDG_STATE_HOME/ashell/alarms.toml` (or
`~/.local/state/ashell/alarms.toml`), and use the system's local time, not
the time zone the panel is showing. They ring while ashell runs.

### Timer

The timer counts down from a preset (1, 5, 10, 15, 25, 30, 45 or 60
minutes), or from a minute added to it. **Start** and **Pause** toggle it,
and **+1 min** gives it more time. A finished timer is announced the same way
an alarm is.

### Stopwatch

The stopwatch counts up to the hundredth. **Start** and **Pause** toggle it,
**Lap** records a split, and the laps are listed newest first with their
split and their total.
