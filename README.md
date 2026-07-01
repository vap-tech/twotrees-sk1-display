# twotrees-sk1-display

Материалы по UART-дисплею TwoTrees SK1 и альтернативному клиенту для работы с
Moonraker/Klipper после перехода на ванильный Armbian.

## Важно

Дисплей питается от `5V`, но уровни `RX/TX` - `3.3V`.

Не подключайте UART дисплея к `5V TTL`. Общую землю (`GND`) подключать обязательно.

## Что уже работает

- Запуск Rust-клиента `vaptechclient` с async UART service для штатного HMI-дисплея.
- Разбор входящих HMI-событий: startup `0x91`, touch, numeric input для ползунов.
- Навигация по базовым экранам: Home, Print, Files, Fans, Settings/Network/Calibration.
- Moonraker WebSocket subscribe и reducer состояния принтера.
- Read-only WebSocket watchdog через `server.info`: если поток событий замолчал,
  клиент проверяет соединение и сам уходит в reconnect.
- Live-отрисовка температур сопла/стола, target values, прогресса печати и времени.
- Автовосстановление экрана печати после init дисплея, если печать уже идёт.
- Орка/G-code thumbnail pipeline: download через Moonraker, extract, TJC encode,
  cache, асинхронная доставка на page 2 и page 77.
- Проверка актуальности thumbnail через `RenderTarget`, чтобы не залить старый
  эскиз в уже сменившийся экран или слот файла.
- Иконки подсветки и вентиляторов на странице печати обновляются от фактического
  состояния Moonraker.
- Управление подсветкой через `SET_PIN PIN=caselight`.
- Управление тремя вентиляторами с HMI page 6: model/part fan, auxiliary/side fan,
  case/filter fan.
- Pause/resume печати с page 2 component 5: при печати отправляет pause, на паузе
  отправляет resume.
- Настраиваемый info-лог для touch/numeric HMI-событий без сырого UART hex.
- Unit/integration tests покрывают parser, reducer, renderer, runtime, thumbnail
  pipeline и print pipeline.

## Что есть в репозитории

- `tools/display_uart_demo_host` - Python-демо/песочница для дисплея.
- `display_uart_raw_protocol.md` - список известных UART-команд и входящих кадров.
- `vaptechclient/` - Rust-клиент для HMI-дисплея и Moonraker.
- `esp32-tft/` - экспериментальный клиент под ESP32-C6.


## vaptechclient

`vaptechclient` - текущий Rust-клиент. Он запускает:

- async UART service для дисплея;
- Moonraker WebSocket client;
- центральный runtime;
- reducer состояния принтера;
- renderer команд HMI;
- thumbnail pipeline для Orca/G-code preview.

Moonraker write-path включён точечно. Сейчас из UI обратно в принтер отправляются
только явно разрешённые runtime команды: подсветка, вентиляторы, target
температуры, pause/resume. Остальные `MoonrakerRequest` намеренно отбрасываются,
чтобы новые кнопки не начали внезапно управлять принтером без явного подключения.

### Архитектура

Главный принцип:

```text
Intent
↓
Request
↓
Confirmation
↓
State
↓
Render
```

UI не делает optimistic update. Например, нажатие подсветки не меняет
`AppState.lights.case_light` напрямую. Оно создаёт `MoonrakerRequest`, Moonraker
выполняет команду, затем WebSocket присылает фактический статус, reducer обновляет
`AppState`, и только после этого renderer меняет иконку.

Основные слои:

- `src/hmi/` - протокол HMI: команды, события, frame buffer, parser, serial service.
- `src/moonraker/` - WebSocket client, parser Moonraker JSON, события принтера.
- `src/app/state.rs` - единое состояние приложения.
- `src/app/reducers/` - применение внешних событий к `AppState`.
- `src/app/runner.rs` - application core: принимает `AppEvent`, меняет состояние и складывает outputs.
- `src/runtime.rs` - async glue: доставляет события в runner и отправляет HMI/Moonraker/thumbnail effects.
- `src/ui/route.rs` - таблица `page/component -> UiIntent`.
- `src/ui/intent.rs` - семантические намерения пользователя.
- `src/ui/action_handler.rs` - guards и преобразование intent в изменения HMI state / Moonraker requests.
- `src/ui/render_target.rs` - решает, что сейчас нужно показать на дисплее.
- `src/ui/render_full.rs` - полная отрисовка выбранного `RenderTarget`.
- `src/ui/render_diff.rs` - минимальная отрисовка изменений между old/new `AppState`.
- `src/ui/components.rs` - vendor mapping: физические компоненты HMI (`b5`, `b6`, `pic 2/3`).
- `src/thumbnail/` - извлечение preview из G-code, TJC encoding, cache, worker.

### Состояния

Состояние HMI и состояние принтера разделены:

```text
HmiState
  current_screen
  selected move/file/etc

PrinterState / PrintState / TemperatureState / FanState / LightState
  приходят из Moonraker/reducer
```

Moonraker reducer не должен менять выбранный пользователем экран. Он обновляет
только состояние принтера. Выбор визуального представления делается отдельно:

```text
AppState
↓
resolve_render_target()
↓
RenderTarget
↓
render_full/render_diff
```

Пример:

```text
HmiState.current_screen = Home
PrinterStatus = Printing
=> RenderTarget::Home(HomeMode::Printing)
=> page 2
```

Если пользователь ушёл в `Files` во время печати:

```text
HmiState.current_screen = Files
PrinterStatus = Printing
=> RenderTarget::Files
```

То есть активная печать не крадёт экран пользователя.

### Поток HMI touch

```text
UART bytes
↓
FrameBuffer
↓
parse_frame()
↓
HmiEvent::Touch { page, component }
↓
route_touch(page, component)
↓
UiIntent
↓
intent_is_blocked_by_printer_state()
↓
apply_hmi_intent()
↓
moonraker_requests_for_intent()
↓
render_diff(old_state, new_state)
```

`route.rs` не знает про `AppState`. Это простая таблица известных кнопок.

`action_handler.rs` знает про состояние и решает:

- можно ли выполнить intent в текущем состоянии принтера;
- нужно ли поменять `HmiState`;
- нужно ли создать `MoonrakerRequest`.

### Поток Moonraker

```text
WebSocket message
↓
parse_moonraker_message()
↓
Vec<MoonrakerEvent>
↓
AppEvent::Moonraker(...)
↓
reduce_moonraker_event()
↓
AppState
↓
render_diff(old_state, new_state)
```

Подписка WebSocket сейчас включает:

- `print_stats`
- `virtual_sdcard`
- `extruder`
- `heater_bed`
- `toolhead`
- `output_pin caselight`
- `fan`
- `fan_generic Side_fan`
- `fan_generic Filter_fan`

Moonraker client отслеживает время последнего входящего WebSocket frame. Если
поток молчит дольше `10s`, клиент отправляет read-only heartbeat `server.info` и
ждёт ответ до `2s`. Если ответа нет или запись/чтение падает, текущий WebSocket
закрывается через ошибку, а внешний loop выполняет reconnect. Heartbeat не
использует write-path принтера и не меняет состояние Klipper.

### Vendor mapping

Физические имена компонентов HMI не должны расползаться по проекту.

Правильно:

```rust
render_case_light_icon(target, state.lights.case_light)
```

Неправильно:

```rust
HmiCommand::raw("b6.picc=3")
```

Компоненты и номера картинок держим в `src/ui/components.rs`. Например:

```text
CaseLightIcon:
  Home  -> b5
  Print -> b6
  off   -> pic 2
  on    -> pic 3
```

### Установка на принтер

Rust toolchain на принтер ставить не нужно. Нормальный путь сейчас такой:

```text
ноут/ПК
  cargo build --release --target aarch64-unknown-linux-musl
      ↓
принтер
  /usr/local/bin/vaptechclient
  /etc/vaptechclient/config.toml
  /etc/systemd/system/vaptechclient.service
```

Готовые шаблоны лежат в `vaptechclient/packaging/`.

Собрать пакетный набор:

```bash
cd vaptechclient
./packaging/build-release.sh
```

Положить на принтер и перезапустить service:

```bash
cd vaptechclient
./packaging/deploy.sh 192.168.0.20
```

Первым делом после установки проверьте на принтере:

```bash
sudo nano /etc/vaptechclient/config.toml
sudo systemctl status vaptechclient
sudo journalctl -u vaptechclient -f
```

В конфиге особенно важен путь к UART дисплея:

```toml
[hmi]
serial = "/dev/ttyS1"
baud = 115200
```

Если клиент запускается прямо на принтере, Moonraker обычно указывается как
`127.0.0.1:7125`.

### Как добавить touch-кнопку

1. Добавить семантический intent в `src/ui/intent.rs`.

```rust
pub enum UiIntent {
    ToggleCaseLight,
    // ...
}
```

2. Привязать HMI page/component в `src/ui/route.rs`.

```rust
(2, 6) => UiIntent::ToggleCaseLight,
```

3. Если действие опасно во время печати, добавить guard в
   `intent_is_blocked_by_printer_state()`.

4. Если intent меняет только интерфейс, обработать его в `apply_hmi_intent()`.

5. Если intent должен управлять принтером, вернуть `MoonrakerRequest` из
   `moonraker_requests_for_intent()`.

6. Добавить unit tests:

- route test: `page/component -> UiIntent`;
- action handler test: intent создаёт правильный request или меняет HMI state;
- runner test: touch проходит весь путь до outputs.

### Как добавить Moonraker event

1. Добавить событие в `src/moonraker/event.rs`.

```rust
pub enum MoonrakerEvent {
    CaseLightChanged(bool),
    // ...
}
```

2. Распарсить JSON в `src/moonraker/parser.rs`.

```rust
events.push(MoonrakerEvent::CaseLightChanged(value > 0.5));
```

3. Если нужен новый object, добавить его в подписку
   `objects_subscribe_message()` в `src/moonraker/client.rs`.

4. Обновить reducer в `src/app/reducers/moonraker.rs`.

```rust
MoonrakerEvent::CaseLightChanged(enabled) => {
    state.lights.case_light = enabled;
}
```

5. Добавить render diff/full, если изменение должно быть видно на экране.

6. Добавить tests:

- parser test с реальным JSON-фрагментом;
- reducer test;
- render_diff test, если событие меняет визуальное состояние.

### Как добавить Moonraker request

1. Добавить request в `src/ui/effect.rs`.

```rust
pub enum MoonrakerRequest {
    SetCaseLight(bool),
    // ...
}
```

2. Создать request из intent в `moonraker_requests_for_intent()`.

3. В `src/runtime.rs` явно разрешить пересылку этого request. По умолчанию
   runtime не должен отправлять новые управляющие команды в Moonraker.

4. В `src/moonraker/client.rs` сериализовать request в JSON-RPC или G-code script.

```json
{
  "jsonrpc": "2.0",
  "method": "printer.gcode.script",
  "params": {
    "script": "SET_PIN PIN=caselight VALUE=1"
  },
  "id": 42
}
```

5. Добавить `info`-лог перед отправкой. Это сильно упрощает проверку на живом
   принтере:

```text
sending caselight command to Moonraker enabled=true
```

6. Не менять `AppState` из request path. Дождаться подтверждения через
   `MoonrakerEvent`.

7. Добавить tests:

- action handler создаёт request;
- runtime пересылает только разрешённый request;
- Moonraker client генерирует корректный JSON.

### Как добавить отображаемый widget

1. Добавить семантический helper в `src/ui/components.rs`.

```rust
pub fn render_case_light_icon(target: RenderTarget, enabled: bool) -> Vec<HmiCommand>
```

2. Внутри helper держать mapping `RenderTarget -> component`.

3. Подключить helper в `render_full`.

4. Подключить helper в `render_diff`, если состояние может меняться без смены
   страницы.

5. Добавить tests на mapping и render output.

### Thumbnail pipeline

Thumbnail не рендерится синхронно в UI:

```text
render_full()
↓
RenderTarget::thumbnail_request(...)
↓
ThumbnailRequest
↓
worker
↓
cache
↓
ThumbnailReady
↓
current RenderTarget::accepts_thumbnail(...)
↓
HmiCommand cp.write(...)
```

Для страницы печати и страницы результата используется один pipeline, но разные
`ThumbnailTarget`.

Правило создания и доставки thumbnail намеренно лежит рядом с visual target:

- `RenderTarget::thumbnail_request(&AppState)` решает, нужен ли экрану эскиз и
  какой `ThumbnailTarget` использовать;
- `RenderTarget::accepts_thumbnail(&AppState, &ThumbnailKey)` решает, можно ли
  лить готовый thumbnail в текущий экран.

Runtime не знает смыслов `PrintPage`, `ResultPage` и `FileSlot`; он только
доставляет готовый thumbnail, если текущий `RenderTarget` его принимает. Если
пользователь ушёл с экрана или слот файла уже занят другим path, готовый эскиз
остаётся в cache и не отправляется в UART.

### Конфиг

Пример:

```toml
[printer]
host = "192.168.0.20"
moonraker_port = 7125

[hmi]
serial = "/dev/ttyUSB0"
baud = 115200

[log]
level = "info"
touch_level = "info"
numeric_level = "info"
```

Полный пример лежит в `vaptechclient/config/config.example.toml`.

### Запуск

Из каталога `vaptechclient`:

```bash
cargo run -- --config config/config.example.toml
```

С debug-логами:

```bash
RUST_LOG=debug cargo run -- --config config/config.example.toml
```

С максимально подробными логами UART/Moonraker:

```bash
RUST_LOG=trace cargo run -- --config config/config.example.toml
```

`RUST_LOG` имеет приоритет над `[log].level` из конфига.

### Уровни логирования

- `error` - только критические ошибки сервисов.
- `warn` - обрывы Moonraker/WebSocket и восстановимые ошибки.
- `info` - старт runtime, подключение к Moonraker.
- `debug` - разобранные события Moonraker, HMI-команды после render diff.
- `trace` - сырые входящие сообщения Moonraker и подробный UART-поток.

Для обычной проверки дисплея удобнее `debug`. Для разбора протокола - `trace`.

Отдельно можно настроить уровень логирования разобранных HMI-событий дисплея:

```toml
[log]
touch_level = "info"
numeric_level = "info"
```

Поддерживаются `off`, `trace`, `debug`, `info`, `warn`, `error`.
`touch_level` логирует `HmiEvent touch` с полями `page` и `component`.
`numeric_level` логирует `HmiEvent numeric` с полями `page`, `component` и
`value`. Это уже разобранные события, без сырого hex UART.

### Проверка

```bash
cargo fmt
cargo test
```

## Поведение дисплея

Дисплей присылает init как одиночный байт:

```text
0x91
```

Это не terminated-frame и не `91 ff ff ff`.

После init клиент смотрит cached state:

- если печать активна или на паузе - отправляет `page 2` и полный render страницы печати;
- иначе отправляет `page 0`.

Это важно после краткой потери питания дисплея: печать не прерывается, а экран
перерисовывается из текущего состояния клиента.
