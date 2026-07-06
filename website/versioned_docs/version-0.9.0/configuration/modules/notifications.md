---
sidebar_position: 5
---

# Notifications

This module displays a notification indicator in the status bar and provides a menu to view and interact with notifications.

The notification indicator shows a bell icon when there are no notifications, and a bell with a badge when notifications are present.

## Notification Daemon

Enabling this module makes ashell register itself as the system notification daemon by claiming the `org.freedesktop.Notifications` DBus name. This means:

- **dunst, mako, and other notification daemons cannot run alongside ashell** while this module is enabled. Only one process can hold the DBus name at a time. Starting another daemon after ashell will take over the name and ashell will stop receiving notifications.
- **Notifications are stored in memory only** and are lost when ashell exits.

### Toast popups

By default, ashell shows transient toast popups when notifications arrive. Toasts appear in a configurable corner of the screen, stack vertically up to `toast_limit`, and auto-dismiss after the timeout. Clicking a toast invokes the notification's default action (if one exists) and dismisses it.

The toast surface spans the full output height and uses a Wayland input region so that only the area occupied by actual toast cards accepts pointer input — the rest of the surface is fully click-through.

The `expire_timeout` hint sent by applications is respected: a value of `-1` falls back to `toast_timeout`, `0` means the toast never auto-dismisses, and any positive value (in milliseconds) is used directly.

If you prefer no popups and only the panel indicator, set `toast = false`.

## Notification Menu

Click the notification indicator to open the notifications menu. The menu displays all active notifications with the following features:

- **Individual notifications**: Shows the app icon, name, summary, and optional body text and timestamp
- **Clear button**: Removes all notifications at once
- **Grouped mode** (optional): Organizes notifications by application with expandable groups
- **Clicking a notification**: Invokes its default action (if provided by the app) and closes it

## Configuration

### format

The format string used to display notification timestamps. Uses chrono strftime format.

**Type:** `string`
**Default:** `"%H:%M"`

### show_timestamps

Whether to display timestamps for each notification in the menu.

**Type:** `boolean`
**Default:** `true`

### show_bodies

Whether to display the body text of notifications in the menu.

**Type:** `boolean`
**Default:** `true`

### grouped

Whether to group notifications by application in the menu.

When enabled, notifications are grouped by app name and each group can be
expanded or collapsed independently. The group header shows the app name,
icon, and count of notifications. When collapsed, only the most recent
notification from each group is previewed. When expanded, all notifications
in the group are shown.

Clicking on a notification in the menu will invoke its default action (if
one exists) and close it.

**Type:** `boolean`
**Default:** `false`

### toast

Whether to show transient toast popups when notifications arrive.

**Type:** `boolean`
**Default:** `true`

### toast_position

The corner of the screen where toast notifications appear.

**Type:** `string` — one of `"top_left"`, `"top_right"`, `"bottom_left"`, `"bottom_right"`
**Default:** `"top_right"`

### toast_timeout

How long (in milliseconds) a toast is shown before auto-dismissing when the application does not specify a timeout (`expire_timeout = -1`).

**Type:** `integer`
**Default:** `5000`

### toast_limit

Maximum number of toasts that can be visible at the same time. When this limit is reached, the oldest toast is removed to make room for a new one.

**Type:** `integer`
**Default:** `5`

### toast_max_height

Maximum height (in pixels) of each individual toast card. Cards with less content will be shorter; this value caps how tall a single card can grow.

**Type:** `integer`
**Default:** `150`

### blocklist

Notification app names to ignore.

Each entry is treated as a regular expression and matched against the notification `app_name`. If any pattern matches, the notification is dropped before it appears in the menu or as a toast.

**Type:** `array of strings`
**Default:** `[]`

### Example

```toml
[notifications]
format = "%m/%d %H:%M"
show_timestamps = true
show_bodies = false
grouped = true
toast = true
toast_position = "top_right"
toast_timeout = 4000
toast_limit = 5
toast_max_height = 150
blocklist = ["blueman", "^org\\.gnome\\."]
```
