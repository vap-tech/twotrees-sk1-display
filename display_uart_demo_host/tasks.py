"""Small background/demo task helpers."""

from __future__ import annotations

import threading
import time

from .pages.home import send_home_state
from .state import DemoState
from .uart import LogFile, log_line, send_cmd


def sleep_cancelable(seconds: float, stop: threading.Event) -> bool:
    deadline = time.monotonic() + seconds
    while not stop.is_set():
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            return True
        time.sleep(min(0.1, remaining))
    return False


def sleep_with_log(seconds: float, label: str, log_file: LogFile, stop: threading.Event) -> bool:
    log_line(log_file, f"# sleep {seconds:g}s: {label}")
    return sleep_cancelable(seconds, stop)


def run_init_sequence(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
    *,
    boot_sleep: float = 0.0,
    starting_sleep: float = 10.0,
    source: str,
) -> None:
    with state.init_lock:
        if state.init_running:
            log_line(log_file, f"# init sequence ignored while already running: source={source}")
            return
        state.init_running = True

    try:
        log_line(log_file, f"# init sequence start: source={source}")
        if boot_sleep > 0:
            if not sleep_with_log(boot_sleep, "pretend Klipper/Linux is booting", log_file, stop):
                return
        send_cmd(fd, "page 45", log_file=log_file)
        if starting_sleep > 0:
            if not sleep_with_log(starting_sleep, "pretend mksclient waits for Klipper", log_file, stop):
                return
        send_home_state(fd, state, log_file=log_file)
        log_line(log_file, f"# init sequence done: source={source}")
    finally:
        with state.init_lock:
            state.init_running = False
