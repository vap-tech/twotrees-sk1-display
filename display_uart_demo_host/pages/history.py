"""Print history page renderer."""

from __future__ import annotations

import time

from ..protocol import quote_tjc_text
from ..state import DemoState
from ..uart import LogFile, log_line, send_cmd


ROW_DEFS = [
    ("t4", "t5", "t6", "q0"),
    ("t7", "t8", "t9", "q1"),
    ("t10", "t11", "t12", "q2"),
    ("t13", "t14", "t15", "q3"),
    ("t16", "t17", "t18", "q4"),
]


def send_history_page(fd: int, state: DemoState, log_file: LogFile = None) -> None:
    with state.lock:
        entries = list(state.history_entries[:5])

    log_line(log_file, "# history page")
    send_cmd(fd, "page 10", log_file)
    time.sleep(0.03)

    for index, (name_component, timestamp_component, duration_component, icon_component) in enumerate(ROW_DEFS):
        entry = entries[index] if index < len(entries) else None
        if entry is None:
            commands = [
                f'{name_component}.txt=""',
                f'{timestamp_component}.txt=""',
                f'{duration_component}.txt=""',
                f"{icon_component}.picc=0",
            ]
        else:
            commands = [
                f'{name_component}.txt="{quote_tjc_text(entry.name)}"',
                f'{timestamp_component}.txt="{quote_tjc_text(entry.timestamp)}"',
                f'{duration_component}.txt="{quote_tjc_text(entry.duration)}"',
                f"{icon_component}.picc={24 if entry.status == 'completed' else 25}",
            ]
        for command in commands:
            send_cmd(fd, command, log_file)
            time.sleep(0.03)

    send_cmd(fd, 't19.txt=""', log_file)
