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
