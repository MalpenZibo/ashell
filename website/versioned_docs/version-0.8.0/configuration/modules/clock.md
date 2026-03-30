---
sidebar_position: 1
---

# Clock

:::warning

This module will be deprecated in future releases. Use the `Tempo` module instead.

:::

This module displays the current time and date in the status bar.

Using the `format` configuration, you can customize how the time and date are displayed.

For more information about the available format options, see the [chrono documentation](https://docs.rs/chrono/latest/chrono/format/strftime/index.html).

By default, the clock displays the time using this format: `%a %d %b %R`,  
which prints the date as `Tue 08 Jul 11:04`.

## Example

This configuration shows the date in the format: `07/22/25 11:11:43 AM`

```toml
[clock]
format = "%D %r"
```
