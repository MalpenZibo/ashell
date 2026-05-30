## Traduction française. Les nouvelles clés sont d'abord ajoutées au catalogue
## en-US ; les autres locales héritent via la négociation Fluent et le repli.

app-name = ashell

## Module mises à jour
updates-up-to-date = À jour ;)
updates-available =
    { $count ->
        [one] { $count } mise à jour disponible
       *[other] { $count } mises à jour disponibles
    }
updates-button-update = Mettre à jour
updates-button-check-now = Vérifier maintenant

## Module lecteur multimédia
media-player-not-connected = Non connecté au service MPRIS
media-player-heading = Lecteurs
media-player-loading-cover = Chargement de la pochette…
media-player-no-title = Sans titre
media-player-unknown-artist = Artiste inconnu
media-player-unknown-album = Album inconnu

## Boîte de dialogue mot de passe / connexion réseau
password-dialog-open-network-title = Réseau ouvert
password-dialog-authentication-required-title = Authentification requise
password-dialog-open-network-warning =
    « { $ssid } » est un réseau ouvert. Les données envoyées via cette connexion peuvent être visibles par d'autres.
    Voulez-vous tout de même vous connecter ?
password-dialog-insert-password = Saisissez le mot de passe pour vous connecter à : { $ssid }
password-dialog-cancel = Annuler
password-dialog-confirm = Confirmer

## OSD
osd-airplane-toggle =
    { $state ->
        [on] Mode avion activé
       *[off] Mode avion désactivé
    }
osd-idle-inhibitor-toggle =
    { $state ->
        [on] Inhibiteur de veille activé
       *[off] Inhibiteur de veille désactivé
    }

## Paramètres — communs
settings-scanning = Recherche en cours…
settings-more = Plus

## Paramètres — réseau
settings-network-wifi = Wi-Fi
settings-network-vpn = VPN
settings-network-vpns-connected =
    { $count ->
        [one] { $count } VPN connecté
       *[other] { $count } VPN connectés
    }
settings-network-airplane-mode = Mode avion
settings-network-nearby-wifi = Wi-Fi à proximité

## Paramètres — Bluetooth
settings-bluetooth = Bluetooth
settings-bluetooth-devices = Appareils Bluetooth
settings-bluetooth-known-devices = Appareils connus
settings-bluetooth-available = Disponibles
settings-bluetooth-pair = Appairer
settings-bluetooth-no-devices = Aucun appareil trouvé
settings-bluetooth-connected-count =
    { $count ->
        [one] { $count } appareil
       *[other] { $count } appareils
    }

## Paramètres — alimentation
settings-power-suspend = Mettre en veille
settings-power-hibernate = Hiberner
settings-power-reboot = Redémarrer
settings-power-shutdown = Éteindre
settings-power-logout = Déconnexion
settings-power-calculating = Calcul en cours…
settings-power-full-in = Pleine dans { $duration }
settings-power-empty-in = Vide dans { $duration }
settings-power-charge-limit = Limite de charge
settings-power-profile-balanced = Équilibré
settings-power-profile-performance = Performance
settings-power-profile-power-saver = Économie d'énergie

## Paramètres — inhibiteur de veille
settings-idle-inhibitor = Inhibiteur de veille

## Module météo
tempo-feels-like = Ressenti { $value }{ $unit }
tempo-humidity = Humidité
tempo-wind = Vent

## Conditions météo (codes WMO Open-Meteo)
weather-clear-sky = Ciel dégagé
weather-mainly-clear = Plutôt dégagé
weather-partly-cloudy = Partiellement nuageux
weather-overcast = Couvert
weather-fog = Brouillard
weather-fog-rime = Brouillard givrant
weather-drizzle-light = Bruine légère
weather-drizzle-moderate = Bruine modérée
weather-drizzle-dense = Bruine dense
weather-drizzle-freezing-light = Bruine verglaçante légère
weather-drizzle-freezing-dense = Bruine verglaçante dense
weather-rain-slight = Pluie faible
weather-rain-moderate = Pluie modérée
weather-rain-heavy = Pluie forte
weather-rain-freezing-light = Pluie verglaçante légère
weather-rain-freezing-heavy = Pluie verglaçante forte
weather-snow-slight = Chute de neige faible
weather-snow-moderate = Chute de neige modérée
weather-snow-heavy = Chute de neige forte
weather-snow-grains = Grésil
weather-rain-showers-slight = Averses de pluie faibles
weather-rain-showers-moderate = Averses de pluie modérées
weather-rain-showers-violent = Averses de pluie violentes
weather-snow-showers-slight = Averses de neige faibles
weather-snow-showers-heavy = Averses de neige fortes
weather-thunderstorm = Orage faible ou modéré
weather-thunderstorm-hail-slight = Orage avec grêle légère
weather-thunderstorm-hail-heavy = Orage avec grêle forte
weather-unknown = Condition météo inconnue

## Module notifications
notifications-heading = Notifications
notifications-empty = Aucune notification
notifications-group-count = { $count } nouvelle(s)

## Module informations système
system-info-heading = Infos système
system-info-cpu-usage = Utilisation CPU
system-info-memory-usage = Utilisation mémoire
system-info-swap-memory-usage = Utilisation mémoire swap
system-info-swap-indicator-prefix = swap
system-info-temperature = Température
system-info-disk-usage = Utilisation disque { $mount }
system-info-ip-address = Adresse IP
system-info-download-speed = Débit descendant
system-info-upload-speed = Débit montant
