"""Print preview, active print and result pages."""

from __future__ import annotations

from datetime import datetime
import threading
import time

from ..protocol import quote_tjc_text
from ..state import DemoState
from ..tasks import sleep_cancelable
from ..uart import LogFile, log_line, send_cmd


def send_print_page(fd: int, state: DemoState, log_file: LogFile = None) -> None:
    with state.lock:
        filename = state.print_filename
        progress = state.print_progress
        paused = state.print_paused
        nozzle_current = state.nozzle_current
        bed_current = state.bed_current
        nozzle_target = state.nozzle_target
        bed_target = state.bed_target
        wifi_pic = 67 + max(0, min(4, state.wifi_signal_bars))

    button_pic, button_pic2 = (5, 4) if paused else (4, 5)
    commands = [
        "page 2",
        f"Print_Trun_1.p0.pic={wifi_pic}",
        f'g0.txt="{quote_tjc_text(filename)}"',
        "n4.val=0",
        "n5.val=0",
        "n7.val=0",
        "n8.val=0",
        f"n6.val={progress}",
        "vis cp0,0",
        f"b5.picc={button_pic}",
        f"b5.picc2={button_pic2}",
        f"n0.val={nozzle_current}",
        f"n1.val={bed_current}",
        f't8.txt="{nozzle_target}"',
        f't9.txt="{bed_target}"',
    ]
    for command in commands:
        send_cmd(fd, command, log_file)
        time.sleep(0.03)


def start_demo_print(fd: int, state: DemoState, log_file: LogFile = None) -> None:
    with state.lock:
        state.print_run_id += 1
        state.print_active = True
        state.print_paused = False
        state.print_progress = 0
        if state.nozzle_target == 0:
            state.nozzle_target = 150
        if state.bed_target == 0:
            state.bed_target = 60
    log_line(log_file, "# demo print started")
    send_print_page(fd, state, log_file)


def pause_or_dialog_print(fd: int, state: DemoState, log_file: LogFile) -> None:
    with state.lock:
        print_active = state.print_active
        print_paused = state.print_paused

    if not print_active:
        log_line(log_file, "# print control ignored without an active print")
        return

    if print_paused:
        resume_print(fd, state, log_file)
        return

    log_line(log_file, "# print pause/stop dialog selected")
    send_cmd(fd, "page 27", log_file)


def pause_print(fd: int, state: DemoState, log_file: LogFile) -> None:
    with state.lock:
        state.print_paused = True
    log_line(log_file, "# print paused")
    send_print_page(fd, state, log_file)


def resume_print(fd: int, state: DemoState, log_file: LogFile) -> None:
    with state.lock:
        state.print_paused = False
    log_line(log_file, "# print resumed")
    send_cmd(fd, "page 74", log_file)
    time.sleep(0.5)
    send_print_page(fd, state, log_file)


def start_stop_print(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
) -> None:
    with state.lock:
        state.print_run_id += 1
        run_id = state.print_run_id
    log_line(log_file, "# print stop requested")
    thread = threading.Thread(
        target=run_stop_print,
        args=(fd, state, log_file, stop, run_id),
        daemon=True,
    )
    thread.start()


def send_print_result_page(
    fd: int,
    state: DemoState,
    log_file: LogFile = None,
    *,
    completed: bool,
) -> None:
    with state.lock:
        filename = state.print_filename

    commands = [
        "page 77",
        "print_done.cp0.close()",
        "vis print_done.cp0,0",
        f'g0.txt="{quote_tjc_text(filename)}"',
        f"print_done_flag={1 if completed else 0}",
        "print_done.tm0.en=1",
        f't2.txt="{datetime.now().strftime("%Y-%m-%d %H:%M")}\\n"',
        't4.txt="0m02s"',
    ]
    for command in commands:
        send_cmd(fd, command, log_file)
        time.sleep(0.03)


def run_stop_print(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
    run_id: int,
) -> None:
    send_cmd(fd, "page 73", log_file)
    if not sleep_cancelable(2.0, stop):
        return
    with state.lock:
        if run_id != state.print_run_id:
            return
        state.print_active = False
        state.print_paused = False
        state.nozzle_target = 0
        state.bed_target = 0
    send_print_result_page(fd, state, log_file, completed=False)
