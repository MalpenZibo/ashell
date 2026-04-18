---
sidebar_position: 5
---

# Troubleshooting

Common issues and quick fixes for ashell.

## NVIDIA Driver Issues

**Problem:** ashell freezes on startup when using NVIDIA graphics on Linux.

**Cause:** The default Vulkan backend may have compatibility issues with NVIDIA drivers.

**Solution 1 (Permanent):** Set renderer backend in your config.toml:

```toml
renderer_backend = "opengl"
```

**Solution 2 (Temporary):** Force the use of OpenGL by setting the `WGPU_BACKEND` environment variable:

```bash
WGPU_BACKEND=gl ashell
```

This uses EGL as the context creation API, bypassing Vulkan entirely.

**If the issue persists:** Try different NVIDIA drivers or use a different compositor.

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
