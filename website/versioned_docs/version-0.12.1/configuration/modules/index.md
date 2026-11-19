# 🧩 Modules

Ashell modules identify the various features of the status bar.

Each module provides a set of functionalities that can be enabled
or disabled in the configuration file.

## Organize modules

The status bar is divided into three main sections: left, center, and right.

Each section holds a list of modules or module groups,
allowing flexible layout configuration.

Modules can be used on their own or organized into groups.

### Default configuration

```toml
[modules]
left = [ "Workspaces" ]
center = [ "WindowTitle" ]
right = [ [ "Tempo", "Privacy", "Settings" ] ]
```

### Example

If we want to add the `SystemInfo` module to the right side of
the status bar but not in the same group as the `Tempo`, `Privacy`,
and `Settings` modules, we can do it like this:

```toml
right = [ "SystemInfo", [ "Tempo", "Privacy", "Settings" ] ]
```

## Available modules

The following modules are available:

### Updates

Provides information about available updates for the system.

:::info
This module requires additional configuration to work properly.
See the dedicated section in the [documentation](./updates.md).
:::

### Workspaces

Provides information about the current workspaces and allows switching between them.

### WindowTitle

Displays the title of the currently focused window.

### SystemInfo

Displays system information such as CPU usage, memory usage, and disk space.

### KeyboardLayout

Displays the current keyboard layout and allows switching between layouts.
Available on Hyprland, Niri and MangoWC.

### KeyboardSubmap

Displays the current keyboard submap, when one is active. Available on Hyprland
and MangoWC.

### Custom modules

Custom modules are declared in `[[CustomModule]]` blocks and placed in a section
by their own `name`, not by the literal string `CustomModule`. Any name in
`[modules]` that is not one of the built-in modules above is resolved as a
custom module. See the [custom module documentation](./custom_module.md) for
details.

### Tray

Displays system tray icons and menus for applications.

### Tempo

Pairs a customizable clock with compact weather info in the bar, plus a menu that shows forecasts, hourly breakdowns, and a calendar. See the [Tempo docs](./tempo.md) for full configuration details.

### Privacy

Shows an indicator while the microphone, the webcam or screen sharing is in use.
It is read-only: it reports usage, it does not toggle access.

### MediaPlayer

Displays media player controls and information about the currently playing media.

### Notifications

Desktop notification daemon with toast popups and a notification center menu.
See the [Notifications docs](./notifications.md) for full configuration details.

### Settings

Provides access to system settings like audio, network, Bluetooth, battery,
power profile, and idle inhibitor.
