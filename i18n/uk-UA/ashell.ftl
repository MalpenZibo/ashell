## Українська (uk-UA) версія. Нові ключі спершу з’являються в каталозі en-US;
## інші мови успадковують їх через узгодження Fluent і резервну мову (fallback).

app-name = ashell

## Модуль оновлень
updates-up-to-date = Усе актуально ;)
updates-available =
    { $count ->
        [one] { $count } оновлення доступне
        [few] { $count } оновлення доступні
       *[other] { $count } оновлень доступно
    }
updates-button-update = Оновити
updates-button-check-now = Перевірити зараз

## Модуль медіапрогравача
media-player-not-connected = Немає з’єднання зі службою MPRIS
media-player-heading = Програвачі
media-player-loading-cover = Завантаження обкладинки…
media-player-no-title = Без назви
media-player-unknown-artist = Невідомий виконавець
media-player-unknown-album = Невідомий альбом

## Діалог пароля / підключення до мережі
password-dialog-open-network-title = Відкрита мережа
password-dialog-authentication-required-title = Потрібна автентифікація
password-dialog-open-network-warning =
    Мережа «{ $ssid }» відкрита. Дані, що передаються цим з’єднанням, можуть бачити інші.
    Усе одно підключитися?
password-dialog-insert-password = Введіть пароль для підключення до: { $ssid }
password-dialog-connect-once = Підключитися лише цього разу
password-dialog-cancel = Скасувати
password-dialog-confirm = Підтвердити

## Діалог «Поділитися Wi-Fi»
share-wifi-title = Поділитися Wi-Fi
share-wifi-subtitle =
    Відскануйте цей QR-код іншим пристроєм,
    щоб підключитися до «{ $ssid }»
share-wifi-password-label = Пароль Wi-Fi:
share-wifi-password-open-network = Відкрита мережа: пароль не потрібен.
share-wifi-password-unavailable-message = Пароль цієї мережі неможливо прочитати, тому ним не можна поділитися.
share-wifi-unsupported-message = Спосіб автентифікації цієї мережі не можна передати через QR-код.
share-wifi-error-message = Не вдалося прочитати пароль Wi-Fi.
share-wifi-close = Закрити

## OSD
osd-airplane-toggle =
    { $state ->
        [on] Режим польоту увімкнено
       *[off] Режим польоту вимкнено
    }
osd-idle-inhibitor-toggle =
    { $state ->
        [on] Блокування присипляння увімкнено
       *[off] Блокування присипляння вимкнено
    }

## Налаштування: загальне
settings-scanning = Пошук…
settings-more = Більше

## Налаштування: мережа
settings-network-wifi = Wi-Fi
settings-network-vpn = VPN
settings-network-vpns-connected = Підключені VPN: { $count }
settings-network-airplane-mode = Режим польоту
settings-network-nearby-wifi = Wi-Fi поблизу

## Налаштування: Bluetooth
settings-bluetooth = Bluetooth
settings-bluetooth-devices = Пристрої Bluetooth
settings-bluetooth-known-devices = Пов’язані пристрої
settings-bluetooth-available = Доступні
settings-bluetooth-pair = Пов’язати
settings-bluetooth-no-devices = Пристроїв не знайдено
settings-bluetooth-connected-count =
    { $count ->
        [one] { $count } пристрій
        [few] { $count } пристрої
       *[other] { $count } пристроїв
    }

## Налаштування: живлення
settings-power-suspend = Призупинити
settings-power-hibernate = Приспати
settings-power-reboot = Перезавантажити
settings-power-shutdown = Вимкнути
settings-power-logout = Вийти
settings-power-calculating = Обчислення…
settings-power-full-in = Зарядиться через { $duration }
settings-power-empty-in = Розрядиться через { $duration }
settings-power-charge-limit = Обмеження заряджання
settings-power-profile-balanced = Збалансований
settings-power-profile-performance = Швидкодія
settings-power-profile-power-saver = Заощадження енергії

settings-power-status-charging = Заряджається
settings-power-status-discharging = Розряджається
settings-power-status-not-charging = Не заряджається
settings-power-status-unknown = Невідомо
settings-power-status-full = Повністю заряджено
settings-power-health = Стан батареї { $percentage }% ({ $energy_full } / { $energy_full_design } Вт·год)
settings-power-charge-cycles =
    { $count ->
        [one] { $count } цикл заряджання
        [few] { $count } цикли заряджання
       *[other] { $count } циклів заряджання
    }

## Налаштування: блокування присипляння
settings-idle-inhibitor = Блокування присипляння

## Налаштування: «Не турбувати»
settings-dnd = Не турбувати

## Налаштування: підказки
settings-tooltip-empty = Немає чого показати
settings-tooltip-empty-audio = Немає активного аудіопристрою
settings-tooltip-empty-bluetooth = Немає підключених пристроїв
settings-tooltip-empty-wifi = Не підключено
settings-tooltip-empty-vpn = Немає активного VPN
settings-tooltip-empty-battery = Немає даних про батарею

## Модуль погоди
tempo-feels-like = Відчувається як { $value }{ $unit }
tempo-humidity = Вологість
tempo-wind = Вітер

## Погодні умови (коди WMO Open-Meteo)
weather-clear-sky = Ясно
weather-mainly-clear = Переважно ясно
weather-partly-cloudy = Мінлива хмарність
weather-overcast = Суцільна хмарність
weather-fog = Туман
weather-fog-rime = Туман із памороззю
weather-drizzle-light = Слабка мряка
weather-drizzle-moderate = Помірна мряка
weather-drizzle-dense = Густа мряка
weather-drizzle-freezing-light = Слабка крижана мряка
weather-drizzle-freezing-dense = Густа крижана мряка
weather-rain-slight = Слабкий дощ
weather-rain-moderate = Помірний дощ
weather-rain-heavy = Сильний дощ
weather-rain-freezing-light = Слабкий крижаний дощ
weather-rain-freezing-heavy = Сильний крижаний дощ
weather-snow-slight = Слабкий сніг
weather-snow-moderate = Помірний сніг
weather-snow-heavy = Сильний снігопад
weather-snow-grains = Снігова крупа
weather-rain-showers-slight = Слабкий короткочасний дощ
weather-rain-showers-moderate = Помірний короткочасний дощ
weather-rain-showers-violent = Сильна злива
weather-snow-showers-slight = Слабкий короткочасний сніг
weather-snow-showers-heavy = Сильний короткочасний сніг
weather-thunderstorm = Слабка або помірна гроза
weather-thunderstorm-hail-slight = Гроза з дрібним градом
weather-thunderstorm-hail-heavy = Гроза з сильним градом
weather-unknown = Невідомі погодні умови

## Модуль сповіщень
notifications-heading = Сповіщення
notifications-empty = Немає сповіщень
notifications-group-count =
    { $count ->
        [one] { $count } нове
        [few] { $count } нові
       *[other] { $count } нових
    }

## Модуль інформації про систему
system-info-heading = Інформація про систему
system-info-cpu-usage = Використання процесора
system-info-memory-usage = Використання пам’яті
system-info-swap-memory-usage = Використання пам’яті підкачки
system-info-swap-indicator-prefix = swap
system-info-temperature = Температура
system-info-disk-usage = Використання диска { $mount }
system-info-ip-address = IP-адреса
system-info-download-speed = Швидкість отримання
system-info-upload-speed = Швидкість вивантаження
