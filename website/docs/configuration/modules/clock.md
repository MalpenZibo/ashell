---
sidebar_position: 9
---

# Clock

This module displays the current time and date in the status bar.

Using the `format` configuration, you can customize how the time and date are displayed.

For more information about the available format options, see the [chrono documentation](https://docs.rs/chrono/latest/chrono/format/strftime/index.html).

By default, the clock displays the time using this format: `%a %d %b %R`,  
which prints the date as `Tue 08 Jul 11:04`.

## Format Cycling

The clock module supports multiple datetime formats that can be cycled through by clicking on the clock. When the `formats` array is provided, clicking the clock will cycle through each format in sequence.

- If `formats` is empty or not provided, the clock uses the single `format` string
- If `formats` contains entries, clicking cycles through them and the single `format` is ignored
- The update interval automatically adjusts (1 second for formats with seconds, 5 seconds otherwise)

## Examples

### Basic Single Format

This configuration shows the date in the format: `07/22/25 11:11:43 AM`

```toml
[clock]
format = "%D %r"
```

### Multiple Formats with Click Cycling

This configuration provides multiple formats that cycle when you click the clock:

```toml
[clock]
# Fallback format (used if formats array is empty)
format = "%a %d %b %R"

# Multiple formats that cycle on click
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
