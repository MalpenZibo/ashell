# Project Layout

## Root Directory

```
ashell/
├── src/                     # Rust source code
├── assets/                  # Fonts and icons
├── .github/workflows/       # CI/CD pipelines
├── website/                 # User-facing Docusaurus website
├── docs/                    # This developer guide (mdbook)
├── build.rs                 # Build script (font subsetting, git hash)
├── Cargo.toml               # Dependencies and project metadata
├── Cargo.lock               # Locked dependency versions
├── Makefile                 # Development convenience targets
├── flake.nix                # Nix development environment
├── dist-workspace.toml      # cargo-dist release configuration
├── README.md                # Project overview
├── CHANGELOG.md             # Version history
└── LICENSE                  # MIT License
```

## Source Tree

```
src/
├── main.rs                  # Entry point: logging, CLI args, iced daemon launch
├── app.rs                   # App struct, Message enum, update/view/subscription
├── config.rs                # TOML config parsing, defaults, hot-reload via inotify
├── outputs.rs               # Multi-monitor management, layer surface creation
├── theme.rs                 # Theme system: colors, spacing, fonts, bar styles
├── menu.rs                  # Menu lifecycle: open/toggle/close, layer switching
├── password_dialog.rs       # Password prompt dialog for network auth
│
├── components/
│   └── icons.rs             # Nerd Font icon constants (~80+ icons)
│
├── modules/                 # UI modules (what the user sees in the bar)
│   ├── mod.rs               # Module registry, routing, section builder
│   ├── clock.rs             # Time display (deprecated, use Tempo)
│   ├── tempo.rs             # Advanced clock: timezones, calendar, weather
│   ├── workspaces.rs        # Workspace indicators and switching
│   ├── window_title.rs      # Active window title display
│   ├── system_info.rs       # CPU, RAM, disk, network, temperature
│   ├── keyboard_layout.rs   # Keyboard layout indicator
│   ├── keyboard_submap.rs   # Hyprland submap display
│   ├── tray.rs              # System tray icon integration
│   ├── media_player.rs      # MPRIS media player control
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
│   │   └── niri.rs           # Niri IPC integration
│   ├── audio.rs             # PulseAudio/PipeWire audio service
│   ├── brightness.rs        # Display brightness via sysfs
│   ├── bluetooth/
│   │   ├── mod.rs            # Bluetooth service logic
│   │   └── dbus.rs           # BlueZ D-Bus proxy definitions
│   ├── network/
│   │   ├── mod.rs            # Network service logic
│   │   ├── dbus.rs           # NetworkManager D-Bus proxies
│   │   └── iwd_dbus/         # IWD (iNet Wireless Daemon) D-Bus bindings
│   ├── mpris/
│   │   ├── mod.rs            # Media player service
│   │   └── dbus.rs           # MPRIS D-Bus proxies
│   ├── tray/
│   │   ├── mod.rs            # System tray service
│   │   └── dbus.rs           # StatusNotifierItem D-Bus proxies
│   ├── upower/
│   │   ├── mod.rs            # Battery/power service
│   │   └── dbus.rs           # UPower D-Bus proxies
│   ├── privacy.rs           # Privacy monitoring (PipeWire portals)
│   ├── idle_inhibitor.rs    # Idle/sleep prevention
│   ├── logind.rs            # systemd-logind (sleep/wake detection)
│   └── throttle.rs          # Stream rate-limiting utility
│
├── utils/
│   ├── mod.rs               # Utility module exports
│   ├── launcher.rs          # Shell command execution
│   └── remote_value.rs      # Remote state tracking with local cache
│
└── widgets/                 # Custom iced widgets
    ├── mod.rs               # Widget exports, ButtonUIRef type
    ├── centerbox.rs         # Three-column layout (left/center/right)
    ├── position_button.rs   # Button that reports its screen position
    └── menu_wrapper.rs      # Menu container with backdrop overlay
```

## Assets

```
assets/
├── SymbolsNerdFont-Regular.ttf       # Nerd Font (source, ~2.4 MB)
├── SymbolsNerdFontMono-Regular.ttf   # Nerd Font Mono (source, ~2.4 MB)
├── AshellCustomIcon-Regular.otf      # Custom ashell icons (~8 KB)
├── battery/                           # Battery state SVG icons
├── weather_icon/                      # Weather condition icons
└── ashell_custom_icon_project.gs2     # Glyphs Studio project file
```

The full Nerd Font files in `assets/` are the source. At build time, `build.rs` subsets them into `target/generated/` containing only the glyphs actually used in the code.

## Other Directories

- **`website/`** — The user-facing documentation site built with [Docusaurus](https://docusaurus.io/). Deployed to GitHub Pages. This is separate from this developer guide.
- **`.github/workflows/`** — CI/CD pipeline definitions. See [CI Pipeline](../ci-and-release/ci-pipeline.md).
