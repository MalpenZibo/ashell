---
sidebar_position: 12
---

# Window Title

Displays the title of the currently focused window in your status bar.

## What It Shows

Using the `mode` field, you can choose what information to display:

- `Title`: The window's title text (default)
- `Class`: The application name or class
- `InitialTitle`: The window's initial title text (ex - *kitty* instead of *hyprctl clients*)
- `InitialClass`: The initial application name or class. This is unlikely to differ from the current class but Hyprland exposes it

Note that *InitialTitle* and *InitialClass* are Hyprland-only and should not be used when running Niri or MangoWC.

## Title Length Control

The `truncate_title_after_length` field limits how long the displayed title can be:

- **Set to a number** (e.g., 75): Cuts off long titles at that length (max 2048)
- **Set to 0**: Shows the full title up to the 2048 character limit
- **Default**: 150 characters

**Important**: Window titles are **hard-limited to 2048 characters** regardless of configuration. This prevents Wayland socket buffer overflow errors that can cause crashes when applications (like games) send very long titles.

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

**Show full titles without any length limit (capped at 2048):**

```toml
[window_title]
mode = "Title"
truncate_title_after_length = 0
```
