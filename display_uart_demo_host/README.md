## Cтруктура

```text
display_uart_demo_host/
  __init__.py
  main.py              # argparse, signal handling, запуск задач
  uart.py              # open_uart, send_cmd, reader, decode raw frames
  protocol.py          # frame decode/encode, hex helpers, constants
  state.py             # DemoState, FileEntry, HistoryEntry
  temperature.py       # temperature target input and ramping
  pages/
    __init__.py
    home.py            # send_home_state
    fans.py            # send_fan_page
    move_temp.py       # send_move_temp_page, movement alerts
    load_unload.py     # load/unload pages and process
    files.py           # local/usb files, preview chunks
    print.py           # active print, result, stop flow
    history.py         # print history rows
    network.py         # network/searching pages
    system.py          # system/about/obico/info/power
    calibration.py     # calibration main/runs/z-tilt
  events.py            # handle_touch_event, handle_numeric_event dispatch
  console.py           # manual console commands and USB probes
  thumbnails.py        # load_demo_thumbnail_payloads
  tasks.py             # cancelable background jobs/thread helpers
```

## Асинхронная модель

Сейчас скрипт работает через потоки и прямые `send_cmd`. Возможно стоит сразу сделать один слой отправки:

```text
page/event code -> tx queue -> uart writer
reader -> event queue -> dispatcher
background jobs -> cancel token -> tx queue
```

Минимально: длинные процессы вроде thumbnail, homing, calibration, temperature ramp не пишут в UART напрямую. Они кладут команды в очередь и регулярно проверяют отмену.


## Текущий статус

Вроде работает:

- `protocol.py`, `uart.py`, `state.py`, `tasks.py`, `console.py`, `thumbnails.py`, `temperature.py`;
- `main.py` как точка входа;
- `pages/home.py`, `pages/fans.py`, `pages/move_temp.py`, `pages/load_unload.py`, `pages/files.py`, `pages/print.py`, `pages/history.py`, `pages/network.py`, `pages/system.py`, `pages/calibration.py`;
- базовый `events.py` для home, fan, move/temp/homing, load/unload, files/local/usb/preview/history, print pause/resume/stop/result, network, system/info/power и calibration.
