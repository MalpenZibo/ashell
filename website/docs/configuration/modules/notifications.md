---
sidebar_position: 5
---

# Notifications

This module displays a notification indicator in the status bar and provides a menu to view and interact with notifications.

The notification indicator shows a bell icon with the current number of unread notifications.

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

## Example

```toml
[notifications]
format = "%m/%d %H:%M"
show_timestamps = true
max_notifications = 20
show_bodies = false
grouped = true
```
