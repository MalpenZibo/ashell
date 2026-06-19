---
sidebar_position: 10
---

# Tray

This module provides a system tray for displaying icons of running applications.

Clicking on an icon will open the corresponding application or menu. The module only appears when applications have tray icons.

## Blocklist

You can filter which tray icons are displayed using the `blocklist` option. If a tray item's name matches any regex pattern in the blocklist, it won't be rendered.

**Note**: Matching is done against the tray item's name using regex patterns.

## Click Behavior

You can configure what happens when right-clicking a tray icon using `right_click`. The left click behavior is automatically set to the complement. If omitted, only left click is active and opens the context menu.

- `"Open"` — right click activates the application (e.g. show/raise its window); left click opens the context menu
- `"Menu"` — right click opens the context menu; left click activates the application

## Examples

**Hide multiple applications by pattern:**

```toml
[tray]
blocklist = ["spotify", "^org\\.gnome\\."]
```

**Right click to open the context menu (left click opens app):**

```toml
[tray]
right_click = "Menu"
```

## Default Configuration

The default configuration is:

```toml
[tray]
blocklist = []
```
