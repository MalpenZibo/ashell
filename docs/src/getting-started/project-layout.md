# Project Layout

## Root Directory

```
ashell/
├── src/                     # Rust source code (the ashell binary)
├── crates/                  # Workspace library crates (see below)
├── assets/                  # Fonts and icons
├── .github/workflows/       # CI/CD pipelines
├── website/                 # User-facing Docusaurus website
├── docs/                    # This developer guide (mdbook)
├── i18n/                    # Fluent translation catalogs (en-US, de-DE, fr-FR, ru-RU, uk-UA)
├── i18n.toml                # i18n-embed configuration
├── build.rs                 # Build script (font subsetting, git hash)
├── Cargo.toml               # Dependencies and project metadata
├── Cargo.lock               # Locked dependency versions
├── Makefile                 # Development convenience targets
├── flake.nix                # Nix development environment
├── dist-workspace.toml      # cargo-dist release configuration
├── README.md                # Project overview
├── CHANGELOG.md             # Version history
└── LICENSE                  # GPL-3.0-or-later License
```

## Workspace Crates

```
crates/
├── ashell-i18n/             # Localization: locale and unit system, catalog loading, t!() macro
└── ashell-services/         # UI-agnostic backend services (no iced dependency)
```

`ashell-i18n` has no catalogs of its own: each app passes its catalogs to `Localizer::resolve`. `t!()` checks message keys at compile time against `{assets_dir}/{fallback_language}/{domain}.ftl` of the crate that calls it, as set in that crate's `i18n.toml`. The domain defaults to the crate name with `-` replaced by `_` (`my-app` uses `my_app.ftl`), and `fallback_language` must be `en-US`. At runtime only the catalogs passed to `Localizer::resolve` count. `t!()` translates only on a thread that called `init_localizer`.

## Source Tree

```
src/
├── main.rs                  # Entry point: logging, CLI args, iced application launch
├── app.rs                   # App struct, update/view/subscription
├── app/
│   ├── message.rs           # Top-level Message enum
│   └── osd_info.rs          # OSD display info for IPC commands
├── config.rs                # TOML config parsing, defaults, hot-reload via inotify
├── outputs.rs               # Multi-monitor management, layer surface creation
├── theme.rs                 # Theme system: colors, spacing, fonts, bar styles
├── osd.rs                   # On-screen display overlay (volume, brightness, ...)
├── ipc.rs                   # IPC socket server and `ashell msg` client
├── i18n.rs                  # Translation catalogs (Fluent)
├── xdg.rs                   # XDG runtime directory lookup
│
├── components/              # Shared UI building blocks and custom iced widgets
│   ├── mod.rs               # Component exports
│   ├── icons.rs             # Nerd Font icon definitions (`StaticIcon`)
│   ├── menu.rs              # Menu lifecycle: open/toggle/close, menu surfaces
│   ├── menu_wrapper.rs      # Menu container with backdrop overlay
│   ├── password_dialog.rs   # Password prompt dialog for network auth
│   ├── centerbox.rs         # Three-column layout (left/center/right)
│   ├── position_button.rs   # Button that reports its screen position (`ButtonUIRef`)
│   └── ...                  # Buttons, sliders, animations, module groups, etc.
│
├── modules/                 # UI modules (what the user sees in the bar)
│   ├── mod.rs               # Module registry, routing, section builder
│   ├── tempo/               # Clock: timezones, calendar, weather
│   │   ├── mod.rs
│   │   ├── calendar.rs
│   │   └── weather.rs
│   ├── workspaces.rs        # Workspace indicators and switching
│   ├── window_title.rs      # Active window title display
│   ├── system_info.rs       # CPU, RAM, disk, network, temperature
│   ├── keyboard_layout.rs   # Keyboard layout indicator
│   ├── keyboard_submap.rs   # Hyprland submap display
│   ├── tray.rs              # System tray icon integration
│   ├── media_player.rs      # MPRIS media player control
│   ├── notifications.rs     # Notification center and toasts
│   ├── privacy.rs           # Microphone/camera/screenshare indicators
│   ├── updates.rs           # Package update checker
│   ├── custom_module.rs     # User-defined custom modules
│   └── settings/            # Settings panel (complex, multi-part)
│       ├── mod.rs            # Settings container and navigation
│       ├── audio.rs          # Volume and audio device control
│       ├── bluetooth.rs      # Bluetooth device management
│       ├── brightness.rs     # Screen brightness slider
│       ├── network.rs        # WiFi and VPN management
│       └── power.rs          # Power menu (shutdown, reboot, sleep)
│
├── services/                # Backend system integrations (no UI)
│   ├── mod.rs               # Service traits (ReadOnlyService, Service)
│   ├── compositor/          # Window manager abstraction
│   │   ├── mod.rs            # Compositor service, backend detection, broadcast
│   │   ├── types.rs          # CompositorState, CompositorEvent, CompositorCommand
│   │   ├── hyprland.rs       # Hyprland IPC integration
│   │   ├── niri.rs           # Niri IPC integration
│   │   ├── mangowc.rs        # MangoWC integration (`mmsg` IPC)
│   │   └── generic.rs        # Generic Wayland fallback (ext-workspace, foreign-toplevel)
│   ├── audio.rs             # PulseAudio/PipeWire audio service
│   ├── brightness.rs        # iced glue for the ashell-services brightness service
│   ├── bluetooth.rs         # iced glue for the ashell-services Bluetooth service
│   ├── network/
│   │   ├── mod.rs            # Network service logic
│   │   ├── dbus.rs           # NetworkManager D-Bus proxies
│   │   └── iwd_dbus/         # IWD (iNet Wireless Daemon) D-Bus bindings
│   ├── mpris/
│   │   ├── mod.rs            # Media player service
│   │   └── dbus.rs           # MPRIS D-Bus proxies
│   ├── notifications/
│   │   ├── mod.rs            # Notification daemon service
│   │   └── dbus.rs           # org.freedesktop.Notifications D-Bus interface
│   ├── tray/
│   │   ├── mod.rs            # System tray service
│   │   └── dbus.rs           # StatusNotifierItem D-Bus proxies
│   ├── upower.rs            # iced glue for the ashell-services UPower service
│   ├── privacy.rs           # iced glue for the ashell-services privacy service
│   ├── idle_inhibitor.rs    # Idle/sleep prevention
│   ├── logind.rs            # systemd-logind (sleep/wake detection)
│   └── xdg_icons.rs         # iced glue for the ashell-services icon lookup
│
└── utils/
    ├── mod.rs               # Utility module exports and helpers
    ├── launcher.rs          # Shell command execution
    └── remote_value.rs      # Remote state tracking with local cache
```

## Assets

```
assets/
├── SymbolsNerdFont-Regular.ttf       # Nerd Font (source, ~2.4 MB)
├── SymbolsNerdFontMono-Regular.ttf   # Nerd Font Mono (source, ~2.4 MB)
├── AshellCustomIcon-Regular.otf      # Custom ashell icons (~6 KB)
├── *_bat_*.svg, key_backlight_*.svg   # Peripheral battery / keyboard backlight SVG icons
├── weather_icon/                      # Weather condition icons
└── ashell_custom_icon_project.gs2     # Glyphs Studio project file
```

The full Nerd Font files in `assets/` are the source. At build time, `build.rs` subsets them into `target/generated/` containing only the glyphs actually used in the code.

## Other Directories

- **`website/`** — The user-facing documentation site built with [Docusaurus](https://docusaurus.io/). Deployed to GitHub Pages. This is separate from this developer guide.
- **`.github/workflows/`** — CI/CD pipeline definitions. See [CI Pipeline](../ci-and-release/ci-pipeline.md).
