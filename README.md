# twotrees-sk1-display

Материалы по UART-дисплею TwoTrees SK1 и альтернативному клиенту для работы с
Moonraker/Klipper после перехода на ванильный Armbian.

## Важно

Дисплей питается от `5V`, но уровни `RX/TX` - `3.3V`.

Не подключайте UART дисплея к `5V TTL`. Общую землю (`GND`) подключать обязательно.

## Что есть в репозитории

- `tools/display_uart_demo_host` - Python-демо/песочница для дисплея.
- `display_uart_raw_protocol.md` - список известных UART-команд и входящих кадров.
- `vaptechclient/` - Rust-клиент для HMI-дисплея и Moonraker.
- `esp32-tft/` - экспериментальный клиент под ESP32-C6.


## vaptechclient

`vaptechclient` - текущий Rust-клиент. Он запускает:

- async UART service для дисплея;
- read-only Moonraker WebSocket client;
- центральный runtime;
- reducer состояния принтера;
- renderer команд HMI.

Сейчас Moonraker WebSocket используется только для чтения состояния. Управляющие
команды из UI пока не отправляются обратно в Moonraker.

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
