---
sidebar_position: 5
---

# Window Title

Displays the title of the currently focused window in your status bar.

## What It Shows

Using the `mode` field, you can choose what information to display:

- `Title`: The window's title text (default)
- `Class`: The application name or class
- `InitialTitle`: The window's initial title text (ex - *kitty* instead of *hyprctl clients*)
- `InitialClass`: The initial application name or class. This is unlikely to differ from the current class but Hyprland exposes it

Note that *InitialTitle* and *InitialClass* are Hyprland-only and should not be used when running Niri.

## Title Length Control

The `truncate_title_after_length` field limits how long the displayed title can be:

- **Set to a number** (e.g., 75): Cuts off long titles at that length
- **Set to 0**: Shows the full title without any limit
- **Default**: 150 characters

When titles are too long, they're shortened to show the beginning and end with "..." in between, so you can still see both the app name and part of the title.

## Examples

**Show window titles, but cut them off at 75 characters:**

```toml
[window_title]
mode = "Title"
truncate_title_after_length = 75
```

**Show application names instead of titles:**

```toml
[window_title]
mode = "Class"
truncate_title_after_length = 50
```

**Show full titles without any length limit:**

```toml
[window_title]
mode = "Title"
truncate_title_after_length = 0
```
