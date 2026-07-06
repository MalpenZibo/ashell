# Known Limitations and Design Debt

This chapter documents known architectural limitations and design tradeoffs. Understanding these helps contributors make informed decisions and avoid re-discovering known issues.

## Menu Surface Architecture

**Issue**: Context menus currently use a fullscreen transparent overlay surface. This has several drawbacks:

- **Memory waste**: ~140 MB VRAM per 4K monitor for a transparent surface
- **Layering bugs**: The overlay doesn't correctly layer popups on top of other windows in all cases
- **Conflicts**: Can interfere with other layer surfaces on the Background layer

**Correct approach**: Use `zwlr_layer_surface_v1::get_popup` to create proper `xdg_popup` surfaces. However, iced_layershell does not currently expose this.

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
