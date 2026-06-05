<h1 align="center">
  <a href="https://malpenzibo.github.io/ashell/">
    <img src="https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/logo_header_dark.svg" alt="ashell" height="140"/>
  </a>
</h1>
<p align="center">A ready to go Wayland status bar for Hyprland and Niri.</p>
<p align="center">
    <a href="https://matrix.to/#/#ashell:matrix.org"><img alt="Matrix" src="https://img.shields.io/badge/matrix-%23ashell-blue?logo=matrix"></a>
    <a href="https://github.com/MalpenZibo/ashell/blob/main/LICENSE"><img alt="GitHub License" src="https://img.shields.io/github/license/MalpenZibo/ashell"></a>
    <a href="https://github.com/MalpenZibo/ashell/releases"><img alt="GitHub Release" src="https://img.shields.io/github/v/release/MalpenZibo/ashell?logo=github"></a>
</p>

<p align="center">
    <a href="https://malpenzibo.github.io/ashell/docs/intro">Getting Started</a> | <a href="https://malpenzibo.github.io/ashell/docs/configuration">Configuration</a> | <a href="https://malpenzibo.github.io/ashell/dev-guide/">Developer&nbsp;Guide</a>
</p>

## Фичи
- Поддержка 2-х языков (en, ru)
- Автоматическое распознавание композитора Hyprland/Niri
- Multi-monitor support (all monitors, active monitor, or specific targets)
- Hot-reload configuration (changes apply automatically via file watch)
- Bar positioning (top or bottom) with configurable layer (Bottom, Top, Overlay)
- Theming: Islands, Solid, and Gradient styles with custom colors, opacity, scale, and fonts
- OS Updates indicator with configurable check interval
- Hyprland/Niri Active Window (title, class, or initial title/class)
- Hyprland/Niri Workspaces with naming, color coding, and per-monitor visibility
- System Information (CPU, RAM, Disk, IP address, Network speed, Temperature) with warn/alert thresholds
- Hyprland/Niri Keyboard Layout with custom labels
- Hyprland Keyboard Submap
- System Tray with context menus
- Clock with calendar, weather, timezone cycling, and format cycling (Tempo)
- Privacy indicators (microphone, camera, and screenshare usage)
- Media Player with album art and track info
- Notification manager with toast popups, grouping, and urgency support
- Settings panel
  - Power menu (shutdown, suspend, hibernate, reboot, logout, lock)
  - Battery and peripheral battery information
  - Audio sources and sinks (with microphone)
  - Screen brightness
  - Network (WiFi scanning, password entry; supports NetworkManager and IWD backends)
  - VPN
  - Bluetooth
  - Power profiles
  - Idle inhibitor
  - Airplane mode
  - Custom quick-action buttons with status commands
- IPC socket for scripting and keybindings (`ashell msg <command>`)
- OSD overlay for volume, brightness, and airplane mode changes
- Custom Modules
  - Button (execute command on click)
  - Text (display-only, update UI with command output via `listen_cmd`)
  - Regex-based icon mapping and alert states

## Установка
Собрать из исходников или закинуть бинарник из последнего релиза в /bin


## Конфигурация
Основную массу конфигурации смотри на сайте исходного проекта, из коробки **работает конфиг исходного проекта**
 [Configuration](https://malpenzibo.github.io/ashell/docs/configuration)
### Что добавлено в этом форке
- Модуль истории буфера обмена (для работы нужен cliphist), по-умолчанию показывает последние 8 записей для настройки
```
[clipboard]
max_entries = 100 
```
- Глубокая настройка оформления
  ##### `ModuleAppearance` — переопределение стиля для каждого модуля

Указывается в секции `[appearance.module_styles]` конфига TOML. Ключ — имя модуля.

| Параметр | Тип | Описание | Fallback |
|---|---|---|---|
| `opacity` | `f32` (0.0–1.0) | Непрозрачность модуля | `appearance.opacity` |
| `background_color` | `BackgroundAppearanceColor` | Цвет фона (в стиле Islands) | `appearance.background_color` |
| `text_color` | `AppearanceColor` | Цвет текста модуля | `appearance.text_color` |
| `border_radius` | `f32` | Радиус скругления (пиксели) | `theme.radius.lg` (16.0) |

**Имена модулей (ключи):**
`Updates`, `Workspaces`, `WindowTitle`, `SystemInfo`, `KeyboardLayout`, `KeyboardSubmap`, `Tray`, `Tempo`, `Privacy`, `Settings`, `MediaPlayer`, `Clipboard`, `Notifications` или имя кастомного модуля.

---

#### Параметры оформления поп-апов (меню)

##### `PopupAppearance` — переопределение стиля для каждого поп-апа

Указывается в секции `[appearance.popup_styles]`. Ключ — тип поп-апа.

| Параметр           | Тип                         | Описание                    | Fallback                      |
| ------------------ | --------------------------- | --------------------------- | ----------------------------- |
| `opacity`          | `f32` (0.0–1.0)             | Непрозрачность поп-апа      | `appearance.menu.opacity`     |
| `backdrop`         | `f32`                       | Затемнение фона за поп-апом | `appearance.menu.backdrop`    |
| `background_color` | `BackgroundAppearanceColor` | Цвет фона поп-апа           | `appearance.background_color` |
| `border_radius`    | `f32`                       | Радиус скругления           | `theme.radius.lg` (16.0)      |
| `width`            | `MenuSizeConfig`            | Размер поп-апа              | Зависит от типа меню          |

**Ключи поп-апов:** `Updates`, `Settings`, `Notifications`, `Tray`, `MediaPlayer`, `SystemInfo`, `Tempo`, `Clipboard`

**Значения `MenuSizeConfig`:**
| Значение | Ширина |
|---|---|
| `Small` | 250px |
| `Medium` | 350px |
| `Large` | 450px |
| `XLarge` | 650px |

---

#### Формат цветов

##### `AppearanceColor` (для text_color, primary_color и т.д.)
- **Простой:** `"#RRGGBB"` — например `"#7AA2F7"`
- **Полный:**
  ```toml
  { base = "#7AA2F7", strong = "#5A82D7", weak = "#9AC2FF", text = "#1A1B26" }
  ```

##### `BackgroundAppearanceColor` (для background_color)
- **Простой:** `"#1A1B26"`
- **Полный:**
  ```toml
  { base = "#1A1B26", weakest = "#11111b", weaker = "#161622", weak = "#24273a", neutral = "#2a2e42", strong = "#414868", stronger = "#565a7e", strongest = "#6a6f94", text = "#a9b1d6" }
  ```
## Авторский конфиг(именно его видите на скринах
```
position = "Top"
layer = "Overlay"
outputs = "All"

# Высота панели в пикселях (по умолчанию 40)
height = 40
[modules]
left = [ "appLauncher", "Workspaces", "SystemInfo" , "Tray" ] 
center = ["Notifications", "Tempo", "MediaPlayer"]
right = ["Clipboard", "KeyboardLayout", "Privacy", "settings", "Settings" ]


[system_info.cpu]
warn_threshold = 60
alert_threshold = 80

[system_info.memory]
warn_threshold = 70
alert_threshold = 85

[system_info.disk]
warn_threshold = 80
alert_threshold = 90

[system_info.temperature]
warn_threshold = 60
alert_threshold = 80
sensor = "coretemp Package id 0"

[settings]
battery_format = "IconAndPercentage"
peripheral_battery_format = "Icon"
peripheral_indicators = { Specific = ["Gamepad", "Keyboard"] }
audio_indicator_format = "Icon"
network_indicator_format = "Icon"
bluetooth_indicator_format = "Icon"
brightness_indicator_format = "Icon"

[keyboard_layout.labels]
"English (US)" = "󰌌 US"
"Russian" = "󰌌 RU"

[notifications]
enable_server = true

[media_player]
max_title_length = 50
indicator_format = "Icon"

[[CustomModule]]
name = "appLauncher"
icon = "󱗼"
command = "nwg-drawer"

[[CustomModule]]
name = "settings"
icon = "⚙"
command = "env XDG_CURRENT_DESKTOP=GNOME gnome-control-center"


[tempo]

weather_location = { City = "Volzhsky" }
weather_indicator = "Icon"

[appearance]
scale_factor =1.1

success_color = "#a3be8c"
text_color = "#eceff4"
workspace_colors = [   "#DCD0FF" ]
menu.opacity= 0.8

[appearance.primary_color]
base = "#DCD0FF"
text = "#242933"

[appearance.danger_color]
base = "#bf616a"
weak = "#ebcb8b"

[appearance.background_color]
base = "#3b4252"
weak = "#434c5e"
strong = "#4c566a"

[appearance.secondary_color]
base = "#DCD0FF"
strong = "#DCD0FF"

# Цвета модулей


[appearance.module_styles.appLauncher]
text_color = "#2E3440"
background_color = "#F4C2C2"
opacity = 0.7

[appearance.module_styles.Workspaces]
text_color = "#2E3440"
background_color = "#A2A2D0"
opacity = 0.7

[appearance.module_styles.SystemInfo]
text_color = "#2E3440"
background_color = "#BBC8F0"
opacity = 0.5

[appearance.module_styles.Tray]
text_color = "#2E3440"
background_color = "#F3E8B0"
opacity = 0.2






[appearance.module_styles.Notifications]
text_color = "#2E3440"
background_color = "#F4C2C2"
opacity = 0.5

[appearance.module_styles.Tempo]
text_color = "#2E3440"
background_color = "#B8E2C8"

[appearance.module_styles.MediaPlayer]
text_color = "#2E3440"
background_color = "#BBC8F0"
opacity = 0.7






[appearance.module_styles.Clipboard]
text_color = "#2E3440"
background_color = "#F3E8B0"
opacity = 0.7

[appearance.module_styles.KeyboardLayout]
text_color = "#2E3440"
background_color = "#F4C2C2"
opacity = 0.5

[appearance.module_styles.Privacy]
text_color = "#2E3440"
background_color = "#B8E2C8"
opacity = 0.7

[appearance.module_styles.settings]
text_color = "#2E3440"
background_color = "#BBC8F0"
opacity = 0.7

[appearance.module_styles.Settings]
text_color = "#2E3440"
background_color = "#FADADD"






```

## 💬 Community

Join the conversation on [Matrix](https://matrix.to/#/#ashell:matrix.org) or open an
[issue](https://github.com/MalpenZibo/ashell/issues) on GitHub.

## 📖 Developer Guide

If you want to contribute or understand the codebase, check out the
[Developer Guide](https://malpenzibo.github.io/ashell/dev-guide/).

## 🤖 AI-Assisted Contributions

AI-assisted contributions are accepted — the same quality standards apply regardless of how
the code was written. Frontier-class models (e.g., Claude Opus or equivalent) are strongly
recommended. **You are responsible for the code you submit**: review AI output carefully,
ensure `make check` passes, and be prepared to explain your changes.

Before working on a feature or large change, **discuss it with maintainers first**.
Small, incremental PRs are preferred — code review is manual and remains the bottleneck.

For the full AI contribution guide, see the
[Developer Guide](https://malpenzibo.github.io/ashell/dev-guide/contributing/ai-assisted-contributions.html).

## 📷 Screenshots

I will try my best to keep these screenshots as updated as possible but some details
could be different

#### Авторский конфиг

<img width="1280" height="42" alt="image" src="https://github.com/user-attachments/assets/3592a8aa-4dcb-499d-93af-defca577db76" />
<img width="443" height="362" alt="image" src="https://github.com/user-attachments/assets/6f2e9d4e-807d-4b33-8d99-ddef7b581fc6" />
<img width="433" height="476" alt="image" src="https://github.com/user-attachments/assets/0c138885-209e-4fa4-9cb3-85e826518d02" />

### История буфера обмена
<img width="433" height="569" alt="image" src="https://github.com/user-attachments/assets/77bbf77d-6791-4147-825c-1469a7869c9e" />


#### opacity settings

<img src="https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/opacity.png"></img>

| ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/updates-panel.png)   | ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/system-menu.png)  |
| ------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------- |
   |
## Building from Source
To build Ashell from source, ensure the following dependencies are installed:

-Rust (with cargo)
-wayland-protocols
-clang
-libxkbcommon
-wayland
-dbus
-libpipewire
-libpulse
### Then, from the root of the repository, run:

```sh
cargo build --release
sudo cp target/release/ashell /usr/local/bin/ashell  
