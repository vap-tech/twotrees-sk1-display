"""System, info, power and Obico pages."""

from __future__ import annotations

import threading
import time

from ..state import DemoState
from ..tasks import sleep_cancelable
from ..uart import LogFile, log_line, send_cmd


def send_system_page(fd: int, log_file: LogFile = None) -> None:
    send_cmd(fd, "page 11", log_file)


def send_lower_system_page(fd: int, log_file: LogFile = None) -> None:
    send_cmd(fd, "page 14", log_file)


def send_export_diary_page(fd: int, log_file: LogFile = None) -> None:
    send_cmd(fd, "page 19", log_file)


def send_about_page(fd: int, log_file: LogFile = None) -> None:
    send_cmd(fd, "page 15", log_file)


def send_factory_reset_page(fd: int, log_file: LogFile = None) -> None:
    send_cmd(fd, "page 20", log_file)


def send_power_menu(fd: int, log_file: LogFile = None) -> None:
    send_cmd(fd, "page 68", log_file)


def start_obico_page(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
) -> None:
    with state.lock:
        state.obico_refresh_id += 1
        refresh_id = state.obico_refresh_id
    log_line(log_file, "# Obico link selected")
    thread = threading.Thread(
        target=run_obico_page,
        args=(fd, state, log_file, stop, refresh_id),
        daemon=True,
    )
    thread.start()


def cancel_obico_refresh(state: DemoState) -> None:
    with state.lock:
        state.obico_refresh_id += 1


def run_obico_page(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
    refresh_id: int,
) -> None:
    demo_link = "https://app.obico.io/printers/wizard/link/?demo=1"
    initial_commands = [
        'obico.qr0.txt=""',
        "obico.qr0.aph=0",
        "obico.q0.picc=132",
        "obico.q1.picc=132",
        "page 78",
    ]
    for command in initial_commands:
        send_cmd(fd, command, log_file)
        time.sleep(0.03)

    while not stop.is_set():
        with state.lock:
            if refresh_id != state.obico_refresh_id:
                return
            commands = (
                "obico.q0.picc=131",
                "obico.qr0.aph=127",
                f'obico.qr0.txt="{demo_link}"',
                "obico.q1.picc=132",
            )
        for command in commands:
            send_cmd(fd, command, log_file)
            time.sleep(0.03)
        if not sleep_cancelable(3.0, stop):
            return


def start_demo_reboot(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
    home_sender,
) -> None:
    log_line(log_file, "# reboot selected")
    thread = threading.Thread(
        target=run_demo_reboot,
        args=(fd, state, log_file, stop, home_sender),
        daemon=True,
    )
    thread.start()


def run_demo_reboot(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
    home_sender,
) -> None:
    send_cmd(fd, "page 67", log_file)
    end_time = time.monotonic() + 8
    while not stop.is_set() and time.monotonic() < end_time:
        time.sleep(0.1)
    if stop.is_set():
        return
    home_sender(fd, state, log_file)


def handle_info_pages(fd: int, log_file: LogFile, page: int, component: int) -> bool:
    transitions = {
        (21, 6): ("# online manual selected from FAQ", "page 52"),
        (21, 7): ("# contact selected from FAQ", "page 53"),
        (52, 5): ("# FAQ selected from online manual", "page 21"),
        (52, 7): ("# contact selected from online manual", "page 53"),
        (53, 5): ("# FAQ selected from contact", "page 21"),
        (53, 6): ("# online manual selected from contact", "page 52"),
    }
    transition = transitions.get((page, component))
    if transition is None:
        return False
    message, command = transition
    log_line(log_file, message)
    send_cmd(fd, command, log_file)
    return True
