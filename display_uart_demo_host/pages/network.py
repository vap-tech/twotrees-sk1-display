"""Network page renderer."""

from __future__ import annotations

import time

from ..protocol import quote_tjc_text
from ..state import DemoState
from ..uart import LogFile, send_cmd


def send_network_page(fd: int, state: DemoState, log_file: LogFile = None) -> None:
    with state.lock:
        ip_address = state.ip_address
        networks = list(state.wifi_networks[:4])
        state.wifi_signal_bars = 4

    while len(networks) < 4:
        networks.append("")

    for command in ["page 18", "page 62"]:
        send_cmd(fd, command, log_file)
        time.sleep(0.03)

    time.sleep(2)

    commands = [
        "page 18",
        f't5.txt="IP:{quote_tjc_text(ip_address)}"',
        "Network.b7.picc2=41",
        "Network.p0.pic=71",
        f'b7.txt="{quote_tjc_text(networks[0])}"',
        "p4.pic=42",
        "Network.b8.picc2=41",
        "Network.p1.pic=68",
        f'b8.txt="{quote_tjc_text(networks[1])}"',
        "Network.b9.picc2=41",
        "Network.p2.pic=68",
        f'b9.txt="{quote_tjc_text(networks[2])}"',
        "Network.b10.picc2=41",
        "Network.p3.pic=68",
        f'b10.txt="{quote_tjc_text(networks[3])}"',
        f't5.txt="IP:{quote_tjc_text(ip_address)}"',
    ]
    for command in commands:
        send_cmd(fd, command, log_file)
        time.sleep(0.03)
