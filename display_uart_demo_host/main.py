#!/usr/bin/env python3
"""Modular demo host entry point for a detached TwoTrees SK1 TJC display."""

from __future__ import annotations

import argparse
from pathlib import Path
import os
import signal
import sys
import threading
import time

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    from display_uart_demo_host.console import handle_console_input
    from display_uart_demo_host.events import handle_numeric_event, handle_touch_event
    from display_uart_demo_host.protocol import NumericEvent, TouchEvent
    from display_uart_demo_host.state import DemoState
    from display_uart_demo_host.tasks import run_init_sequence
    from display_uart_demo_host.uart import log_line, open_uart, reader
else:
    from .console import handle_console_input
    from .events import handle_numeric_event, handle_touch_event
    from .protocol import NumericEvent, TouchEvent
    from .state import DemoState
    from .tasks import run_init_sequence
    from .uart import log_line, open_uart, reader


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("-p", "--port", default="/dev/ttyUSB0")
    parser.add_argument("-b", "--baud", type=int, default=115200)
    parser.add_argument(
        "-o",
        "--log",
        type=Path,
        default=Path("captures/display-demo-host-modular-latest.log"),
    )
    parser.add_argument("--append", action="store_true", help="append instead of overwriting")
    parser.add_argument("--boot-sleep", type=float, default=15.0)
    parser.add_argument("--starting-sleep", type=float, default=10.0)
    parser.add_argument(
        "--z-home-probe-error",
        action="store_true",
        help="simulate the Home Z 'Probe triggered prior to movement' alert later",
    )
    args = parser.parse_args()

    stop = threading.Event()
    state = DemoState()
    state.z_home_probe_error = args.z_home_probe_error

    def handle_signal(_signum: int, _frame: object) -> None:
        stop.set()

    def handle_touch(event: TouchEvent) -> None:
        handle_touch_event(fd, state, log_file, stop, event)

    def handle_numeric(event: NumericEvent) -> None:
        handle_numeric_event(fd, state, log_file, stop, event)

    def handle_display_init() -> None:
        thread = threading.Thread(
            target=run_init_sequence,
            args=(fd, state, log_file, stop),
            kwargs={
                "boot_sleep": 0.0,
                "starting_sleep": args.starting_sleep,
                "source": "display 0x91",
            },
            daemon=True,
        )
        thread.start()

    signal.signal(signal.SIGINT, handle_signal)
    signal.signal(signal.SIGTERM, handle_signal)

    args.log.parent.mkdir(parents=True, exist_ok=True)
    mode = "a" if args.append else "w"
    log_file = args.log.open(mode, encoding="utf-8")
    run_id = time.strftime("%Y-%m-%d %H:%M:%S %z")

    fd = open_uart(args.port, args.baud)
    thread = threading.Thread(
        target=reader,
        args=(fd, stop, log_file, state),
        kwargs={
            "on_touch": handle_touch,
            "on_numeric": handle_numeric,
            "on_display_init": handle_display_init,
        },
        daemon=True,
    )
    thread.start()

    try:
        log_line(log_file, f"--- modular demo-host start {run_id} port={args.port} baud={args.baud} ---")
        run_init_sequence(
            fd,
            state,
            log_file,
            stop,
            boot_sleep=args.boot_sleep,
            starting_sleep=args.starting_sleep,
            source="demo startup",
        )
        log_line(
            log_file,
            "# modular demo sequence done; listening for display events; type '> command' to send TJC, Ctrl-C to exit",
        )
        while not stop.is_set():
            handle_console_input(fd, log_file)
            time.sleep(0.2)
    finally:
        stop.set()
        thread.join(timeout=1)
        os.close(fd)
        log_line(log_file, "--- modular demo-host end ---")
        log_file.close()

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
