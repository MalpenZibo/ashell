---
sidebar_position: 10
---

# Tray

This module provides a system tray for displaying icons of running applications.

Clicking on an icon will open the corresponding application or menu. The module only appears when applications have tray icons.

The tray fully implements the KDE **Status Notifier Item (SNI)** protocol
(`org.kde.StatusNotifierItem` + `org.kde.StatusNotifierWatcher`), so it
works with Qt (`QSystemTrayIcon`), GTK (`GtkStatusIcon`), Chromium/CEF,
Gecko (`ksni`) and other SNI clients.

## Status and attention icons

SNI items expose a `Status` property: `Passive`, `Active` or
`NeedsAttention`.

- **Passive** items are **hidden** from the tray until they become `Active`.
- **Active** items are shown normally.
- **NeedsAttention** items are shown using the client's **attention icon**
  (and overlay icon when available) so the user notices them.

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
