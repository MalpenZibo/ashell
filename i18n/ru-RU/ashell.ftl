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
password-dialog-authentication-required-title = Требуется аутентификация
password-dialog-open-network-warning =
    «{ $ssid }» — открытая сеть. Данные, передаваемые по этому соединению, могут быть видны другим.
    Подключиться всё равно?
password-dialog-insert-password = Введите пароль для подключения к: { $ssid }
password-dialog-connect-once = Подключиться только в этот раз
password-dialog-cancel = Отмена
password-dialog-confirm = Подтвердить

## Диалог «Поделиться Wi-Fi»
share-wifi-title = Поделиться Wi-Fi
share-wifi-subtitle =
    Отсканируйте этот QR-код другим устройством,
    чтобы подключиться к «{ $ssid }»
share-wifi-password-label = Пароль Wi-Fi:
share-wifi-password-open-network = Открытая сеть — пароль не требуется.
share-wifi-password-unavailable-message = Пароль этой сети нельзя прочитать, поэтому поделиться им невозможно.
share-wifi-unsupported-message = Метод аутентификации этой сети нельзя передать через QR-код.
share-wifi-error-message = Не удалось прочитать пароль Wi-Fi.
share-wifi-close = Закрыть

## OSD
osd-airplane-toggle =
    { $state ->
        [on] Режим полёта включён
       *[off] Режим полёта выключен
    }
osd-idle-inhibitor-toggle =
    { $state ->
        [on] Запрет сна включён
       *[off] Запрет сна выключен
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
settings-bluetooth-known-devices = Сопряжённые устройства
settings-bluetooth-available = Доступные
settings-bluetooth-pair = Сопряжение
settings-bluetooth-no-devices = Устройства не найдены
settings-bluetooth-connected-count =
    { $count ->
        [one] { $count } устройство
        [few] { $count } устройства
       *[other] { $count } устройств
    }

## Настройки — питание
settings-power-suspend = Спящий режим
settings-power-hibernate = Гибернация
settings-power-reboot = Перезагрузить
settings-power-shutdown = Выключить
settings-power-logout = Выйти
settings-power-calculating = Вычисление…
settings-power-full-in = Зарядится через { $duration }
settings-power-empty-in = Разрядится через { $duration }
settings-power-charge-limit = Ограничение заряда
settings-power-profile-balanced = Сбалансированный
settings-power-profile-performance = Производительность
settings-power-profile-power-saver = Экономия энергии
settings-power-status-charging = Заряжается
settings-power-status-discharging = Разряжается
settings-power-status-not-charging = Не заряжается
settings-power-status-unknown = Неизвестно
settings-power-status-full = Полностью заряжена
settings-power-health = Состояние { $percentage }% ({ $energy_full } / { $energy_full_design } Вт·ч)
settings-power-charge-cycles =
    { $count ->
        [one] { $count } цикл зарядки
        [few] { $count } цикла зарядки
       *[other] { $count } циклов зарядки
    }

## Настройки — запрет сна
settings-idle-inhibitor = Запрет сна

## Настройки — «Не беспокоить»
settings-dnd = Не беспокоить

## Настройки — всплывающие подсказки
settings-tooltip-empty = Нечего показать
settings-tooltip-empty-audio = Нет активного аудиоустройства
settings-tooltip-empty-bluetooth = Нет подключённых устройств
settings-tooltip-empty-wifi = Нет подключения
settings-tooltip-empty-vpn = Нет активного VPN
settings-tooltip-empty-battery = Нет информации о батарее

## Модуль погоды
tempo-feels-like = Ощущается как { $value }{ $unit }
tempo-humidity = Влажность
tempo-wind = Ветер

## Условие погоды (коды WMO Open-Meteo)
weather-clear-sky = Ясно
weather-mainly-clear = Малооблачно
weather-partly-cloudy = Переменная облачность
weather-overcast = Пасмурно
weather-fog = Туман
weather-fog-rime = Туман с изморозью
weather-drizzle-light = Небольшая морось
weather-drizzle-moderate = Морось
weather-drizzle-dense = Сильная морось
weather-drizzle-freezing-light = Небольшая ледяная морось
weather-drizzle-freezing-dense = Сильная ледяная морось
weather-rain-slight = Небольшой дождь
weather-rain-moderate = Дождь
weather-rain-heavy = Сильный дождь
weather-rain-freezing-light = Небольшой ледяной дождь
weather-rain-freezing-heavy = Сильный ледяной дождь
weather-snow-slight = Небольшой снег
weather-snow-moderate = Снег
weather-snow-heavy = Сильный снег
weather-snow-grains = Снежная крупа
weather-rain-showers-slight = Небольшой ливень
weather-rain-showers-moderate = Ливень
weather-rain-showers-violent = Сильный ливень
weather-snow-showers-slight = Небольшой снегопад
weather-snow-showers-heavy = Сильный снегопад
weather-thunderstorm = Слабая или умеренная гроза
weather-thunderstorm-hail-slight = Гроза с небольшим градом
weather-thunderstorm-hail-heavy = Гроза с сильным градом
weather-unknown = Неизвестные погодные условия

## Модуль уведомлений
notifications-heading = Уведомления
notifications-empty = Нет уведомлений
notifications-group-count =
    { $count ->
        [one] { $count } новое
       *[other] { $count } новых
    }

## Модуль информации о системе
system-info-heading = Информация о системе
system-info-cpu-usage = Использование процессора
system-info-memory-usage = Использование памяти
system-info-swap-memory-usage = Использование области подкачки
system-info-swap-indicator-prefix = swap
system-info-temperature = Температура
system-info-disk-usage = Использование диска { $mount }
system-info-ip-address = IP-адрес
system-info-download-speed = Скорость загрузки
system-info-upload-speed = Скорость отправки
