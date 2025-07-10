---
sidebar_position: 9
---

# Clock

This module displays the current time and date in the status bar.

Using the `format` configuration, you can customize how the time and date are displayed.

For more information about the available format options, see the [chrono documentation](https://docs.rs/chrono/latest/chrono/format/strftime/index.html).

By default, the clock will display the time in using this format `%a %d %b %R`
that prints the date in the format `Tue 08 Jul 11:04`.

## Example

This configuration show the date in this format: `07/22/25 11:11:43 AM`

```toml
[clock]
format = "%D %r"
```
