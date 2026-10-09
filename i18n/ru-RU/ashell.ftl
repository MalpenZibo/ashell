## Русская (ru-RU) версия. Новые ключи сначала появляются в каталоге en-US;
## остальные языки наследуют их через согласование Fluent и резерв (fallback).

app-name = ashell

## Модуль обновлений
updates-up-to-date = Всё актуально ;)
updates-available =
    { $count ->
        [one] { $count } обновление доступно
       [few] { $count } обновления доступно
      *[other] { $count } обновлений доступно
    }
updates-button-update = Обновить
updates-button-check-now = Проверить сейчас

## Модуль медиаплеера
media-player-not-connected = Нет соединения с сервисом MPRIS
media-player-heading = Плееры
media-player-loading-cover = Загрузка обложки…
media-player-no-title = Без названия
media-player-unknown-artist = Неизвестный исполнитель
media-player-unknown-album = Неизвестный альбом

## Диалог пароля / подключения к сети
password-dialog-open-network-title = Открытая сеть
password-dialog-authentication-required-title = Требуется авторизация
password-dialog-open-network-warning =
    «{ $ssid }» — открытая сеть. Данные, передаваемые по этому соединению, могут быть видны другим.
    Подключиться всё равно?
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
        [on] Блокировка простоя включена
       *[off] Блокировка простоя выключена
    }

## Настройки — общие
settings-scanning = Поиск…
settings-more = Ещё

## Настройки — сеть
settings-network-wifi = Wi-Fi
settings-network-vpn = VPN
settings-network-vpns-connected =
    { $count ->
        [one] { $count } VPN подключён
       [few] { $count } VPN подключено
      *[other] { $count } VPN подключено
    }
settings-network-airplane-mode = Режим полёта
settings-network-nearby-wifi = Wi-Fi поблизости

## Настройки — Bluetooth
settings-bluetooth = Bluetooth
settings-bluetooth-devices = Bluetooth-устройства
settings-bluetooth-known-devices = Известные устройства
settings-bluetooth-available = Доступные
settings-bluetooth-pair = Спарить
settings-bluetooth-no-devices = Устройства не найдены
settings-bluetooth-connected-count =
    { $count ->
        [one] { $count } устройство
       [few] { $count } устройства
      *[other] { $count } устройств
    }

## Настройки — питание
settings-power-suspend = Сuspend
settings-power-hibernate = Спящий режим
settings-power-reboot = Перезагрузить
settings-power-shutdown = Выключить
settings-power-logout = Выйти
settings-power-calculating = Рассчитываем…
settings-power-full-in = Полная зарядка через { $duration }
settings-power-empty-in = Разрядится через { $duration }
settings-power-charge-limit = Лимит зарядки
settings-power-profile-balanced = Сбалансированный
settings-power-profile-performance = Производительность
settings-power-profile-power-saver = Экономия энергии
settings-power-status-charging = Заряжается
settings-power-status-discharging = Разряжается
settings-power-status-not-charging = Не заряжается
settings-power-status-unknown = Неизвестно
settings-power-status-full = Полная зарядка

## Настройки — блокировка простоя
settings-idle-inhibitor = Блокировка простоя

## Настройки — всплывающие подсказки
settings-tooltip-empty = Нечего показать
settings-tooltip-empty-audio = Нет активного аудиоустройства
settings-tooltip-empty-bluetooth = Нет подключённых устройств
settings-tooltip-empty-wifi = Нет подключения
settings-tooltip-empty-vpn = Нет активного VPN
settings-tooltip-empty-battery = Нет информации о батарее

## Модуль погоды
tempo-feels-like = Ощущается { $value }{ $unit }
tempo-humidity = Влажность
tempo-wind = Ветер

## Условие погоды (коды WMO Open-Meteo)
weather-clear-sky = Ясно
weather-mainly-clear = Преимущественно ясно
weather-partly-cloudy = Переменная облачность
weather-overcast = Облачно
weather-fog = Туман
weather-fog-rime = Изморозь
weather-drizzle-light = Лёгкая морось
weather-drizzle-moderate = Морось
weather-drizzle-dense = Густая морось
weather-drizzle-freezing-light = Лёгкая налётная морось
weather-drizzle-freezing-dense = Густая налётная морось
weather-rain-slight = Слабый дождь
weather-rain-moderate = Дождь
weather-rain-heavy = Сильный дождь
weather-rain-freezing-light = Лёгкий налётный дождь
weather-rain-freezing-heavy = Сильный налётный дождь
weather-snow-slight = Слабый снег
weather-snow-moderate = Снег
weather-snow-heavy = Сильный снег
weather-snow-grains = Снежная крупа
weather-rain-showers-slight = Слабый ливень
weather-rain-showers-moderate = Ливень
weather-rain-showers-violent = Сильный ливень
weather-snow-showers-slight = Слабый снежный ливень
weather-snow-showers-heavy = Сильный снежный ливень
weather-thunderstorm = Слабая или умеренная гроза
weather-thunderstorm-hail-slight = Гроза с лёгким градом
weather-thunderstorm-hail-heavy = Гроза с сильным градом
weather-unknown = Неизвестное погодное условие

## Модуль уведомлений
notifications-heading = Уведомления
notifications-empty = Нет уведомлений
notifications-group-count = { $count } новых

## Модуль информации о системе
system-info-heading = Информация о системе
system-info-cpu-usage = Загрузка CPU
system-info-memory-usage = Загрузка памяти
system-info-swap-memory-usage = Загрузка памяти swap
system-info-swap-indicator-prefix = swap
system-info-temperature = Температура
system-info-disk-usage = Использование диска { $mount }
system-info-ip-address = IP-адрес
system-info-download-speed = Скорость загрузки
system-info-upload-speed = Скорость отправки
