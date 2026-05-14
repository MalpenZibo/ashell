## Seed translation catalog. New keys land here first; other locales inherit
## via Fluent's negotiation + fallback.

app-name = ashell

## Updates module
updates-up-to-date = Обновлений нет ;)
updates-available =
    { $count ->
        [one] Доступно { $count } обновление
        [few] Доступно { $count } обновления
       *[other] Доступно { $count } обновлений
    }
updates-button-update = Обновить
updates-button-check-now = Проверить сейчас

## Media player module
media-player-not-connected = Не подключено к сервису MPRIS
media-player-heading = Плееры
media-player-loading-cover = Загрузка обложки...
media-player-no-title = Без названия
media-player-unknown-artist = Неизвестный исполнитель
media-player-unknown-album = Неизвестный альбом

## Password / network connection dialog
password-dialog-open-network-title = Открытая сеть
password-dialog-authentication-required-title = Требуется аутентификация
password-dialog-open-network-warning =
    Сеть «{ $ssid }» является открытой. Данные, передаваемые через неё, могут быть видны другим.
    Всё равно подключиться?
password-dialog-insert-password = Введите пароль для подключения к: { $ssid }
password-dialog-cancel = Отмена
password-dialog-confirm = Подтвердить

## OSD
osd-airplane-toggle =
    { $state ->
        [on] Режим полёта включён
       *[off] Режим полёта выключен
    }
osd-idle-inhibitor-toggle =
    { $state ->
        [on] Блокировка спящего режима включена
       *[off] Блокировка спящего режима выключена
    }

## Settings — shared
settings-scanning = Поиск...
settings-more = Ещё

## Settings — network
settings-network-wifi = Wi-Fi
settings-network-vpn = VPN
settings-network-vpns-connected =
    { $count ->
        [one] Подключён { $count } VPN
        [few] Подключено { $count } VPN
       *[other] Подключено { $count } VPN
    }
settings-network-airplane-mode = Режим полёта
settings-network-nearby-wifi = Доступные сети Wi-Fi

## Settings — bluetooth
settings-bluetooth = Bluetooth
settings-bluetooth-devices = Устройства Bluetooth
settings-bluetooth-known-devices = Известные устройства
settings-bluetooth-available = Доступно
settings-bluetooth-pair = Сопрячь
settings-bluetooth-no-devices = Устройства не найдены
settings-bluetooth-connected-count =
    { $count ->
        [one] { $count } устройство
        [few] { $count } устройства
       *[other] { $count } устройств
    }

## Settings — power
settings-power-suspend = Ждущий режим
settings-power-hibernate = Гибернация
settings-power-reboot = Перезагрузка
settings-power-shutdown = Выключение
settings-power-logout = Выйти из системы
settings-power-calculating = Рассчитывается...
settings-power-full-in = Заряжено через { $duration }
settings-power-empty-in = Разряжено через { $duration }
settings-power-profile-balanced = Сбалансированный
settings-power-profile-performance = Производительный
settings-power-profile-power-saver = Экономия энергии

## Settings — idle inhibitor
settings-idle-inhibitor = Блокировка спящего режима

## Tempo / weather module
tempo-feels-like = Ощущается как { $value }{ $unit }
tempo-humidity = Влажность
tempo-wind = Ветер

## Weather conditions (Open-Meteo WMO codes)
weather-clear-sky = Ясно
weather-mainly-clear = Преимущественно ясно
weather-partly-cloudy = Переменная облачность
weather-overcast = Пасмурно
weather-fog = Туман
weather-fog-rime = Переохлаждённый туман
weather-drizzle-light = Лёгкая морось
weather-drizzle-moderate = Умеренная морось
weather-drizzle-dense = Сильная морось
weather-drizzle-freezing-light = Лёгкая ледяная морось
weather-drizzle-freezing-dense = Сильная ледяная морось
weather-rain-slight = Небольшой дождь
weather-rain-moderate = Умеренный дождь
weather-rain-heavy = Сильный дождь
weather-rain-freezing-light = Небольшой ледяной дождь
weather-rain-freezing-heavy = Сильный ледяной дождь
weather-snow-slight = Небольшой снег
weather-snow-moderate = Умеренный снег
weather-snow-heavy = Сильный снег
weather-snow-grains = Снежная крупа
weather-rain-showers-slight = Небольшие ливни
weather-rain-showers-moderate = Умеренные ливни
weather-rain-showers-violent = Сильные ливни
weather-snow-showers-slight = Небольшой снегопад
weather-snow-showers-heavy = Сильный снегопад
weather-thunderstorm = Гроза (слабая или умеренная)
weather-thunderstorm-hail-slight = Гроза с небольшим градом
weather-thunderstorm-hail-heavy = Гроза с крупным градом
weather-unknown = Погодные условия не определены

## Notifications module
notifications-heading = Уведомления
notifications-empty = Нет уведомлений
notifications-group-count = { $count } новых

## Clipboard module
clipboard-heading = История буфера обмена
clipboard-empty = Буфер обмена пуст
clipboard-loading = Загрузка буфера обмена...
clipboard-image-entry = [Изображение]
