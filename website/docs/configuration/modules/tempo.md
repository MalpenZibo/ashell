---
sidebar_position: 10
---

# Tempo

Tempo combines a highly configurable clock with an optional weather summary in the status bar—you can run it as a clock-only module or pair it with live conditions. Clicking the module opens a rich menu with a calendar, hourly forecast, and a seven-day outlook.

## What Tempo shows

- **Status bar** – current time (using your preferred `clock_format`) and, when weather data is available, an icon + temperature badge that match the current conditions.
- **Menu** – a resizable panel containing:
  - A calendar with month navigation and highlighted selections.
  - Current city, timestamp, weather description, feels-like temperature, humidity, and wind information.
  - A horizontally scrollable hourly forecast.
  - A vertically stacked seven-day forecast with dominant wind direction and speeds.

## Configuration

| Option             | Type     | Default       | Description                                                                                                                                                                                           |
| ------------------ | -------- | ------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `clock_format`     | `string` | `%a %d %b %R` | Strftime-compatible format used for the clock in the bar and in the menu header. See the [chrono formatting guide](https://docs.rs/chrono/latest/chrono/format/strftime/index.html) for placeholders. |
| `formats`          | `array`  | `[]`          | Multiple datetime formats that can be cycled through by right-clicking the clock. When provided, clicking cycles through each format in sequence.                                                     |
| `timezones`        | `array`  | `[]`          | Timezone identifiers that can be cycled through by scrolling. Supports both IANA names (e.g., `"UTC"`, `"America/New_York"`) and fixed offsets (e.g., `"+00:00"`, `"-05:00"`).                        |
| `weather_location` | `enum`   | `None`        | Determines which coordinates are queried when requesting weather data. `Current` geo-locates via IP using `ip-api.com`. Use the `City` variant to pin the module to a specific place.                 |

### City-based weather

```toml
[tempo]
clock_format = "%a %d %b %R"
weather_location = { City = "Rome" }
```

### Clock-only mode

If you omit `weather_location`, Tempo renders just the clock and calendar UI—no network calls or weather widgets.

```toml
[tempo]
clock_format = "%a %d %b %R"
# weather_location left unspecified on purpose
```

### Format Cycling

The Tempo module supports multiple datetime formats that can be cycled through by right-clicking on the clock. When the `formats` array is provided, right-clicking the clock will cycle through each format in sequence.

- If `formats` is empty or not provided, the clock uses the single `clock_format` string
- If `formats` contains entries, right-clicking cycles through them and the single `clock_format` is ignored
- The update interval automatically adjusts (1 second for formats with seconds, 5 seconds otherwise)

#### Example: Multiple Formats with Right-Click Cycling

```toml
[tempo]
# Fallback format (used if formats array is empty)
clock_format = "%a %d %b %R"

# Multiple formats that cycle on right-click
formats = [
    "%a %d %b %R",         # Mon 23 Dec 14:30
    "%Y-%m-%d %H:%M:%S",   # 2024-12-23 14:30:45
    "%d/%m/%Y %I:%M %p",   # 23/12/2024 02:30 PM
    "%A, %B %d, %Y",       # Monday, December 23, 2024
    "%H:%M:%S",            # 14:30:45 (updates every second)
    "%x",                  # Locale date (e.g., 12/23/24)
    "%X",                  # Locale time (e.g., 14:30:45)
]
```

### Timezone Support

Tempo supports cycling through multiple timezones by scrolling on the clock widget. The `timezones` array accepts both IANA timezone names and fixed offset strings.

- IANA names (e.g., `"UTC"`, `"America/New_York"`) are used when the format contains `%Z` to show timezone abbreviations
- Fixed offsets (e.g., `"+00:00"`, `"-05:00"`) are used for numeric offset display with `%z`, `%:z`, etc.
- Scroll up/down to cycle forward/backward through the configured timezones

#### Example: Multiple Timezones

```toml
[tempo]
clock_format = "%a %d %b %R %Z"  # Include timezone abbreviation
timezones = [
    "UTC",
    "America/New_York",
    "Europe/London",
    "+09:00",  # Fixed offset for JST
    "-05:00",  # Fixed offset for EST
]
```

#### Example: Numeric Offsets Only

```toml
[tempo]
clock_format = "%a %d %b %R %:z"  # Show numeric offset with colon
timezones = [
    "+00:00",  # UTC
    "-05:00",  # EST
    "+09:00",  # JST
    "+01:00",  # CET
]
```

### Modules placement

Add `"Tempo"` to any section in `[modules]` so it renders in the status bar:

```toml
[modules]
right = [ [ "Tempo", "Privacy", "Settings" ] ]
```

### Defaults

```toml
[tempo]
clock_format = "%a %d %b %R"
weather_location = "Current"
```

## Networking & privacy

- Tempo fetches location data either from `ip-api.com` (for `Current`) or from Open-Meteo's geocoding endpoint (for `City`).
- Weather observations and forecasts are requested from the Open-Meteo API every 30 minutes. Ensure `ashell` has network access.
- If an API call fails the module keeps showing the last successful reading and logs a warning.

## Tips

1. Include seconds in `clock_format` (e.g., `%T`) only if you need them—the module automatically increases the refresh rate when second specifiers are present.
2. Use `City` when running behind VPNs or privacy relays, so the weather is tied to a predictable location.
3. Combine Tempo with the dynamic menu wrapper branch to get a spacious layout for the weather panel.
