"""Local/USB files pages and file preview."""

from __future__ import annotations

import time

from ..protocol import (
    quote_tjc_text,
    scoped_component,
)
from ..state import DemoState, FileEntry, shorten_file_name
from ..thumbnails import load_demo_thumbnail_payloads
from ..uart import LogFile, log_line, send_cmd


SLOT_DEFS = [
    {
        "name": "t12",
        "q": "q0",
        "b": "b12",
        "cp": "cp0",
        "hide": ["t3", "t0", "t1", "t2", "n0", "n1"],
        "time": "t2",
        "n": ("n0", "n1"),
    },
    {
        "name": "t13",
        "q": "q1",
        "b": "b13",
        "cp": "cp1",
        "hide": ["t7", "t4", "t5", "t6", "n2", "n3"],
        "time": "t6",
        "n": ("n2", "n3"),
    },
    {
        "name": "t14",
        "q": "q2",
        "b": "b14",
        "cp": "cp2",
        "hide": ["t11", "t8", "t9", "t10", "n4", "n5"],
        "time": "t10",
        "n": ("n4", "n5"),
    },
]


def send_files_page(
    fd: int,
    state: DemoState,
    log_file: LogFile = None,
    *,
    display_page: int = 7,
    close_scope: str | None = "Local_Files",
    write_scope: str | None = None,
) -> None:
    with state.lock:
        path = state.files_path
        page = state.files_page
        entries = _current_entries(state)
        visible_entries = entries[page * 3 : page * 3 + 3]
        state.files_view = "usb" if display_page == 54 else "local"
        view = state.files_view

    log_line(log_file, f"# files page: view={view} path={path} page={page}")
    commands = [
        f"page {display_page}",
        f"{scoped_component(close_scope, 'cp0')}.close()",
        f"{scoped_component(close_scope, 'cp1')}.close()",
        f"{scoped_component(close_scope, 'cp2')}.close()",
        "vis cp0,0",
        "vis cp1,0",
        "vis cp2,0",
    ]
    for command in commands:
        send_cmd(fd, command, log_file)
        time.sleep(0.03)

    thumbnail_payloads = load_demo_thumbnail_payloads()
    file_name_limit = 16 if display_page == 54 else 22

    for index, slot in enumerate(SLOT_DEFS):
        entry = visible_entries[index] if index < len(visible_entries) else None
        _send_slot(fd, log_file, slot, entry, file_name_limit)

        if entry is not None and entry.kind != "folder" and entry.has_preview:
            payload = thumbnail_payloads.get(index, [])
            if payload:
                send_cmd(fd, f'{scoped_component(close_scope, slot["cp"])}.close()', log_file)
                send_cmd(fd, f'vis {slot["cp"]},1', log_file)
                for chunk in payload:
                    send_cmd(fd, f'{scoped_component(write_scope, slot["cp"])}.write("{chunk}")', log_file)
                    time.sleep(0.03)


def send_usb_files_page(fd: int, state: DemoState, log_file: LogFile = None) -> None:
    send_files_page(fd, state, log_file, display_page=54, close_scope=None)


def redraw_files_page(fd: int, state: DemoState, log_file: LogFile = None) -> None:
    with state.lock:
        view = state.files_view
    if view == "usb":
        send_usb_files_page(fd, state, log_file)
    else:
        send_files_page(fd, state, log_file)


def select_local_files(fd: int, state: DemoState, log_file: LogFile) -> None:
    with state.lock:
        state.files_path = "/"
        state.files_page = 0
        state.files_view = "local"
    send_files_page(fd, state, log_file)


def select_usb_files(fd: int, state: DemoState, log_file: LogFile) -> None:
    with state.lock:
        state.files_path = "/sda"
        state.files_page = 0
        state.files_view = "usb"
    send_usb_files_page(fd, state, log_file)


def select_file_slot(fd: int, state: DemoState, log_file: LogFile, component: int) -> None:
    slot_index = {8: 0, 9: 1, 10: 2}[component]
    with state.lock:
        entries = _current_entries(state)
        entry_index = state.files_page * 3 + slot_index
        entry = entries[entry_index] if entry_index < len(entries) else None
        current_path = state.files_path

    if entry is None:
        log_line(
            log_file,
            f"# empty file slot selected: component={component} slot={slot_index} path={current_path}",
        )
        return

    if entry.kind == "folder":
        with state.lock:
            if state.files_path == "/" and entry.name == "sda":
                state.files_path = "/sda"
                state.files_page = 0
            elif state.files_path == "/sda":
                state.files_path = "/sda"
                state.files_page = 0
        log_line(log_file, f"# folder selected: component={component} slot={slot_index} name={entry.name}")
        redraw_files_page(fd, state, log_file)
        return

    log_line(log_file, f"# gcode selected: component={component} slot={slot_index} name={entry.name}")
    with state.lock:
        state.print_filename = entry.name
    send_file_preview_page(fd, entry.name, log_file)


def change_files_page(fd: int, state: DemoState, log_file: LogFile, component: int) -> None:
    with state.lock:
        entries = _current_entries(state)
        max_page = max(0, (len(entries) - 1) // 3)
        if component == 11:
            state.files_page = max(0, state.files_page - 1)
        else:
            state.files_page = min(max_page, state.files_page + 1)
        files_page = state.files_page
    log_line(log_file, f"# files page changed: {files_page}")
    redraw_files_page(fd, state, log_file)


def back_to_files_root(fd: int, state: DemoState, log_file: LogFile) -> None:
    with state.lock:
        if state.files_path != "/":
            state.files_path = "/"
            state.files_page = 0
            state.files_view = "local"
    log_line(log_file, "# files back to local root")
    redraw_files_page(fd, state, log_file)


def send_file_preview_page(fd: int, filename: str, log_file: LogFile = None) -> None:
    commands = [
        "page 9",
        f'g0.txt="{quote_tjc_text(filename)}"',
        "n4.val=00",
        "n5.val=00",
        't2.txt="0"',
        "preview.cp0.close()",
        "vis cp0,0",
    ]
    for command in commands:
        send_cmd(fd, command, log_file)
        time.sleep(0.03)


def _current_entries(state: DemoState) -> list[FileEntry]:
    return state.files_sda_entries if state.files_path == "/sda" else state.files_root_entries


def _send_slot(
    fd: int,
    log_file: LogFile,
    slot: dict[str, object],
    entry: FileEntry | None,
    file_name_limit: int,
) -> None:
    for component in slot["hide"]:
        send_cmd(fd, f"vis {component},0", log_file)
        time.sleep(0.01)

    if entry is None:
        slot_commands = [
            f'{slot["name"]}.txt=""',
            f'vis {slot["name"]},0',
            f'{slot["q"]}.picc=100',
            f'{slot["q"]}.picc2=100',
            f'{slot["b"]}.picc=100',
            't15.txt=""',
        ]
    elif entry.kind == "folder":
        slot_commands = [
            f'vis {slot["name"]},1',
            f'{slot["name"]}.txt="{quote_tjc_text(shorten_file_name(entry.name, file_name_limit))}"',
            f'{slot["q"]}.picc=99',
            f'{slot["q"]}.picc2=100',
            f'{slot["b"]}.picc=100',
            't15.txt=""',
        ]
    else:
        first_n, second_n = slot["n"]
        slot_commands = [
            f'vis {slot["name"]},1',
            f'{slot["name"]}.txt="{quote_tjc_text(shorten_file_name(entry.name, file_name_limit))}"',
            f'{slot["q"]}.picc=98',
            f'{slot["q"]}.picc2=99',
            f'{slot["b"]}.picc=18',
            f'vis {slot["time"]},1',
            f"vis {first_n},1",
            f"vis {second_n},1",
            f'{slot["time"]}.txt="0.000"',
            f"{first_n}.val=0",
            f"{second_n}.val=0",
            f"vis {slot['cp']},{1 if entry.has_preview else 0}",
            't15.txt=""',
        ]

    for command in slot_commands:
        send_cmd(fd, command, log_file)
        time.sleep(0.03)
