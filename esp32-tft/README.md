# SK1 ESP32 Display Client

ESP-IDF client for the TwoTrees SK1 TJC UART display.

## Hardware

- Target: ESP32-C6
- Flash: 16 MB
- Display UART: UART1, 115200 8N1
- TX: GPIO10
- RX: GPIO11
- Display logic level: 3.3 V

Do not connect the display's 5 V supply to an ESP32 GPIO.

## Architecture

- `display_uart`: the only UART writer, with high, normal, and low priority
  queues.
- `display_protocol`: decodes touch, numeric, text, status, and init frames.
- `ui_controller`: dispatches display events and owns page transitions.
- `ui_state`: shared printer/UI state protected by a mutex.
- `ui_pages_home`: first page renderer and periodic temperature updates.
- `wifi_manager`: station connection, reconnect, IP and RSSI events.
- `moonraker_client`: asynchronous HTTP heartbeat and printer-state polling.
- `app_state`: thread-safe cache shared between backend workers and UI.
- `files_cache`: thread-safe snapshot of the current Moonraker G-code
  directory.
- `ui_pages_files`: three-slot file browser with folders, pagination, parent
  navigation, and a file preview page without thumbnails.
- `ui_pages_print`: minimal live print page with filename, temperatures,
  progress, Wi-Fi state, and pause-button state.
- `thumbnail_worker`: cancellable low-priority Moonraker metadata/image
  loader, PNG decoder, ColPic encoder, and chunked HMI renderer.
- `thumbnail_cache`: small RAM cache for Moonraker thumbnail metadata.
- `app_watchdog`: reports stalled UART RX/TX activity.

Low-priority jobs use a generation token. A page transition increments the
generation and drains queued thumbnail/file-preview commands.

The UI never calls Wi-Fi or Moonraker directly. Backend tasks update
`app_state`; UI tasks only read snapshots from that cache.

Opening Files immediately renders page 7 in a loading state and queues an
asynchronous Moonraker directory request. Hidden entries are excluded,
directories are sorted before files, and selecting a file opens page 9.
Selecting a file queues its preview independently of the UI. The worker picks
the largest Moonraker thumbnail, decodes PNG rows into a 155x155 RGB image,
encodes the result as a 1024-color ColPic stream, and sends 1024-byte
`preview.cp0.write(...)` chunks through the LOW queue. A page transition
cancels HTTP reception, encoding output, queued chunks, and subsequent writes
through the shared generation token.

Encoded image caching in the `storage` partition is intentionally deferred
until the renderer has been visually checked with several different slicer
previews.

Pressing Print on page 9 queues an asynchronous
`POST /printer/print/start?filename=...` request. Repeated presses are ignored
while the command is pending. Page 2 is shown only after Moonraker returns
success; its basic live fields are then refreshed from `app_state`.

Connection and print status are represented by independent state machines:

```text
connection:
  boot -> wifi_connecting -> wifi_connected
       -> moonraker_connecting -> moonraker_ready

print:
  idle -> starting -> printing
  printing -> pausing -> paused
  paused -> resuming -> printing
  printing/paused -> cancelling -> cancelled
  printing -> complete
  error
```

Keeping these states independent avoids impossible combined states during
Wi-Fi reconnects while a print remains paused or active.

`complete` and `cancelled` are latched until the result page is acknowledged.
This prevents a short-lived Moonraker terminal state from being lost between
polls. If the ESP restarts or reconnects while Moonraker reports `printing` or
`paused`, the current filename is recovered from `print_stats` and page 2 is
restored automatically.

Print controls use the same asynchronous command queue:

- page 2 component 5 opens page 27 while printing, or resumes from paused;
- page 27 component 0 sends `/printer/print/pause`;
- page 27 component 1 shows page 73 and sends `/printer/print/cancel`;
- page 27 component 2 returns without issuing a command;
- resume briefly uses page 74 and sends `/printer/print/resume`;
- successful cancel keeps page 73 visible until Moonraker reports the print
  idle, then opens page 77 with the abnormal result; Back returns home.

## Build

```sh
idf.py build
```

## Flash

```sh
idf.py -p /dev/ttyACM0 flash monitor
```

The custom partition table provides two 5 MB OTA application slots and about
5.9 MB of storage for cached previews and application data.
