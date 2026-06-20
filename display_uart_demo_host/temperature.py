"""Temperature target input and current-temperature ramping."""

from __future__ import annotations

import threading
import time

from .pages.home import send_home_state
from .pages.move_temp import send_move_temp_page
from .protocol import NumericEvent
from .state import DemoState
from .uart import LogFile, log_line, send_cmd


def handle_temperature_numeric_event(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
    event: NumericEvent,
) -> bool:
    if event.page == 0 and event.component == 7:
        set_temperature_target(fd, state, log_file, stop, "nozzle", event.value, display_page=0)
        return True
    if event.page == 0 and event.component == 8:
        set_temperature_target(fd, state, log_file, stop, "bed", event.value, display_page=0)
        return True
    if event.page == 3 and event.component == 16:
        set_temperature_target(fd, state, log_file, stop, "nozzle", event.value, display_page=3)
        return True
    if event.page == 3 and event.component == 17:
        set_temperature_target(fd, state, log_file, stop, "bed", event.value, display_page=3)
        return True
    return False


def set_temperature_target(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
    heater: str,
    value: int,
    *,
    display_page: int,
) -> None:
    with state.lock:
        if heater == "nozzle":
            state.nozzle_target = value
            state.nozzle_ramp_id += 1
            ramp_id = state.nozzle_ramp_id
        elif heater == "bed":
            state.bed_target = value
            state.bed_ramp_id += 1
            ramp_id = state.bed_ramp_id
        else:
            raise ValueError(f"unknown heater: {heater}")

    page_name = "move/temp " if display_page == 3 else ""
    log_line(log_file, f"# {page_name}{heater} target set from display: {value}")
    if display_page == 3:
        send_move_temp_page(fd, state, log_file)
    else:
        send_home_state(fd, state, log_file)

    thread = threading.Thread(
        target=ramp_temperature_to_target,
        args=(fd, state, log_file, stop, ramp_id),
        kwargs={"heater": heater, "display_page": display_page},
        daemon=True,
    )
    thread.start()


def ramp_temperature_to_target(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
    ramp_id: int,
    *,
    heater: str,
    display_page: int,
) -> None:
    while not stop.is_set():
        with state.lock:
            if heater == "bed":
                if ramp_id != state.bed_ramp_id:
                    return
                current = state.bed_current
                target = state.bed_target
            elif heater == "nozzle":
                if ramp_id != state.nozzle_ramp_id:
                    return
                current = state.nozzle_current
                target = state.nozzle_target
            else:
                raise ValueError(f"unknown heater: {heater}")

            if current == target:
                return

            current += 1 if current < target else -1
            if heater == "bed":
                state.bed_current = current
                component = "n2" if display_page == 3 else "n1"
            else:
                state.nozzle_current = current
                component = "n3" if display_page == 3 else "n0"

        send_cmd(fd, f"{component}.val={current}", log_file)
        time.sleep(1)
