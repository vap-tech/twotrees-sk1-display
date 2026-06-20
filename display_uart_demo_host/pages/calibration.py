"""Calibration pages and demo calibration runs."""

from __future__ import annotations

import threading
import time

from ..state import DemoState
from ..tasks import sleep_cancelable
from ..uart import LogFile, log_line, send_cmd


MESH_VALUES = [
    -0.18, -0.03, 0.01, 0.01, -0.02, -0.15,
    -0.16, -0.01, 0.03, 0.02, -0.01, -0.15,
    -0.17, -0.03, 0.02, 0.00, -0.05, -0.18,
    -0.18, -0.05, -0.00, -0.02, -0.09, -0.22,
    -0.22, -0.09, -0.04, -0.06, -0.13, -0.27,
    -0.26, -0.15, -0.12, -0.15, -0.20, -0.32,
]

CALIBRATION_LABELS = {
    8: "Triangle",
    9: "Z_Tilt",
    10: "Bed Mesh",
    11: "Shaper_Calibrate",
}


def send_calibration_page(fd: int, state: DemoState, log_file: LogFile = None) -> None:
    with state.lock:
        z_max = state.z_max
        z_min = state.z_min
        z_offset = state.z_offset
        shaper_freq_x = state.shaper_freq_x
        shaper_freq_y = state.shaper_freq_y
        tilt_tolerance = state.tilt_tolerance

    log_line(log_file, "# calibration page")
    send_cmd(fd, "page 33", log_file)

    for index in range(36):
        send_cmd(fd, f"q{index + 4}.picc=116", log_file)
        send_cmd(fd, f't{index + 6}.txt="0.00"', log_file)
        time.sleep(0.01)

    for index, value in enumerate(MESH_VALUES):
        send_cmd(fd, f"q{index + 4}.picc=80", log_file)
        send_cmd(fd, f't{index + 6}.txt="{value:.2f}"', log_file)
        time.sleep(0.01)

    commands = [
        f't1.txt="{z_max:.2f}"',
        f't2.txt="{z_min:.2f}"',
        f't3.txt="{z_offset:.3f}"',
        f't4.txt="{shaper_freq_x:.1f}"',
        f't5.txt="{shaper_freq_y:.1f}"',
        f't0.txt="{tilt_tolerance:.4f}"',
        "filament.b9.picc=28",
        "filament.b9.picc2=28",
    ]
    send_commands(fd, commands, log_file)


def start_calibration_action(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
    component: int,
) -> None:
    mode_by_component = {
        8: "triangle",
        9: "z_tilt",
        10: "bed_mesh",
        11: "shaper",
    }
    runner_by_component = {
        8: run_demo_triangle,
        9: run_demo_z_tilt,
        10: run_demo_bed_mesh,
        11: run_demo_shaper,
    }
    log_line(log_file, f"# calibration action selected: {CALIBRATION_LABELS[component]}")
    run_id = start_calibration_run(state, mode_by_component[component])
    thread = threading.Thread(
        target=runner_by_component[component],
        args=(fd, state, log_file, stop, run_id),
        daemon=True,
    )
    thread.start()


def stop_calibration(fd: int, state: DemoState, log_file: LogFile, page: int) -> None:
    with state.lock:
        state.calibration_run_id += 1
        state.calibration_mode = None
        state.z_tilt_manual_ready = False
    labels = {
        34: "# triangle emergency stop selected",
        35: "# z_tilt emergency stop selected",
    }
    log_line(log_file, labels.get(page, f"# calibration emergency stop selected from page {page}"))
    send_cmd(fd, "page 68", log_file)


def start_calibration_run(state: DemoState, mode: str) -> int:
    with state.lock:
        state.calibration_run_id += 1
        state.calibration_mode = mode
        state.z_tilt_manual_ready = False
        return state.calibration_run_id


def calibration_run_active(state: DemoState, run_id: int, mode: str) -> bool:
    with state.lock:
        return state.calibration_run_id == run_id and state.calibration_mode == mode


def send_commands(
    fd: int,
    commands: list[str],
    log_file: LogFile = None,
    *,
    delay: float = 0.03,
) -> None:
    for command in commands:
        send_cmd(fd, command, log_file)
        if delay > 0:
            time.sleep(delay)


def run_demo_triangle(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
    run_id: int,
) -> None:
    send_commands(fd, [
        "page 34",
        "q0.picc=79",
        "q1.picc=78",
        "q2.picc=78",
        "q3.picc=78",
    ], log_file)

    stages = [
        (2.0, ["heat_complete=1", "q1.picc=79", "vis t2,1"]),
        (2.0, ["home_complete=1", "q2.picc=79", "vis t3,1"]),
    ]
    for delay, stage_commands in stages:
        if not sleep_cancelable(delay, stop) or not calibration_run_active(state, run_id, "triangle"):
            return
        send_commands(fd, stage_commands, log_file)

    for component in ("q4", "q5", "q6"):
        for _ in range(4):
            if stop.is_set() or not calibration_run_active(state, run_id, "triangle"):
                return
            send_cmd(fd, f"{component}.picc=82", log_file)
            time.sleep(0.25)

    send_commands(fd, ["level_complete=1", "q3.picc=79", "vis t4,1"], log_file)
    if sleep_cancelable(2.0, stop) and calibration_run_active(state, run_id, "triangle"):
        send_calibration_page(fd, state, log_file)


def run_demo_bed_mesh(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
    run_id: int,
) -> None:
    send_commands(fd, [
        "page 36",
        "q0.picc=79",
        "q1.picc=78",
        "q2.picc=78",
        "q3.picc=78",
    ], log_file)

    stages = [
        (2.0, ["heat_complete=1"]),
        (2.0, ["q1.picc=79", "vis t2,1", "home_complete=1"]),
        (1.0, ["q2.picc=79", "vis t3,1"]),
    ]
    for delay, stage_commands in stages:
        if not sleep_cancelable(delay, stop) or not calibration_run_active(state, run_id, "bed_mesh"):
            return
        send_commands(fd, stage_commands, log_file)

    probe_order = [
        4, 4, 5, 5, 5, 6, 6, 7, 7, 8, 8, 8, 9, 9,
        15, 15, 15, 14, 14, 13, 13, 13, 12, 12, 11, 11, 10, 10, 10,
        16, 16, 17, 17, 17, 18, 18, 19, 19, 19, 20, 20, 21, 21, 21,
        27, 27, 26, 26, 25, 25, 25, 24, 24, 23, 23, 23, 22, 22,
        28, 28, 28, 29, 29, 30, 30, 31, 31, 31, 32, 32, 33, 33, 33,
        39, 39, 38, 38, 38, 37, 37, 36, 36, 36, 35, 35, 34, 34, 34,
    ]
    for component in probe_order:
        if stop.is_set() or not calibration_run_active(state, run_id, "bed_mesh"):
            return
        send_cmd(fd, f"q{component}.picc=80", log_file)
        time.sleep(0.08)

    with state.lock:
        state.z_max = 0.04
        state.z_min = -0.31
    send_commands(fd, ["q3.picc=79", "vis t4,1", "level_complete=1"], log_file)
    if sleep_cancelable(2.0, stop) and calibration_run_active(state, run_id, "bed_mesh"):
        send_calibration_page(fd, state, log_file)


def run_demo_shaper(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
    run_id: int,
) -> None:
    send_commands(fd, [
        "page 37",
        "q0.picc=79",
        "q1.picc=78",
        "q2.picc=78",
        "q3.picc=78",
        "q4.picc=109",
    ], log_file)

    stages = [
        (2.0, ["home_complete=1", "q1.picc=79", "q4.picc=110", "vis t2,1"]),
        (2.0, ["q2.picc=79", "q4.picc=111", "vis t3,1"]),
        (2.0, ["q3.picc=79", "q4.picc=109", "vis t4,1"]),
    ]
    for delay, stage_commands in stages:
        if not sleep_cancelable(delay, stop) or not calibration_run_active(state, run_id, "shaper"):
            return
        send_commands(fd, stage_commands, log_file)

    with state.lock:
        state.shaper_freq_x = 57.8
        state.shaper_freq_y = 73.2
    if sleep_cancelable(2.0, stop) and calibration_run_active(state, run_id, "shaper"):
        send_calibration_page(fd, state, log_file)


def run_demo_z_tilt(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
    run_id: int,
) -> None:
    send_commands(fd, [
        "page 35",
        "q0.picc=79",
        "q1.picc=78",
        "q2.picc=78",
        "q3.picc=78",
        "b12.picc=83",
        "b13.picc=83",
        "b14.picc=84",
    ], log_file)

    stages = [
        (2.0, ["heat_complete=1", "q1.picc=79", "vis t2,1"]),
        (2.0, ["home_complete=1", "q2.picc=79", "vis t3,1", "b15.picc2=84", "b16.picc2=84", "b17.picc2=84"]),
    ]
    for delay, stage_commands in stages:
        if not sleep_cancelable(delay, stop) or not calibration_run_active(state, run_id, "z_tilt"):
            return
        send_commands(fd, stage_commands, log_file)

    with state.lock:
        if state.calibration_run_id == run_id and state.calibration_mode == "z_tilt":
            state.z_tilt_manual_ready = True
    log_line(log_file, "# z_tilt manual controls enabled; waiting for OK")


def set_z_tilt_step(fd: int, state: DemoState, log_file: LogFile, step: float) -> None:
    frame_map = {
        0.02: ("b12.picc=83", "b13.picc=83", "b14.picc=84"),
        0.10: ("b12.picc=83", "b13.picc=84", "b14.picc=83"),
        0.20: ("b12.picc=84", "b13.picc=83", "b14.picc=83"),
    }
    with state.lock:
        state.z_tilt_step = step
    log_line(log_file, f"# z_tilt step selected: {step:.2f}")
    send_commands(fd, list(frame_map[step]), log_file)


def nudge_z_tilt(fd: int, state: DemoState, log_file: LogFile, direction: int) -> None:
    with state.lock:
        if not state.z_tilt_manual_ready:
            log_line(log_file, "# z_tilt nudge ignored before manual stage")
            return
        state.z_offset = round(state.z_offset + direction * state.z_tilt_step, 3)
        z_offset = state.z_offset
    log_line(log_file, f"# z_tilt offset nudged: {z_offset:.3f}")


def complete_z_tilt(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
) -> None:
    with state.lock:
        if state.calibration_mode != "z_tilt" or not state.z_tilt_manual_ready:
            log_line(log_file, "# z_tilt OK ignored before manual stage")
            return
        state.z_tilt_manual_ready = False
        run_id = state.calibration_run_id
    send_commands(fd, [
        "q3.picc=79",
        "b15.picc2=83",
        "b16.picc2=83",
        "b17.picc2=83",
        "level_complete=1",
        "vis t4,1",
    ], log_file)
    if sleep_cancelable(2.0, stop) and calibration_run_active(state, run_id, "z_tilt"):
        send_calibration_page(fd, state, log_file)
