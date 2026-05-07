<h1 align="center">
  <a href="https://malpenzibo.github.io/ashell/">
    <img src="https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/logo_header_dark.svg" alt="ashell" height="140"/>
  </a>
</h1>
<p align="center">Готовая к использованию статус-бар для Wayland для Hyprland и Niri.</p>
<p align="center">
    <a href="https://matrix.to/#/#ashell:matrix.org"><img alt="Матрица" src="https://img.shields.io/badge/matrix-%23ashell-blue?logo=matrix"></a>
    <a href="https://github.com/MalpenZibo/ashell/blob/main/LICENSE"><img alt="Лицензия GitHub" src="https://img.shields.io/github/license/MalpenZibo/ashell"></a>
    <a href="https://github.com/MalpenZibo/ashell/releases"><img alt="Релиз GitHub" src="https://img.shields.io/github/v/release/MalpenZibo/ashell?logo=github"></a>
</p>

<p align="center">
    <a href="https://malpenzibo.github.io/ashell/docs/intro">Начало работы</a> | <a href="https://malpenzibo.github.io/ashell/docs/configuration">Конфигурация</a> | <a href="https://malpenzibo.github.io/ashell/dev-guide/">Руководство разработчика</a>
</p>

## ✨ Возможности

- Автоматическое определение композитора (Hyprland / Niri)
- Поддержка нескольких мониторов (все мониторы, активный монитор или выбранные)
- Горячая перезагрузка конфигурации (изменения применяются автоматически через отслеживание файлов)
- Позиционирование панели (сверху или снизу) с настраиваемым слоем (Bottom, Top, Overlay)
- Темы: стили Islands, Solid и Gradient с настраиваемыми цветами, прозрачностью, масштабом и шрифтами
- Индикатор обновлений ОС с настраиваемым интервалом проверки
- Активное окно (заголовок, класс или начальный заголовок/класс) для Hyprland / Niri
- Рабочие пространства Hyprland / Niri с заданием имён, цветовым кодированием и видимостью для каждого монитора
- Системная информация (ЦП, ОЗУ, диск, IP-адрес, скорость сети, температура) с порогами предупреждений и тревог
- Раскладка клавиатуры Hyprland / Niri с настраиваемыми метками
- Подслой клавиатуры Hyprland (Keyboard Submap)
- Системный трей с контекстными меню
- Часы с календарём, погодой, циклическим переключением часовых поясов и форматов (Tempo)
- Индикаторы приватности (использование микрофона, камеры и демонстрации экрана)
- Медиаплеер с обложкой альбома и информацией о треке
- Менеджер уведомлений с всплывающими тостами, группировкой и поддержкой уровней важности
- Панель настроек
  - Меню питания (выключение, сон, гибернация, перезагрузка, выход, блокировка)
  - Информация о батарее и заряде периферийных устройств
  - Источники и приёмники звука (включая микрофон)
  - Яркость экрана
  - Сеть (сканирование Wi-Fi, ввод пароля; поддержка бэкендов NetworkManager и IWD)
  - VPN
  - Bluetooth
  - Профили питания
  - Запрет бездействия
  - Режим полёта
  - Настраиваемые кнопки быстрых действий с командами статуса
- Сокет IPC для скриптов и горячих клавиш (`ashell msg <команда>`)
- Оверлей OSD для изменения громкости, яркости и режима полёта
- Пользовательские модули
  - Кнопка (выполнение команды по клику)
  - Текст (только отображение; обновление интерфейса через вывод команды, задаваемой `listen_cmd`)
  - Сопоставление иконок на основе регулярных выражений и состояния тревоги

## 🛠️ Установка

[![Статус упаковки](https://repology.org/badge/vertical-allrepos/ashell.svg)](https://repology.org/project/ashell/versions)

Подробную информацию смотрите на странице [Установки](https://malpenzibo.github.io/ashell/docs/installation).

## ⚙️ Конфигурация

ashell поставляется с конфигурацией по умолчанию, которая должна работать «из коробки».

Если вы хотите настроить её под себя, обратитесь к странице [Конфигурации](https://malpenzibo.github.io/ashell/docs/configuration).

## 💬 Сообщество

Присоединяйтесь к обсуждению в [Matrix](https://matrix.to/#/#ashell:matrix.org) или создайте [запрос](https://github.com/MalpenZibo/ashell/issues) на GitHub.

## 📖 Руководство разработчика

Если вы хотите внести вклад или разобраться в кодовой базе, обратитесь к [Руководству разработчика](https://malpenzibo.github.io/ashell/dev-guide/).

## 🤖 Правки, созданные с помощью ИИ

Правки, созданные с использованием ИИ, принимаются — к ним применяются те же стандарты качества, независимо от способа написания кода. Настоятельно рекомендуется использовать модели передового уровня (например, Claude Opus или аналоги). **Вы несёте ответственность за отправляемый код**: тщательно проверяйте вывод ИИ, убедитесь, что `make check` проходит успешно, и будьте готовы объяснить свои изменения.

Прежде чем работать над функцией или крупным изменением, **сначала обсудите это с мейнтейнерами**. Предпочтительны небольшие, инкрементальные PR-ы — проверка кода выполняется вручную и остаётся узким местом.

Полное руководство по участию с помощью ИИ смотрите в [Руководстве разработчика](https://malpenzibo.github.io/ashell/dev-guide/contributing/ai-assisted-contributions.html).

## 📷 Скриншоты

Я постараюсь поддерживать эти скриншоты в актуальном состоянии, но некоторые детали могут отличаться.

#### стиль по умолчанию

<img src="https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/ashell.png"></img>

#### стиль Solid

<img src="https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/ashell-solid.png"></img>

#### стиль Gradient

<img src="https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/ashell-gradient.png"></img>
### История буфера обмена
<img width="433" height="569" alt="image" src="https://github.com/user-attachments/assets/77bbf77d-6791-4147-825c-1469a7869c9e" />
#### настройки непрозрачности

<img src="https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/opacity.png"></img>

| ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/updates-panel.png)   | ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/system-menu.png)  |
| ------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------- |
| ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/tray-menu.png)       | ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/power-menu.png)   |
| ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/sinks-selection.png) | ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/network-menu.png) |
| ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/bluetooth-menu.png)  | ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/vpn-menu.png)     |
