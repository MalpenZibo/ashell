## Deutsche Übersetzung. Neue Schlüssel landen zuerst im en-US Katalog,
## danach werden diese hier übernommen.
## Übersetzung bereitgestellt via Absprache durch Fluent + Ausweichlösung.

app-name = ashell

## Updates module
updates-up-to-date = Alles aktuell ;)
updates-available =
    { $count ->
        [one] { $count } Update verfügbar
       *[other] { $count } Updates verfügbar
    }
updates-button-update = Update
updates-button-check-now = Jetzt Prüfen

## Media player module
media-player-not-connected = Nicht mit dem MPRIS-Dienst verbunden
media-player-heading = Spieler
media-player-loading-cover = Lade Cover ...
media-player-no-title = Kein Titel
media-player-unknown-artist = Unbekannter Künstler
media-player-unknown-album = Unbekanntes Album

## Password / network connection dialog
password-dialog-open-network-title = Netzwerk verbinden
password-dialog-authentication-required-title = Authentifizierung benötigt
password-dialog-open-network-warning =
    "{ $ssid }" ist ein öffentliches Netzwerk. Gesendete Daten können von Anderen beobachtet werden.
    Trotzdem verbinden?
password-dialog-insert-password = Passwort eingeben für: { $ssid }
password-dialog-cancel = Abbrechen
password-dialog-confirm = Bestätigen

## OSD
osd-airplane-toggle =
    { $state ->
        [on] Flugzeugmodus an
       *[off] Flugzeugmodus aus
    }
osd-idle-inhibitor-toggle =
    { $state ->
        [on] Nicht-Schlafen an
       *[off] Nicht-Schlafen aus
    }

## Settings — shared
settings-scanning = Scanne...
settings-more = Mehr

## Settings — network
settings-network-wifi = Netze
settings-network-vpn = VPN
settings-network-vpns-connected =
    { $count ->
        [one] { $count } VPN verbunden
       *[other] { $count } VPNs verbunden
    }
settings-network-airplane-mode = Flugzeugmodus
settings-network-nearby-wifi = Verfügbare Netze

## Settings — bluetooth
settings-bluetooth = Bluetooth
settings-bluetooth-devices = Bluetooth Geräte
settings-bluetooth-known-devices = Bekannte Geräte
settings-bluetooth-available = Verfügbare
settings-bluetooth-pair = Paaren
settings-bluetooth-no-devices = Keine Geräte gefunden
settings-bluetooth-connected-count =
    { $count ->
        [one] { $count } Gerät
       *[other] { $count } Geräte
    }

## Settings — power
settings-power-suspend = Suspend
settings-power-hibernate = Hibernate
settings-power-reboot = Neustart
settings-power-shutdown = Herunterfahren
settings-power-logout = Abmelden
settings-power-calculating = Berechne ...
settings-power-full-in = Voll in { $duration }
settings-power-empty-in = Leer in { $duration }
settings-power-charge-limit = Laden limitieren
settings-power-profile-balanced = Ausbalanciert
settings-power-profile-performance = Leistung
settings-power-profile-power-saver = Stromsparen

settings-power-status-charging = Lädt
settings-power-status-discharging = Entlädt
settings-power-status-full = Voll

## Settings — idle inhibitor
settings-idle-inhibitor = Nicht-Schlafen

## Settings — tooltips
settings-tooltip-empty = Ziemlich leer hier
settings-tooltip-empty-audio = Kein aktives Audio-Gerät
settings-tooltip-empty-bluetooth = Kein verbundes Gerät
settings-tooltip-empty-wifi = Kein Netz verbunden
settings-tooltip-empty-vpn = Kein aktiver VPN
settings-tooltip-empty-battery = Keine Batterie-Infos

## Tempo / weather module
tempo-feels-like = Gefühlt { $value }{ $unit }
tempo-humidity = Luftfeuchte
tempo-wind = Wind

## Weather conditions (Open-Meteo WMO codes)
weather-clear-sky = Klarer Himmel
weather-mainly-clear = Größtenteils klar
weather-partly-cloudy = Teilweise bewölkt
weather-overcast = Verhangen
weather-fog = Nebel
weather-fog-rime = Depositing rime fog
weather-drizzle-light = Leichter Nieselregen
weather-drizzle-moderate = Nieselregen
weather-drizzle-dense = Dichter Nieselregen
weather-drizzle-freezing-light = Leichter gefrierender Nieselregen
weather-drizzle-freezing-dense = Dichter gefrierender Nieselregen
weather-rain-slight = Leichter Regen
weather-rain-moderate = Regen
weather-rain-heavy = Starkregen
weather-rain-freezing-light = Leichter gefrierender Regen
weather-rain-freezing-heavy = Starker gefrierender Regen
weather-snow-slight = Leichter Schneefall
weather-snow-moderate = Schneefall
weather-snow-heavy = Dichter Schneefall
weather-snow-grains = Schneekörner
weather-rain-showers-slight = Leichter Regenschauer
weather-rain-showers-moderate = Regenschauer
weather-rain-showers-violent = Dichter Regenschauer
weather-snow-showers-slight = Leichter Schneeschauer
weather-snow-showers-heavy = Dichter Schneeschauer
weather-thunderstorm = Gewitter
weather-thunderstorm-hail-slight = Gewitter mit leichten Hagel
weather-thunderstorm-hail-heavy = Gewitter mit schwerem Hagel
weather-unknown = Unbekanntes Wetter

## Notifications module
notifications-heading = Benachrichtigungen
notifications-empty = Keine Benachrichtigungen
notifications-group-count = { $count } Neue

## System info module
system-info-heading = System Info
system-info-cpu-usage = Prozessor Auslastung
system-info-memory-usage = Arbeitsspeicher Auslastung
system-info-swap-memory-usage = Swap Auslastung
system-info-swap-indicator-prefix = swap
system-info-temperature = Temperatur
system-info-disk-usage = Festplatten Auslastung { $mount }
system-info-ip-address = IP Addresse
system-info-download-speed = Download Geschwindigkeit
system-info-upload-speed = Upload Geschwindigkeit
