"""Home page renderer."""

from __future__ import annotations

import time

from ..state import DemoState
from ..uart import LogFile, send_cmd


def send_home_state(fd: int, state: DemoState, log_file: LogFile = None) -> None:
    with state.lock:
        nozzle_current = state.nozzle_current
        bed_current = state.bed_current
        nozzle_target = state.nozzle_target
        bed_target = state.bed_target
        caselight_pic = 3 if state.caselight_on else 2
        fan_pic = 3 if any(value > 0 for value in state.fan_values) else 2
        wifi_pic = 67 + max(0, min(4, state.wifi_signal_bars))

    commands = [
        "page 0",
        f"Start.p0.pic={wifi_pic}",
        f"n0.val={nozzle_current}",
        f"n1.val={bed_current}",
        f"n4.val={nozzle_target}",
        f"n5.val={bed_target}",
        f"b6.picc={fan_pic}",
        f"b6.picc2={fan_pic}",
        f"b5.picc={caselight_pic}",
        f"b5.picc2={caselight_pic}",
    ]
    for command in commands:
        send_cmd(fd, command, log_file)
        time.sleep(0.03)

