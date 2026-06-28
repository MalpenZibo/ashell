---
sidebar_position: 5
---

# Troubleshooting

Common issues and quick fixes for ashell.

## Visual Issues, Freezing, or Rendering Problems

**Problem:** ashell may freeze, show visual artifacts, or fail to render properly on some systems.

**Cause:** Graphics driver compatibility issues. By default, ashell uses `wgpu` which uses `vulkan` as the default backend.

### Rendering Backends

ashell supports two rendering paths:

1. **wgpu** (default) - Uses Vulkan by default. You can force a specific backend using the `WGPU_BACKEND` environment variable:
   ```bash
   WGPU_BACKEND=vulkan ashell  # Force Vulkan
   WGPU_BACKEND=gl ashell     # Force OpenGL/EGL
   ```

2. **tiny-skia** (CPU renderer) - Falls back automatically if wgpu fails, or can be forced with:
   ```bash
   ICED_BACKEND=tiny-skia ashell
   ```

### Valid `WGPU_BACKEND` values

You can use a single value or a comma-separated list (e.g. `WGPU_BACKEND=gl,vulkan`). Values are case-insensitive.

| Value | Aliases | Description |
|---|---|---|
| `vulkan` | `vk` | Vulkan API |
| `gl` | `gles`, `opengl` | OpenGL / OpenGL ES (EGL)

### Backend Behavior

- ashell tries `wgpu` first, then falls back to `tiny-skia` if wgpu fails
- `tiny-skia` is a CPU renderer with less RAM consumption but potentially higher battery usage

**If the issue persists:** Try forcing a different backend or using `ICED_BACKEND=tiny-skia`.

### Hybrid / Multi-GPU: Black Bar or Missing Bar on External Monitor

**Problem:** ashell renders a black bar, or the bar doesn't appear on all monitors. Common on laptops with hybrid graphics (e.g. NVIDIA dGPU + iGPU) or when an external monitor is connected to a different GPU.

**Cause:** By default, `iced_layershell` sets `WGPU_POWER_PREF=low`, which tells wgpu to prefer the integrated GPU. On hybrid setups, the iGPU may not be connected to all outputs, for example, an external monitor wired to the NVIDIA dGPU won't receive frames rendered on the iGPU. The result is a black bar or a bar that only appears on one screen.

**Solution:** Force the discrete GPU with:
```bash
WGPU_POWER_PREF=high ashell
```

To make it permanent, add to your compositor config. For Hyprland, in `hyprland.conf`:
```
env = WGPU_POWER_PREF,high
```

**Note:** `WGPU_POWER_PREF` is a regular process-level environment variable. Using Hyprland's `env =` propagates it to all wgpu-based applications (browsers, games, etc.), not just ashell. To limit it to ashell only, use `exec-once` instead:
```
exec-once = env WGPU_POWER_PREF=high ashell
```

## Idle Inhibitor Issues

**Problem:** Screen sleeps even when ashell is running.

**Cause:** This is a swayidle bug in version 1.9.0+ with `BlockInhibited` property parsing.

**Solutions:**

- Downgrade swayidle to 1.8.x
- Wait for swayidle fix upstream
- Use alternative idle management tools like `hypridle`

## Missing Tray Icons

**Problem:** Telegram doesn't appear in tray when ashell starts after Telegram.

**Cause:** Telegram doesn't re-register with tray services if ashell starts afterward.

**Solutions:**

- Start Telegram after ashell
- Restart Telegram after starting ashell

## Debug Mode

Run with debug logging to find issues:

```bash
RUST_LOG=debug ashell
```

## Get Help

Include this info when reporting issues:

- OS and compositor
- GPU/driver info
- Full debug logs
- Your ashell config

## Font Changes Don't Take Effect

**Problem:** Setting `font_name` in the config has no visible effect.

**Cause:** Most often, the font name doesn't exactly match the font's family name,
or the font has a non-standard weight that causes a silent fallback.

**Fix:**

1. Verify the exact family name:
   ```bash
   fc-list | grep -i <search-term>
   ```
2. ashell prints warnings at startup when the font name is not found or when
   the weight doesn't match — check the logs.
3. Restart ashell after editing the config (font changes are not hot-reloaded).
