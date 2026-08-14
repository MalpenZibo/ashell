---
sidebar_position: 3
---

# 🌐 Network Backend Support

Ashell supports multiple network management backends. The backend is selected
automatically at startup. **NetworkManager** is tried first, and if it is
unavailable ashell falls back to **IWD**. No configuration is required.

## NetworkManager

NetworkManager is the primary and most fully-featured backend. It provides
complete networking support including wired connections, Wi-Fi, VPN, and more.
This is the recommended backend for most users.

## IWD

IWD (iNet Wireless Daemon) is a lightweight Wi-Fi-focused backend. It is used
as a fallback when NetworkManager is not available. Since IWD only manages
wireless connections, several features that depend on NetworkManager are
unavailable.

## Feature matrix

| Feature | NetworkManager | IWD |
| --- | :---: | :---: |
| Wi-Fi scan & connect | ✅ | ✅ |
| Wi-Fi enable/disable | ✅ | ✅ |
| Signal strength | ✅ | ✅ |
| Ethernet detection | ✅ | ❌ |
| VPN management | ✅ | ❌ |
| Airplane mode | ✅ | ✅ |
| Connectivity state | ✅ | ✅ |

A ❌ means the backend does not support that feature; the corresponding UI
element is hidden or unavailable when running under that backend.

### Notes

- **Ethernet detection** - IWD is a wireless-only daemon and has no knowledge of
  wired connections.
- **VPN management** - VPN support (including WireGuard) requires NetworkManager.
  The VPN toggle and sub-menu are hidden when running under IWD.

