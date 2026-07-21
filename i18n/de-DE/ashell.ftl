## Deutsche Übersetzung. Neue Schlüssel landen zuerst im en-US Katalog,
## danach werden diese hier übernommen. Verhandlung erfolgt über Fluent,
## nicht gefundene Schlüssel fallen auf den en-US Katalog zurück.

app-name = ashell

## Updates Modul
updates-up-to-date = Alles aktuell ;)
updates-available =
    { $count ->
        [one] { $count } Update verfügbar
       *[other] { $count } Updates verfügbar
    }
updates-button-update = Jetzt Aktualisieren
updates-button-check-now = Nach Updates prüfen

## Medienwiedergabe Modul
media-player-not-connected = Nicht mit dem MPRIS-Dienst verbunden
media-player-heading = Player
media-player-loading-cover = Cover wird geladen...
media-player-no-title = Kein Titel
media-player-unknown-artist = Unbekannter Künstler
media-player-unknown-album = Unbekanntes Album

## Passwort / Netzwerkkonfiguration
password-dialog-open-network-title = Offenes Netzwerk
password-dialog-authentication-required-title = Authentifizierung erforderlich
password-dialog-open-network-warning =
    "{ $ssid }" ist ein offenes Netzwerk. Daten, die über diese Verbindung gesendet werden, könnten für andere sichtbar sein.
    Trotzdem verbinden?
password-dialog-insert-password = Passwort eingeben für: { $ssid }
password-dialog-cancel = Abbrechen
password-dialog-confirm = Bestätigen

## OSD
osd-airplane-toggle =
    { $state ->
        [on] Flugmodus aktiv
       *[off] Flugmodus inaktiv
    }
osd-idle-inhibitor-toggle =
    { $state ->
        [on] Ruhezustand wird verhindert
       *[off] Ruhezustand wird erlaubt
    }

## Einstellungen — Geteilt
settings-scanning = Wird gesucht...
settings-more = Mehr

## Einstellungen — Netzwerk
settings-network-wifi = WLAN
settings-network-vpn = VPN
settings-network-vpns-connected =
    { $count ->
        [one] { $count } VPN verbunden
       *[other] { $count } VPNs verbunden
    }
settings-network-airplane-mode = Flugmodus
settings-network-nearby-wifi = Verfügbare WLANs

## Einstellungen — Bluetooth
settings-bluetooth = Bluetooth
settings-bluetooth-devices = Bluetooth-Geräte
settings-bluetooth-known-devices = Bekannte Geräte
settings-bluetooth-available = Verfügbar
settings-bluetooth-pair = Koppeln
settings-bluetooth-no-devices = Keine Geräte gefunden
settings-bluetooth-connected-count =
    { $count ->
        [one] { $count } Gerät
       *[other] { $count } Geräte
    }

## Einstellungen — Energie
settings-power-suspend = Ruhezustand
settings-power-hibernate = Tiefschlaf
settings-power-reboot = Neustart
settings-power-shutdown = Herunterfahren
settings-power-logout = Abmelden
settings-power-calculating = Wird berechnet...
settings-power-full-in = Voll in { $duration }
settings-power-empty-in = Leer in { $duration }
settings-power-charge-limit = Ladebegrenzung
settings-power-profile-balanced = Ausgewogen
settings-power-profile-performance = Leistung
settings-power-profile-power-saver = Stromsparen

settings-power-status-charging = Wird geladen
settings-power-status-discharging = Entlädt sich
settings-power-status-not-charging = Lädt nicht
settings-power-status-unknown = Unbekannt
settings-power-status-full = Voll

## Einstellungen — Kein Ruhezustand
settings-idle-inhibitor = Kein Ruhezustand

## Einstellungen – Tastaturbeleuchtung
settings-kbd-backlight = Tastatur

## Einstellungen — tooltips
settings-tooltip-empty = Ziemlich leer hier
settings-tooltip-empty-audio = Kein aktives Audio-Gerät
settings-tooltip-empty-bluetooth = Keine verbundenen Geräte
settings-tooltip-empty-wifi = Kein WLAN verbunden
settings-tooltip-empty-vpn = Kein aktives VPN
settings-tooltip-empty-battery = Keine Akku-Informationen

## Tempo / Wetter Modul
tempo-feels-like = Gefühlte { $value }{ $unit }
tempo-humidity = Luftfeuchte
tempo-wind = Wind

## Wetterbedingungen (Open-Meteo WMO codes)
weather-clear-sky = Klarer Himmel
weather-mainly-clear = Überwiegend klar
weather-partly-cloudy = Teilweise bewölkt
weather-overcast = Bewölkt
weather-fog = Nebel
weather-fog-rime = Raureifnebel
weather-drizzle-light = Leichter Nieselregen
weather-drizzle-moderate = Nieselregen
weather-drizzle-dense = Starker Nieselregen
weather-drizzle-freezing-light = Leichter gefrierender Nieselregen
weather-drizzle-freezing-dense = Starker gefrierender Nieselregen
weather-rain-slight = Leichter Regen
weather-rain-moderate = Regen
weather-rain-heavy = Starker Regen
weather-rain-freezing-light = Leichter gefrierender Regen
weather-rain-freezing-heavy = Starker gefrierender Regen
weather-snow-slight = Leichter Schneefall
weather-snow-moderate = Schneefall
weather-snow-heavy = Starker Schneefall
weather-snow-grains = Schneegriesel
weather-rain-showers-slight = Leichte Regenschauer
weather-rain-showers-moderate = Regenschauer
weather-rain-showers-violent = Heftige Regenschauer
weather-snow-showers-slight = Leichte Schneeschauer
weather-snow-showers-heavy = Dichte Schneeschauer
weather-thunderstorm = Gewitter
weather-thunderstorm-hail-slight = Gewitter mit leichtem Hagel
weather-thunderstorm-hail-heavy = Gewitter mit starkem Hagel
weather-unknown = Unbekanntes Wetter

## Benachrichtigungs Modul
notifications-heading = Benachrichtigungen
notifications-empty = Keine Benachrichtigungen
notifications-group-count = { $count } neu

## Systeminformations Modul
system-info-heading = Systeminformationen
system-info-cpu-usage = CPU-Auslastung
system-info-memory-usage = RAM-Auslastung
system-info-swap-memory-usage = Swap-Nutzung
system-info-swap-indicator-prefix = Swap
system-info-temperature = Temperatur
system-info-disk-usage = Festplattennutzung { $mount }
system-info-ip-address = IP-Adresse
system-info-download-speed = Download
system-info-upload-speed = Upload
