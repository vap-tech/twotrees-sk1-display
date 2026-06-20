"""Load/unload page renderer and demo heating process."""

from __future__ import annotations

import threading
import time

from ..state import DemoState
from ..uart import LogFile, log_line, send_cmd


def send_load_unload_page(fd: int, state: DemoState, log_file: LogFile = None) -> None:
    with state.lock:
        temperature = state.load_unload_temp

    commands = [
        "page 4",
        f"n1.val={temperature}",
    ]
    for command in commands:
        send_cmd(fd, command, log_file)
        time.sleep(0.03)


def adjust_load_unload_temperature(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    delta: int,
) -> None:
    with state.lock:
        state.load_unload_temp = max(0, min(300, state.load_unload_temp + delta))
        temperature = state.load_unload_temp
    log_line(log_file, f"# load/unload temperature set: {temperature}")
    send_cmd(fd, f"n1.val={temperature}", log_file)


def start_load_unload_process(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
    action: str,
) -> None:
    with state.lock:
        state.load_unload_process_id += 1
        process_id = state.load_unload_process_id

    log_line(log_file, f"# {action} process started")
    thread = threading.Thread(
        target=run_load_unload_process,
        args=(fd, state, log_file, stop, process_id, action),
        daemon=True,
    )
    thread.start()


def run_load_unload_process(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
    process_id: int,
    action: str,
) -> None:
    with state.lock:
        current = state.nozzle_current
        target = state.load_unload_temp

    process_page = 5 if action == "load" else 42
    target_component = "n1" if action == "load" else "n2"
    heat_complete_flag = "Maen_heat_comp=1" if action == "load" else "Mare_heat_comp=1"
    commands = [
        f"page {process_page}",
        "t2.aph=0",
        "t3.aph=0",
        "q1.picc=14",
        "q2.picc=14",
        "q0.picc=15",
        "t1.aph=100",
        f"n0.val={current}",
        f"{target_component}.val={target}",
    ]
    for command in commands:
        send_cmd(fd, command, log_file)
        time.sleep(0.03)

    while not stop.is_set():
        with state.lock:
            if process_id != state.load_unload_process_id:
                return
            current = state.nozzle_current
            target = state.load_unload_temp
            if current == target:
                break
            step = min(6, abs(target - current))
            current += step if current < target else -step
            state.nozzle_current = current
        send_cmd(fd, f"n0.val={current}", log_file)
        send_cmd(fd, f"{target_component}.val={target}", log_file)
        time.sleep(0.5)

    for command in [
        heat_complete_flag,
        "q1.picc=15",
        "t2.aph=100",
        "vis t2,1",
    ]:
        send_cmd(fd, command, log_file)
        time.sleep(0.03)

    time.sleep(2)
    with state.lock:
        if process_id != state.load_unload_process_id:
            return
        current = state.nozzle_current
        target = state.load_unload_temp
    for command in [
        f"n0.val={current}",
        f"{target_component}.val={target}",
        "q2.picc=15",
        "t3.aph=100",
        "vis t3,1",
    ]:
        send_cmd(fd, command, log_file)
        time.sleep(0.03)

    end_time = time.monotonic() + 2.0
    while not stop.is_set() and time.monotonic() < end_time:
        time.sleep(0.1)
    if stop.is_set():
        return

    for command in [
        "page 4",
        f"n0.val={current}",
        f"n1.val={target}",
    ]:
        send_cmd(fd, command, log_file)
        time.sleep(0.03)
