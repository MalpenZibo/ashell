+++
title = "Tempo"
weight = 7
+++

Tempo is the module of the time, the weather and the calendar: the `clock`,
`date`, `weather` and `forecast` items, and the `calendar` panel. The
built-in `clock` zone shows them:

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

| Item | Shows | Empty when | States | `notable` when | Opens |
| ---- | ----- | ---------- | ------ | -------------- | ----- |
| `clock` | the time | never | none | always | `calendar` |
| `date` | the date | never | none | always | `calendar` |
| `weather` | the sky now, and how warm | no weather yet, or none configured | `wet`: it rains, snows or storms | `wet` | `calendar` |
| `forecast` | the next hours: when, the sky, how warm | no weather yet, or none configured | `wet`: rain, snow or a storm is coming in them | `wet` | `calendar` |

An empty item never shows, whatever its `show`.

### Options

| Item | Option | Values | Default |
| ---- | ------ | ------ | ------- |
| `clock` | `format` | a [date format](https://docs.rs/chrono/latest/chrono/format/strftime/index.html) | `"%R"` |
| `clock` | `formats` | a list of date formats, gone through with a right click | the `format` alone |
| `date` | `format` | a date format | `"%a %-d"` |
| `weather` | `style` | `"icon"`, `"value"`, `"icon-value"`: the sky, how warm, or both | `"icon-value"` |
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

The time zone is the module's: changed on one item, every item and the
calendar tell the time in it. The format is the item's own.

## The calendar

{{ <picture name="tempo/tempo-calendar" alt="The calendar panel: the time, the time zones and the weather beside the month" width="960" height="540" /> }}

The `calendar` panel has the time, large, with its seconds; the time zones,
each with its time, the one chosen marked, and a click on one tells the time
in it; and the weather now, with the next hours.

Beside them is the month, today marked, the days with a forecast showing
their sky. A click on one of those shows its weather and hours instead of
now's; the arrows go through the months, and **Today** comes back. A click on
the place hides it, for a screenshot, and shows it again.
