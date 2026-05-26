## Seed translation catalog. New keys land here first; other locales inherit
## via Fluent's negotiation + fallback.

app-name = ashell

## Updates module
updates-up-to-date = Up to date ;)
updates-available =
    { $count ->
        [one] { $count } Update available
       *[other] { $count } Updates available
    }
updates-button-update = Update
updates-button-check-now = Check now

## Media player module
media-player-not-connected = Not connected to MPRIS service
media-player-heading = Players
media-player-loading-cover = Loading cover...
media-player-no-title = No Title
media-player-unknown-artist = Unknown Artist
media-player-unknown-album = Unknown Album

## Password / network connection dialog
password-dialog-open-network-title = Open network
password-dialog-authentication-required-title = Authentication required
password-dialog-open-network-warning =
    "{ $ssid }" is an open network. Data sent over this connection may be visible to others.
    Do you want to connect anyway?
password-dialog-insert-password = Insert password to connect to: { $ssid }
password-dialog-cancel = Cancel
password-dialog-confirm = Confirm

## OSD
osd-airplane-toggle =
    { $state ->
        [on] Airplane mode turned on
       *[off] Airplane mode turned off
    }
osd-idle-inhibitor-toggle =
    { $state ->
        [on] Idle inhibitor turned on
       *[off] Idle inhibitor turned off
    }

## Settings — shared
settings-scanning = Scanning...
settings-more = More

## Settings — network
settings-network-wifi = Wi-Fi
settings-network-vpn = VPN
settings-network-vpns-connected =
    { $count ->
        [one] { $count } VPN Connected
       *[other] { $count } VPNs Connected
    }
settings-network-airplane-mode = Airplane Mode
settings-network-nearby-wifi = Nearby Wifi

## Settings — bluetooth
settings-bluetooth = Bluetooth
settings-bluetooth-devices = Bluetooth Devices
settings-bluetooth-known-devices = Known devices
settings-bluetooth-available = Available
settings-bluetooth-pair = Pair
settings-bluetooth-no-devices = No devices found
settings-bluetooth-connected-count =
    { $count ->
        [one] { $count } device
       *[other] { $count } devices
    }

## Settings — power
settings-power-suspend = Suspend
settings-power-hibernate = Hibernate
settings-power-reboot = Reboot
settings-power-shutdown = Shutdown
settings-power-logout = Logout
settings-power-calculating = Calculating...
settings-power-full-in = Full in { $duration }
settings-power-empty-in = Empty in { $duration }
settings-power-profile-balanced = Balanced
settings-power-profile-performance = Performance
settings-power-profile-power-saver = Power Saver

## Settings — idle inhibitor
settings-idle-inhibitor = Idle Inhibitor

## Tempo / weather module
tempo-feels-like = Feels like { $value }{ $unit }
tempo-humidity = Humidity
tempo-wind = Wind

## Weather conditions (Open-Meteo WMO codes)
weather-clear-sky = Clear sky
weather-mainly-clear = Mainly clear
weather-partly-cloudy = Partly cloudy
weather-overcast = Overcast
weather-fog = Fog
weather-fog-rime = Depositing rime fog
weather-drizzle-light = Light drizzle
weather-drizzle-moderate = Moderate drizzle
weather-drizzle-dense = Dense intensity drizzle
weather-drizzle-freezing-light = Light freezing drizzle
weather-drizzle-freezing-dense = Dense intensity freezing drizzle
weather-rain-slight = Slight rain
weather-rain-moderate = Moderate rain
weather-rain-heavy = Heavy intensity rain
weather-rain-freezing-light = Light freezing rain
weather-rain-freezing-heavy = Heavy intensity freezing rain
weather-snow-slight = Slight snow fall
weather-snow-moderate = Moderate snow fall
weather-snow-heavy = Heavy intensity snow fall
weather-snow-grains = Snow grains
weather-rain-showers-slight = Slight rain showers
weather-rain-showers-moderate = Moderate rain showers
weather-rain-showers-violent = Violent rain showers
weather-snow-showers-slight = Slight snow showers
weather-snow-showers-heavy = Heavy snow showers
weather-thunderstorm = Slight or moderate thunderstorm
weather-thunderstorm-hail-slight = Thunderstorm with slight hail
weather-thunderstorm-hail-heavy = Thunderstorm with heavy hail
weather-unknown = Unknown weather condition

## Notifications module
notifications-heading = Notifications
notifications-empty = No notifications
notifications-group-count = { $count } new

## System info module
system-info-heading = System Info
system-info-cpu-usage = CPU Usage
system-info-memory-usage = Memory Usage
system-info-swap-memory-usage = Swap Memory Usage
system-info-swap-indicator-prefix = swap
system-info-temperature = Temperature
system-info-disk-usage = Disk Usage { $mount }
system-info-ip-address = IP Address
system-info-download-speed = Download Speed
system-info-upload-speed = Upload Speed
