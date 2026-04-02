# Known Limitations and Design Debt

This chapter documents known architectural limitations and design tradeoffs. Understanding these helps contributors make informed decisions and avoid re-discovering known issues.

## The iced Fork Dependency

**Issue**: ashell depends on a fork of a fork of iced (upstream → Pop!_OS/cosmic → MalpenZibo). This creates maintenance burden:

- Upstream iced improvements must be manually cherry-picked.
- The fork diverges over time, making rebases increasingly difficult.
- Contributors can't easily use upstream iced documentation or examples without checking for differences.

**Why it exists**: Upstream iced doesn't support Wayland layer shell surfaces. The Pop!_OS fork adds SCTK integration, and MalpenZibo's fork adds further fixes needed by ashell.

**Status**: The maintainer has explored alternatives (including an experimental GUI library called "guido"), but there are no concrete plans to migrate away from the iced fork. The fork is updated periodically to track upstream changes.

**Related**: [GitHub Issue #450 (Roadmap)](https://github.com/MalpenZibo/ashell/issues/450)

## Menu Surface Architecture

**Issue**: Context menus currently use a fullscreen transparent overlay surface. This has several drawbacks:

- **Memory waste**: ~140 MB VRAM per 4K monitor for a transparent surface
- **Layering bugs**: The overlay doesn't correctly layer popups on top of other windows in all cases
- **Conflicts**: Can interfere with other layer surfaces on the Background layer

**Correct approach**: Use `zwlr_layer_surface_v1::get_popup` to create proper `xdg_popup` surfaces. However, the SCTK library has this method but the iced fork doesn't expose it.

**Workaround**: The current fullscreen overlay approach was also chosen because iced has a HiDPI scaling regression where newly created surfaces initially render blurry.

**Related**: [GitHub Issue #491](https://github.com/MalpenZibo/ashell/issues/491)

## Memory Usage

**Issue**: ashell's process RSS (total memory) is 100–300 MB, despite the application heap being only ~3.5 MB.

**Breakdown** (from DHAT profiling):
- Font system (fontdb + cosmic_text): ~59% of peak heap
- Shader compilation (naga/wgpu): ~16%
- A bare wgpu application uses >50 MB RSS

**Factors that increase usage**:
- High-refresh-rate monitors (reported: 300 MB on a 240 Hz 49" ultra-wide)
- Multiple monitors
- Complex bar configurations with many modules

**Possible improvements**:
- Adding a `tiny-skia` CPU renderer could reduce RAM by ~80 MB (at the cost of CPU usage and battery)
- Users can set `renderer.backend = "egl"` in the iced configuration as a partial workaround

**Related**: [GitHub Issue #529](https://github.com/MalpenZibo/ashell/issues/529)

## Services Refactoring

**Issue**: The service layer has inconsistencies across different services. Some use broadcast channels, others use mpsc. Error handling patterns vary.

**Status**: This is an ongoing refactoring effort tracked in [GitHub Issue #445](https://github.com/MalpenZibo/ashell/issues/445). The network service is the most problematic, with unreliable WiFi scanning and incompatible architectural patterns between the NetworkManager and IWD backends.

## No Test Suite

**Issue**: The project currently has no automated test suite. The UI is tested manually, and services are verified by running ashell on real hardware.

**Impact**: Regressions can slip through, especially for compositor-specific behavior (Hyprland vs. Niri) or hardware-specific features (brightness, Bluetooth).

## NVIDIA Compatibility

**Issue**: ashell can crash or fail to render on NVIDIA GPUs with the Niri compositor.

**Workaround**: Set the `WGPU_BACKEND=gl` environment variable to use the OpenGL backend instead of Vulkan.

**Related**: [GitHub Issue #471](https://github.com/MalpenZibo/ashell/issues/471)
