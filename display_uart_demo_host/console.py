"""Interactive console commands and USB component probes."""

from __future__ import annotations

import select
import sys
import time

from .protocol import scoped_component
from .thumbnails import load_demo_thumbnail_payloads
from .uart import LogFile, log_line, send_cmd


def send_usb_text_probe(fd: int, log_file: LogFile, *, qualified: bool = False) -> None:
    scope = "U_disk" if qualified else None
    label = "qualified" if qualified else "plain"
    log_line(log_file, f"# usb text probe: {label}")
    send_cmd(fd, "page 54", log_file)
    time.sleep(0.05)
    for index in range(25):
        send_cmd(fd, f'{scoped_component(scope, f"t{index}")}.txt="{label[:1]}t{index}"', log_file)
        time.sleep(0.01)
    for index in range(21):
        send_cmd(fd, f'{scoped_component(scope, f"b{index}")}.txt="{label[:1]}b{index}"', log_file)
        time.sleep(0.01)
    for index in range(6):
        send_cmd(fd, f'{scoped_component(scope, f"g{index}")}.txt="{label[:1]}g{index}"', log_file)
        time.sleep(0.01)


def send_usb_row_probe(fd: int, log_file: LogFile) -> None:
    log_line(log_file, "# usb row probe")
    send_cmd(fd, "page 54", log_file)
    time.sleep(0.05)
    for prefix, start, stop in (
        ("t", 12, 31),
        ("b", 12, 31),
        ("q", 0, 21),
        ("p", 0, 21),
        ("g", 0, 12),
        ("m", 0, 12),
        ("u", 0, 12),
        ("s", 0, 12),
    ):
        for index in range(start, stop):
            name = f"{prefix}{index}"
            send_cmd(fd, f'{name}.txt="{name}"', log_file)
            time.sleep(0.01)


def send_usb_cp_probe(fd: int, log_file: LogFile, *, target: int | None = None) -> None:
    thumbnail_payloads = load_demo_thumbnail_payloads()
    payload = next((chunks for chunks in thumbnail_payloads.values() if chunks), [])
    if not payload:
        log_line(log_file, "# usb cp probe skipped: no captured thumbnail payload")
        return

    log_line(log_file, f"# usb cp probe: {'scan' if target is None else f'cp{target} full'}")
    send_cmd(fd, "page 54", log_file)
    time.sleep(0.05)

    if target is None:
        indexes = range(9)
        chunks = payload[:1]
    else:
        indexes = range(target, target + 1)
        chunks = payload

    for index in indexes:
        component = f"cp{index}"
        send_cmd(fd, f"{component}.close()", log_file)
        send_cmd(fd, f"vis {component},1", log_file)
        for chunk in chunks:
            send_cmd(fd, f'{component}.write("{chunk}")', log_file)
            time.sleep(0.03)
        time.sleep(0.08)


def send_usb_get_text_probe(fd: int, log_file: LogFile) -> None:
    log_line(log_file, "# usb get text probe")
    send_cmd(fd, "page 54", log_file)
    time.sleep(0.05)
    for prefix in ("t", "b", "m", "g", "u", "s"):
        for index in range(32):
            send_cmd(fd, f"get {prefix}{index}.txt", log_file)
            time.sleep(0.04)


def send_usb_attr_probe(fd: int, log_file: LogFile, *, wide: bool = False) -> None:
    log_line(log_file, f"# usb attr probe: {'wide' if wide else 'compact'}")
    send_cmd(fd, "page 54", log_file)
    time.sleep(0.05)

    prefixes = {
        "t": range(0, 31 if wide else 16),
        "b": range(0, 31 if wide else 21),
        "q": range(0, 12 if wide else 4),
        "p": range(0, 12 if wide else 4),
        "n": range(0, 10 if wide else 6),
        "cp": range(0, 6 if wide else 3),
    }
    attrs = ("x", "y", "w", "h", "txt")
    for prefix, indexes in prefixes.items():
        for index in indexes:
            component = f"{prefix}{index}"
            log_line(log_file, f"# usb attr candidate: {component}")
            for attr in attrs:
                send_cmd(fd, f"get {component}.{attr}", log_file)
                time.sleep(0.035)


def send_usb_label_style_probe(fd: int, log_file: LogFile) -> None:
    log_line(log_file, "# usb label style probe")
    send_cmd(fd, "page 54", log_file)
    time.sleep(0.05)

    labels = (
        ("t12", "AAA-t12", 65535, 0, 0),
        ("t13", "BBB-t13", 63488, 65535, 1),
        ("t14", "CCC-t14", 0, 65504, 0),
    )
    for component, text, pco, bco, font in labels:
        commands = [
            f"vis {component},1",
            f"tsw {component},1",
            f"{component}.font={font}",
            f"{component}.pco={pco}",
            f"{component}.bco={bco}",
            f"{component}.sta=1",
            f"{component}.xcen=1",
            f"{component}.ycen=1",
            f'{component}.txt="{text}"',
            f"ref {component}",
        ]
        for command in commands:
            send_cmd(fd, command, log_file)
            time.sleep(0.025)


def send_usb_label_move_probe(fd: int, log_file: LogFile) -> None:
    log_line(log_file, "# usb label move probe")
    send_cmd(fd, "page 54", log_file)
    time.sleep(0.05)

    labels = (
        ("t12", "MOVE12", 15, 205),
        ("t13", "MOVE13", 148, 205),
        ("t14", "MOVE14", 281, 205),
    )
    for component, text, x, y in labels:
        commands = [
            f"vis {component},1",
            f"{component}.x={x}",
            f"{component}.y={y}",
            f"{component}.w=115",
            f"{component}.h=22",
            f"{component}.font=0",
            f"{component}.pco=65535",
            f"{component}.bco=0",
            f"{component}.sta=1",
            f'{component}.txt="{text}"',
            f"ref {component}",
            f"get {component}.x",
            f"get {component}.y",
            f"get {component}.txt",
        ]
        for command in commands:
            send_cmd(fd, command, log_file)
            time.sleep(0.03)


def send_usb_number_probe(fd: int, log_file: LogFile) -> None:
    candidates = [
        *(f"t{index}" for index in range(15)),
        "b3",
        *(f"b{index}" for index in range(5, 15)),
    ]

    log_line(log_file, "# usb number probe")
    send_cmd(fd, "page 54", log_file)
    time.sleep(0.05)
    for number, component in enumerate(candidates, start=1):
        label = f"{number:02d}"
        log_line(log_file, f"# usb number probe map: {label} -> {component}")
        send_cmd(fd, f"vis {component},1", log_file)
        send_cmd(fd, f'{component}.txt="{label}"', log_file)
        time.sleep(0.04)


def send_usb_named_probe(fd: int, log_file: LogFile, group: str) -> None:
    candidate_groups = {
        "a": [
            *(f"file{index}" for index in range(8)),
            *(f"fname{index}" for index in range(8)),
            *(f"name{index}" for index in range(8)),
            *(f"fn{index}" for index in range(8)),
        ],
        "b": [
            *(f"txt{index}" for index in range(8)),
            *(f"text{index}" for index in range(8)),
            *(f"label{index}" for index in range(8)),
            *(f"item{index}" for index in range(8)),
        ],
        "c": [
            *(f"f{index}" for index in range(16)),
            *(f"l{index}" for index in range(16)),
            *(f"r{index}" for index in range(16)),
            *(f"d{index}" for index in range(16)),
        ],
        "d": [
            *(f"u{index}" for index in range(16)),
            *(f"usb{index}" for index in range(8)),
            *(f"ud{index}" for index in range(8)),
            *(f"disk{index}" for index in range(8)),
            *(f"path{index}" for index in range(8)),
        ],
    }
    candidates = candidate_groups[group]

    log_line(log_file, f"# usb named probe: group={group}")
    send_cmd(fd, "page 54", log_file)
    time.sleep(0.05)
    for number, component in enumerate(candidates, start=1):
        label = f"{number:02d}"
        log_line(log_file, f"# usb named probe map {group}: {label} -> {component}")
        send_cmd(fd, f"vis {component},1", log_file)
        send_cmd(fd, f'{component}.txt="{label}"', log_file)
        time.sleep(0.035)


def handle_console_input(fd: int, log_file: LogFile) -> None:
    if not sys.stdin.isatty():
        return
    ready, _, _ = select.select([sys.stdin], [], [], 0)
    if not ready:
        return
    line = sys.stdin.readline()
    if line == "":
        return
    line = line.strip()
    if not line:
        return
    if line == ":probe-usb-text":
        send_usb_text_probe(fd, log_file, qualified=False)
        return
    if line == ":probe-usb-text-qualified":
        send_usb_text_probe(fd, log_file, qualified=True)
        return
    if line == ":probe-usb-rows":
        send_usb_row_probe(fd, log_file)
        return
    if line == ":probe-usb-cp":
        send_usb_cp_probe(fd, log_file)
        return
    if line in (":probe-usb-cp0", ":probe-usb-cp1", ":probe-usb-cp2"):
        send_usb_cp_probe(fd, log_file, target=int(line[-1]))
        return
    if line == ":probe-usb-get-text":
        send_usb_get_text_probe(fd, log_file)
        return
    if line == ":probe-usb-attrs":
        send_usb_attr_probe(fd, log_file, wide=False)
        return
    if line == ":probe-usb-attrs-wide":
        send_usb_attr_probe(fd, log_file, wide=True)
        return
    if line == ":probe-usb-labels-style":
        send_usb_label_style_probe(fd, log_file)
        return
    if line == ":probe-usb-labels-move":
        send_usb_label_move_probe(fd, log_file)
        return
    if line == ":probe-usb-num":
        send_usb_number_probe(fd, log_file)
        return
    if line in (":probe-usb-name-a", ":probe-usb-name-b", ":probe-usb-name-c", ":probe-usb-name-d"):
        send_usb_named_probe(fd, log_file, line[-1])
        return
    if not line.startswith(">"):
        log_line(
            log_file,
            "# console input ignored; prefix TJC commands with '>' or use :probe-usb-text/:probe-usb-rows/:probe-usb-cp",
        )
        return
    command = line[1:].strip()
    if not command:
        return
    send_cmd(fd, command, log_file=log_file)
