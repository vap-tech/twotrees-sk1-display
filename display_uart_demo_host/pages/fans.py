"""Fan page renderer."""

from __future__ import annotations

import time

from ..state import DemoState, FAN_NAMES
from ..uart import LogFile, log_line, send_cmd


def send_fan_page(fd: int, state: DemoState, log_file: LogFile = None) -> None:
    with state.lock:
        fan_values = list(state.fan_values)

    log_line(
        log_file,
        "# fan page: "
        + ", ".join(f"{FAN_NAMES[index]}={value}" for index, value in enumerate(fan_values)),
    )
    commands = ["page 6"]
    for index, value in enumerate(fan_values):
        commands.append(f"h{index}.val={value}")
        commands.append(f"n{index}.val={value}")

    for command in commands:
        send_cmd(fd, command, log_file)
        time.sleep(0.03)


def set_fan_value(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    index: int,
    value: int,
) -> None:
    value = max(0, min(100, value))
    with state.lock:
        state.fan_values[index] = value
    log_line(log_file, f"# {FAN_NAMES[index]} set from display: {value}")
    send_cmd(fd, f"h{index}.val={value}", log_file)
    send_cmd(fd, f"n{index}.val={value}", log_file)

