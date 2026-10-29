# Гайд: как строить пакет на Luna

Этот документ описывает правильную модель разработки Luna-пакета.

Luna — не библиотека, которую пакет встраивает в себя. Это runtime и gateway,
уже работающий на станции. Пакет загружается в Luna, получает Lua API и
общается с железом только через этот API.

```text
пакет → Luna API → capability station crate → железо
```

Благодаря этому один и тот же Lua-код может работать на Mini, Mini 3, Midi,
Micro и других станциях. Различия LED, кнопок, часов и драйверов остаются
внутри station crate.

## 1. Создайте структуру пакета

Рекомендуемая структура исходников:

```text
my-plugin/
├── yam.yaml
└── plugin/
    ├── plugin.toml
    └── main.lua
```

Для пакета `vasya/clock` Yam создаст каталог Luna:

```text
share/luna/plugins/vasya.clock/
├── plugin.toml
└── main.lua
```

Точка в каталоге — это только формат идентификатора Luna-плагина. Имя пакета
Yam остаётся в формате `namespace/name`.

## 2. Опишите пакет Yam

`yam.yaml`:

```yaml
name: vasya/clock
version: 1.0.0
license: MIT
description: Показывает состояние пакета на LED
targets: [any]
depends: [yam/luna ^0.2]
files:
  share/luna/plugins/vasya.clock/: plugin/
```

`targets: [any]` подходит для чистого Lua-пакета: в нём нет native-бинарника,
поэтому код не зависит от CPU и libc станции.

`depends` поддерживается актуальным Yam и означает, что на станции должна быть
установлена совместимая версия `yam/luna`. Ограничение `^0.2` допускает версии
`0.2.x`, но не `0.3.0`. Сам Lua-код не импортирует Luna и не содержит
отдельного runtime.

Актуальные правила упаковки Yam: [YAM PACKAGING.md](https://github.com/nyakokitsu/yam/blob/main/docs/PACKAGING.md).

## 3. Опишите manifest Luna

`plugin/plugin.toml`:

```toml
id = "vasya.clock"
name = "Luna Clock"
version = "1.0.0"
entry = "main.lua"
description = "Пример Lua-пакета для Luna"
permissions = ["led", "clock"]

[compatibility]
features = ["led.ring.rgb"]
```

Обязательные поля:

| Поле | Назначение |
|---|---|
| `id` | Должен совпадать с каталогом `namespace.name` |
| `name` | Отображаемое имя |
| `version` | Версия самого плагина |
| `entry` | Lua-файл относительно каталога плагина |
| `permissions` | Явный список разрешённых API |
| `[compatibility]` | Требования к модели или capability станции |

Если capability нет, Luna покажет пакет как несовместимый. Не копируйте в
пакет sysfs-пути, I²C-адреса или названия устройств — это ломает переносимость.

## 4. Напишите Lua-код

Минимальный пример строится от capability: сначала проверяем, есть ли нужная
зона, и только потом вызываем её API.

```lua
if device.has_ring then
  alice.led.set.ring({"#00aaff"})
end

if device.has_logo then
  alice.led.set.logo({"#ffffff"})
end
```

Устройство описывается через `device`:

```lua
device.model
device.architecture
device.has_ring
device.ring_count
device.has_logo
device.logo_count
device.has_clock
device.has_mute_button
device.features
device.room
```

### Собственная веб-страница

Для HTML-интерфейса добавьте `web` в `permissions`, положите страницу внутрь
пакета и зарегистрируйте её из Lua:

```toml
permissions = ["web"]

[launch]
mode = "service"
```

```lua
local url = register.customweb("web/index.html")
print(url) -- /web/vasya.clock

while true do
  alice.sleep(60000)
end
```

Страница открывается как `/web/<plugin-id>` и может обращаться к Luna API через
`fetch('/api/...')`. Разрешены только `.html`/`.htm` из каталога пакета размером
до 4 МБ. Регистрация действует, пока запущен плагин.

Есть и совместимые PascalCase-имена: `device.HasRing`, `device.RingCount`,
`device.HasLogo`, `device.LogoCount`, `device.HasClock`,
`device.HasMuteButton`, `device.HasMicrophone`.

Наличие штатного микрофонного тракта проверяется через `device.has_microphone`.
На станциях без запущенного `maind` значение будет `false`.

Проверка `is_available()` нужна всегда, если capability может отсутствовать на
части станций:

```lua
if device.has_clock then
  alice.clock.time("12:34", 180)
end

if alice.input.is_available() then
  local event = alice.input.next_event(1000)
end
```

`is_available()` используется для capability-модулей без отдельной topology-
проверки. Для LED достаточно `device.has_ring` и `device.has_logo`. Permission
в manifest всё равно должен быть объявлен.

### LED

```lua
if device.has_logo then
  alice.led.set.logo({"#ff0000", "#0000ff"})
end

if device.has_ring then
  alice.led.set.ring({"#00ff00"})
end

if device.has_logo then
  alice.led.set.logo({"#ffffff"})
end

if device.has_logo then
  alice.led.off.logo()
end
```

Пакет может узнать физический размер зоны через `device.ring_count` и
`device.logo_count`, но ему не нужно вручную раскладывать цвета по индексам.
Переданный список — это компактная цветовая схема: Luna растягивает его на всю
зону. Например, два цвета на шести LED дадут три LED первого цвета и три LED
второго; если число LED не делится на число цветов, распределение остаётся
максимально равномерным.

Если цветов больше, чем LED в выбранной зоне, Luna не будет молча отбрасывать
лишние значения: API вернёт ошибку `400` с ожидаемым количеством цветов.
Пакет может либо ограничить список сам, либо показать эту ошибку пользователю.

### Собственные звуки

Положите PCM WAV-файл внутрь пакета и запросите permission `audio`:

```toml
permissions = ["audio"]
```

```lua
if alice.audio.is_available() then
  alice.audio.play("sounds/notification.wav")
end
```

На станциях Quasar Luna передаёт воспроизведение штатному сервису `mediad` — тому
же маршруту, которым пользуется система. Пакет не запускает собственный
`aplay`, не подменяет ALSA-библиотеки станции и не забирает аудиоустройство у
системы. Сейчас входной файл должен быть стерео PCM WAV: signed 16-bit,
48000 Hz, размер до 64 MiB и длительность до 600 секунд. MP3, Opus и другие
частоты пакет должен заранее преобразовать в такой WAV.

### Часы, input и сенсоры

```lua
if device.has_clock then
  alice.clock.time("12:34", 200)
end

if alice.input.is_available() then
  local event = alice.input.next_event(5000)
  if event and event.action == "volume_up" then
    -- реакция на кнопку +
  end
end

if alice.sensors.is_available() then
  local sensors = alice.sensors.list()
end
```

Для точного набора Lua-методов и HTTP-эквивалентов используйте
[docs/API.md](API.md).

## 5. Permissions

Запрашивайте только необходимые разрешения:

| Permission | Доступ |
|---|---|
| `led` | LED ring/logo и выключение |
| `clock` | сегментные часы |
| `input` | input events и semantic actions |
| `volume` | громкость |
| `audio` | проигрывание аудио |
| `microphone` | микрофон |
| `bluetooth` | статус, scan, pair/connect/disconnect/remove и доступное управление питанием Bluetooth |
| `display` | backlight/display |
| `sensors` | сенсоры |
| `network` | HTTP в рамках network policy |
| `ipc.read` | чтение разрешённых IPC-сервисов |
| `ipc.control` | управление разрешёнными IPC-сервисами |
| `commands` | голосовые команды из manifest |
| `system` | read-only информация о системе |
| `exec` | запуск команд; для binary execution нужен отдельный флаг Luna |

Для станций с аппаратным mute/unmute микрофона:

```lua
if microphone.waitformute(60000) then
  -- микрофон выключен аппаратной кнопкой
end

if microphone.waitforunmute(60000) then
  -- микрофон снова включён
end
```

Эти методы скрывают аппаратный источник сигнала конкретной станции.

Например, сетевой пакет должен явно описать список хостов:

```toml
permissions = ["network"]

[network]
allow = ["api.example.com"]
```

Отсутствующее permission должно считаться ошибкой разработки, а не поводом
просить пользователя включить всё сразу.

## 6. Соберите и установите

Упаковка:

```sh
yam-publish pack --source . --platform yandexmini3 \
  --profiles /path/to/yam/platforms \
  --output luna-clock-1.0.0.yampkg
```

Для чистого Lua-пакета используйте `targets: [any]` и опубликуйте пакет через
обычный registry Yam:

```sh
yam install vasya/clock
```

После установки Luna найдёт его в `share/luna/plugins`. Проверка:

```sh
curl http://<IP Станции>:8099/api/plugins
curl http://<IP Станции>:8099/api/plugins/sources
```

Плагин из Yam имеет `source: "yam"`. Его файлы принадлежат Yam, поэтому Luna
не удаляет их и не заменяет загрузкой `.yap`. Удаление выполняется так:

```sh
yam remove vasya/clock
```

## 7. Локальная разработка

Для быстрого теста можно собрать `.yap` и установить его через Luna API:

```sh
curl -X POST http://<IP Станции>:8099/api/plugins \
  -F 'package=@luna-clock-1.0.0.yap'
curl http://<IP Станции>:8099/api/plugins
```

Запуск и остановка выполняются через `/api/plugins/<id>/invoke` и
`/api/plugins/<id>/stop`. Для локального тестирования ограничьте слушатель:

```sh
luna --listen 127.0.0.1:8100 --data-dir /tmp/luna-test
```

## 8. Проверка на разных станциях

Минимальный чек-лист:

1. `GET /api/device` — проверить модель и `features`.
2. Проверить `is_available()` перед каждой необязательной capability.
3. Проверить `device.has_ring` и количество ring LED; logo является базовой зоной Luna.
4. Проверить отсутствие железных путей в исходниках пакета.
5. Запустить пакет на станциях с разной topology LED.
6. Проверить остановку и повторный запуск пакета.

Если пакет содержит native-код, это уже не переносимый Lua-пакет: для каждой
архитектуры нужен отдельный target и отдельный артефакт. По возможности
выносите аппаратную часть в Luna station crate, а пакет оставляйте Lua-only.

## 9. Типичные ошибки

### `permission denied`

Добавьте нужное permission в `plugin.toml`. Настройки Luna не могут выдать
пакету permission, которого нет в его manifest.

### `capability is unavailable`

Проверьте `GET /api/device`, `device.*` и `is_available()`. Не предполагайте,
что наличие capability на одной модели означает её наличие на всех.

### Плагин не запускается после установки Yam

Проверьте `GET /api/plugins/sources`, затем:

```sh
curl -X POST http://<IP Станции>:8099/api/plugins/rescan
```

Также проверьте, что каталог имеет формат `namespace.name`, а `id` в
`plugin.toml` совпадает с ним.
