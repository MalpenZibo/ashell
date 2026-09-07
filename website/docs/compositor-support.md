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
- **MangoWC**

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

| Feature | Hyprland | Niri | MangoWC | Generic Wayland |
| --- | :---: | :---: | :---: | :---: |
| Active window * | ✅ | ✅ | ✅ | ✅ |
| Workspaces: list and click to focus | ✅ | ✅ | ✅ | ✅ |
| Workspaces: scroll to switch | ✅ | ✅ | ✅ | ❌ |
| Workspaces: empty/occupied distinction ** | ✅ | ✅ | ✅ | ❌ |
| Workspaces: urgent highlight | ❌ | ✅ | ✅ | ✅ |
| Workspaces: window icons *** | ✅ | ✅ | ❌ | ❌ |
| Special workspaces | ✅ | ❌ | ❌ | ❌ |
| Virtual desktops **** | ✅ | ❌ | ❌ | ❌ |
| Keyboard layout | ✅ | ✅ | ✅ | ❌ |
| Keyboard submap | ✅ | ❌ | ✅ | ❌ |

A ❌ means the backend (or the underlying protocol) does not expose that
feature; the corresponding module, or that part of it, is simply unavailable on
that compositor.

\* The `InitialTitle` and `InitialClass` window-title modes rely on
Hyprland-specific data and are unavailable on the other backends, where the
title falls back to an empty value.

\*\* `ext-workspace-v1` carries no per-workspace window count, so the generic
backend draws every workspace in the occupied style instead of the outlined
empty one.

\*\*\* `indicator_format = "NameAndIcons"` needs the per-workspace window
classes, which only the Hyprland and Niri backends collect. Elsewhere the
indicator shows the workspace name only.

\*\*\*\* `enable_virtual_desktops` targets the
[hyprland-virtual-desktops](https://github.com/levnikmyskin/hyprland-virtual-desktops)
Hyprland plugin.

Urgent highlighting on the generic backend needs the compositor to report the
`urgent` state through `ext-workspace-v1`. Hyprland's workspace and client
data carry no urgency field (its IPC only emits a per-window `urgent` event,
which ashell does not track), so ashell always reports "not urgent" there.
