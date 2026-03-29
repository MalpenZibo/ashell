# Testing and Debugging

## Current Test Status

ashell does not currently have an automated test suite. Testing is done manually by running the application on real hardware with Hyprland or Niri.

## Debugging with Logs

ashell writes logs to `/tmp/ashell/`. To watch logs in real time:

```bash
tail -f /tmp/ashell/*.log
```

### Adjusting Log Level

In the config file:

```toml
log_level = "debug"
```

Common levels: `error`, `warn`, `info`, `debug`, `trace`.

You can also set per-module levels:

```toml
log_level = "warn,ashell::services::audio=debug"
```

### Debug Build Logging

In debug builds (`cargo build` without `--release`), all logs are also printed to stdout.

## Common Debugging Scenarios

### Service Not Starting

Check logs for initialization errors:

```bash
grep -i "error\|failed\|panic" /tmp/ashell/*.log
```

### D-Bus Issues

Use `busctl` to check if the D-Bus service is available:

```bash
# System bus services
busctl --system list | grep -i "bluez\|networkmanager\|upower"

# Session bus services
busctl --user list | grep -i "mpris\|statusnotifier"
```

### Compositor Detection

If ashell fails to detect your compositor, check the environment variables:

```bash
echo $HYPRLAND_INSTANCE_SIGNATURE
echo $NIRI_SOCKET
```

### NVIDIA Issues

If you experience rendering issues on NVIDIA, try:

```bash
WGPU_BACKEND=gl ashell
```

## Running a Test Configuration

You can run ashell with a custom config for testing:

```bash
ashell --config-path /tmp/test-config.toml
```

Create a minimal config to test specific features in isolation.

## Hot-Reload Testing

Edit the config file while ashell is running. Changes should apply immediately without restart. Useful for testing:

- Theme changes
- Module layout changes
- Module-specific settings

## Multi-Monitor Testing

If you only have one monitor, you can test multi-monitor behavior by:

- Using virtual outputs in Hyprland
- Using a headless Wayland compositor for basic testing
