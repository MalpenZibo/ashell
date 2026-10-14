---
sidebar_position: 10
---

# Tray

This module provides a system tray for displaying icons of running applications.

Clicking on an icon will open the corresponding application or menu. The module only appears when applications have tray icons.

The tray uses the StatusNotifierItem (SNI) protocol: ashell acts as the
StatusNotifierWatcher and shows the items applications register there. This
covers Qt applications, Electron/Chromium applications and GTK applications
using libappindicator or libayatana-appindicator. Applications that only
support the legacy X11 (XEmbed) tray, such as those using `GtkStatusIcon`,
are not shown.

Middle click and scrolling on an icon are not supported.

## Hidden and highlighted icons

Applications can tell the tray how relevant their icon is:

- **Passive** icons (nothing to report right now) are hidden, and appear again
  as soon as the application changes state.
- **Active** icons are shown normally. Applications that don't report a state
  are treated as active.
- **NeedsAttention** icons are shown with the application's attention icon,
  or with the normal icon if it doesn't provide one.

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
