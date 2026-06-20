"""UART transport for the modular display demo host."""

from __future__ import annotations

from collections.abc import Callable
import os
import select
import termios
import threading
import time

from .protocol import (
    decode_frame,
    encode_command,
    hex_bytes,
    parse_numeric_frame,
    parse_touch_frame,
    split_frames,
    NumericEvent,
    TouchEvent,
)
from .state import DemoState


BAUD_MAP = {
    9600: termios.B9600,
    19200: termios.B19200,
    38400: termios.B38400,
    57600: termios.B57600,
    115200: termios.B115200,
    230400: termios.B230400,
}

LogFile = object | None
TouchHandler = Callable[[TouchEvent], None]
NumericHandler = Callable[[NumericEvent], None]
DisplayInitHandler = Callable[[], None]


def open_uart(path: str, baud: int) -> int:
    if baud not in BAUD_MAP:
        raise ValueError(f"unsupported baud rate: {baud}")

    fd = os.open(path, os.O_RDWR | os.O_NOCTTY | os.O_NONBLOCK)
    attrs = termios.tcgetattr(fd)
    attrs[0] = 0
    attrs[1] = 0
    attrs[2] = termios.CS8 | termios.CREAD | termios.CLOCAL
    attrs[3] = 0
    attrs[4] = BAUD_MAP[baud]
    attrs[5] = BAUD_MAP[baud]
    attrs[6][termios.VMIN] = 0
    attrs[6][termios.VTIME] = 1
    termios.tcsetattr(fd, termios.TCSANOW, attrs)
    return fd


def log_line(log_file: LogFile, line: str) -> None:
    print(line, flush=True)
    if log_file is not None:
        print(line, file=log_file, flush=True)


def send_cmd(fd: int, command: str, log_file: LogFile = None) -> None:
    os.write(fd, encode_command(command))
    termios.tcdrain(fd)
    log_line(log_file, f"> {command}")


def reader(
    fd: int,
    stop: threading.Event,
    log_file: LogFile,
    state: DemoState,
    *,
    on_touch: TouchHandler | None = None,
    on_numeric: NumericHandler | None = None,
    on_display_init: DisplayInitHandler | None = None,
) -> None:
    buffer = bytearray()
    last_rx = 0.0
    while not stop.is_set():
        ready, _, _ = select.select([fd], [], [], 0.2)
        if not ready:
            if buffer and last_rx and time.monotonic() - last_rx >= 0.5:
                if len(buffer) >= 8 and all(byte == 0x91 for byte in buffer):
                    should_init = _accept_display_init(state)
                    prefix = hex_bytes(buffer[:16])
                    suffix = " ..." if len(buffer) > 16 else ""
                    log_line(log_file, f"< {prefix}{suffix} display-init-signal repeat={len(buffer)}")
                    buffer.clear()
                    last_rx = 0.0
                    if should_init and on_display_init is not None:
                        on_display_init()
                    continue
                log_line(log_file, f"< {hex_bytes(buffer)} partial")
                buffer.clear()
                last_rx = 0.0
            continue

        try:
            chunk = os.read(fd, 4096)
        except BlockingIOError:
            continue
        if not chunk:
            continue
        buffer.extend(chunk)
        last_rx = time.monotonic()
        for frame in split_frames(buffer):
            log_line(log_file, f"< {decode_frame(frame)}")
            touch = parse_touch_frame(frame)
            if touch is not None and on_touch is not None:
                on_touch(touch)
            numeric = parse_numeric_frame(frame)
            if numeric is not None and on_numeric is not None:
                on_numeric(numeric)


def _accept_display_init(state: DemoState) -> bool:
    now = time.monotonic()
    with state.init_lock:
        elapsed = now - state.last_display_init_signal
        if elapsed < 5.0:
            return False
        state.last_display_init_signal = now
        return True

