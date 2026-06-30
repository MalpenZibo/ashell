---
sidebar_position: 2
---

# 🖥️ Compositor Support

Ashell runs on any Wayland compositor. It ships **dedicated integrations** for
the compositors it knows natively and falls back to a **generic Wayland backend**
everywhere else. The right backend is selected automatically at startup, with no
configuration required.

## Dedicated integrations

Dedicated backends talk to the compositor's own IPC, so they expose the richest
feature set, including compositor-specific concepts.

- **Hyprland**
- **Niri**

## Generic Wayland fallback

When no dedicated backend is detected, ashell uses a generic backend built on
standard Wayland protocols. It works on any compositor that implements them, and
each protocol is optional, and an unadvertised one simply disables the feature it
backs.

| Protocol | Provides |
| --- | --- |
| `wl_output` | Monitors |
| `ext-workspace-v1` | Workspaces (listing and switching) |
| `wlr-foreign-toplevel-management` | Active window (title and class) |

## Feature matrix

| Feature | Hyprland | Niri | Generic Wayland |
| --- | :---: | :---: | :---: |
| Active window * | ✅ | ✅ | ✅ |
| Workspaces | ✅ | ✅ | ✅ |
| Keyboard layout | ✅ | ✅ | ❌ |
| Keyboard submap | ✅ | ❌ | ❌ |

A ❌ means the backend (or the underlying protocol) does not expose that
feature; the corresponding module is simply unavailable on that compositor.

\* The `InitialTitle` and `InitialClass` window-title modes rely on
Hyprland-specific data and are unavailable on the other backends, where the
title falls back to an empty value.
