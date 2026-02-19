---
sidebar_position: 5
---

# Notifications

This module displays a notification indicator in the status bar and provides a menu to view and interact with notifications.

The notification indicator shows a bell icon with the current number of unread notifications.

## Notification Daemon

Enabling this module makes ashell register itself as the system notification daemon by claiming the `org.freedesktop.Notifications` DBus name. This means:

- **dunst, mako, and other notification daemons cannot run alongside ashell** while this module is enabled. Only one process can hold the DBus name at a time. Starting another daemon after ashell will take over the name and ashell will stop receiving notifications.
- **Notifications are stored in memory only** and are lost when ashell exits.

### Toast popups

By default, ashell shows transient toast popups when notifications arrive. Toasts appear in a configurable corner of the screen, stack vertically up to `toast_max_visible`, and auto-dismiss after the timeout. Clicking a toast invokes the notification's default action and dismisses it.

The `expire_timeout` hint sent by applications is respected: a value of `-1` falls back to `toast_default_timeout`, `0` means the toast never auto-dismisses, and any positive value (in milliseconds) is used directly.

If you prefer no popups and only the panel, set `toast = false`.

## Configuration

### format

The format string used to display notification timestamps. Uses chrono strftime format.

**Type:** `string`
**Default:** `"%H:%M"`

### show_timestamps

Whether to display timestamps for each notification in the menu.

**Type:** `boolean`
**Default:** `true`

### max_notifications

Maximum number of notifications to display in the menu. If not set, all notifications are shown.

**Type:** `integer` (optional)
**Default:** `null`

### show_bodies

Whether to display the body text of notifications in the menu.

**Type:** `boolean`
**Default:** `true`

### grouped

Whether to group notifications by application in the menu.

When enabled, notifications are grouped by app name and each group can be
expanded or collapsed independently. The group header shows the newest
notification time for that app, and the collapsed preview shows up to 3
notifications. Clicking the app icon clears all notifications for that group.

Note: in grouped mode, `max_notifications`, `show_timestamps`, and
`show_bodies` are not applied.

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

### toast_default_timeout

How long (in milliseconds) a toast is shown before auto-dismissing when the application does not specify a timeout (`expire_timeout = -1`).

**Type:** `integer`
**Default:** `5000`

### toast_max_visible

Maximum number of toasts that can be visible at the same time. When this limit is reached, the oldest toast is removed to make room for a new one.

**Type:** `integer`
**Default:** `5`

## Example

```toml
[notifications]
format = "%m/%d %H:%M"
show_timestamps = true
max_notifications = 20
show_bodies = false
grouped = true
toast = true
toast_position = "top_right"
toast_default_timeout = 4000
toast_max_visible = 3
```
