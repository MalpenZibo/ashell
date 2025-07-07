# 🧩 Modules

Ashell modules identifies the various features of the status bar.

Each modules provide a set of functionality that can be enabled
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
right = [ [ "Clock", "Privacy", "Settings" ] ]
```

### Example

If we want to add the `SystemInfo` module in the right side of
the status bar but not in the same group as the `Clock`, `Privacy`,
and `Settings` modules, we can do it like this:

```toml
right = [ "SystemInfo", [ "Clock", "Privacy", "Settings" ] ]
```

## Available modules

The following modules are available:

### AppLauncher

Provides a way to launch applications from the status bar.

:::info

This module requires additional configuration to work properly.
See the dedicated section in the [documentation](./app_launcher.md).

:::

:::warning

This module will be deprecated in the futures releases

:::

### Updates

Provide information about available updates for the system.

:::info

This module requires additional configuration to work properly.
See the dedicated section in the [documentation](./updates.md).

:::

### Clipboard

Launch a clipboard manager.

:::info

This module requires additional configuration to work properly.
See the dedicated section in the [documentation](./clipboard.md).

:::

:::warning

This module will be deprecated in the futures releases

:::

### Workspaces

Provides information about the current workspaces and allows switching between them.

### WindowTitle

Displays the title of the currently focused window.

### SystemInfo

Displays system information such as CPU usage, memory usage, and disk space.

### KeyboardLayout

Displays the current keyboard layout and allows switching between layouts.

### KeyboardSubmap

Displays the current keyboard submap.

### Tray

Displays system tray icons and menus for applications.

### Clock

Displays the current time and date.

### Privacy

Provides privacy-related features, such as toggling microphone and camera access.

### MediaPlayer

Displays media player controls and information about the currently playing media.

### Settings

Provides access to system settings like audio, network, bluetooth,
battery, power profile and idle inhibitor.
