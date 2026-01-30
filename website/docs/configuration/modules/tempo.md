---
sidebar_position: 10
---

# Tempo

Tempo combines a highly configurable clock with a compact weather summary in the status bar. Clicking the module opens a rich menu with a calendar, hourly forecast, and a seven-day outlook.

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
| `weather_location` | `enum`   | `Current`     | Determines which coordinates are queried when requesting weather data. `Current` geo-locates via IP using `ip-api.com`. Use the `City` variant to pin the module to a specific place.                 |

### City-based weather

```toml
[tempo]
clock_format = "%a %d %b %R"
weather_location = { City = "Berlin" }
```

### Modules placement

Add `"Tempo"` to any section in `[modules]` so it renders in the status bar:

```toml
[modules]
right = [ [ "Clock", "Tempo", "Privacy", "Settings" ] ]
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
