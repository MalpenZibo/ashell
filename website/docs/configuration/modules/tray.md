---
sidebar_position: 10
---

# Tray

This module provides a system tray for displaying icons of running applications.

Clicking on an icon will open the corresponding application or menu. The module only appears when applications have tray icons.

## Blocklist

You can filter which tray icons are displayed using the `blocklist` option. If a tray item's name matches any regex pattern in the blocklist, it won't be rendered.

**Note**: Matching is done against the tray item's name using regex patterns.

## Examples

**Hide multiple applications by pattern:**

```toml
[tray]
blocklist = ["spotify", "^org\\.gnome\\."]
```

## Default Configuration

The default configuration is:

```toml
[tray]
blocklist = []
```
