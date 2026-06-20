"""Move/temperature page renderer and small interactions."""

from __future__ import annotations

import threading
import time

from ..protocol import quote_tjc_text
from ..state import DemoState
from ..uart import LogFile, log_line, send_cmd


DISTANCE_COMPONENTS = {
    5: 1,
    6: 10,
    7: 30,
}

MOVE_COMPONENTS = {8, 10, 11, 12, 13, 14}


def send_move_temp_page(fd: int, state: DemoState, log_file: LogFile = None) -> None:
    with state.lock:
        nozzle_current = state.nozzle_current
        nozzle_target = state.nozzle_target
        bed_current = state.bed_current
        bed_target = state.bed_target
        distance = state.move_distance

    distance_pics = {
        1: (7, 6, 6),
        10: (6, 7, 6),
        30: (6, 6, 7),
    }
    b5_pic, b6_pic, b7_pic = distance_pics.get(distance, distance_pics[1])
    commands = [
        "page 3",
        "vis t2,0",
        f"b5.picc={b5_pic}",
        f"b6.picc={b6_pic}",
        f"b7.picc={b7_pic}",
        f"n3.val={nozzle_current}",
        f"n0.val={nozzle_target}",
        f"n2.val={bed_current}",
        f"n1.val={bed_target}",
    ]
    for command in commands:
        send_cmd(fd, command, log_file)
        time.sleep(0.03)


def select_move_distance(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    component: int,
) -> None:
    distance = DISTANCE_COMPONENTS[component]
    with state.lock:
        state.move_distance = distance
    log_line(log_file, f"# move distance selected: {distance}mm")
    send_move_temp_page(fd, state, log_file)


def show_move_alert(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
    warning_id: int,
    flag: int,
) -> None:
    send_cmd(fd, f"C3_send_flag={flag}", log_file)
    send_cmd(fd, "vis t2,1", log_file)
    end_time = time.monotonic() + 2
    while not stop.is_set() and time.monotonic() < end_time:
        time.sleep(0.1)
    with state.lock:
        if warning_id != state.move_warning_id:
            return
    send_cmd(fd, "vis t2,0", log_file)


def handle_move_request(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
    component: int,
) -> None:
    with state.lock:
        xy_components = {11, 12, 13, 14}
        required_axes = {"x", "y"} if component in xy_components else {"z"}
        homed = required_axes.issubset(state.homed_axes)
        required_axes_text = ",".join(sorted(required_axes))
        homed_axes_text = ",".join(sorted(state.homed_axes)) or "-"
        state.move_warning_id += 1
        warning_id = state.move_warning_id

    if not homed:
        log_line(
            log_file,
            f"# move rejected before homing: component {component} required={required_axes_text} homed={homed_axes_text}",
        )
        thread = threading.Thread(
            target=show_move_alert,
            args=(fd, state, log_file, stop, warning_id, 1),
            daemon=True,
        )
        thread.start()
        return

    with state.lock:
        distance = float(state.move_distance)
        deltas = {
            8: (0.0, 0.0, distance),
            10: (0.0, 0.0, -distance),
            11: (0.0, distance, 0.0),
            12: (0.0, -distance, 0.0),
            13: (-distance, 0.0, 0.0),
            14: (distance, 0.0, 0.0),
        }
        dx, dy, dz = deltas[component]
        attempted = (
            state.x_position + dx,
            state.y_position + dy,
            state.z_position + dz,
        )
        in_range = (
            0.0 <= attempted[0] <= 256.0
            and 0.0 <= attempted[1] <= 256.0
            and 0.0 <= attempted[2] <= 256.0
        )
        if in_range:
            state.x_position, state.y_position, state.z_position = attempted
            position_text = f"{state.x_position:.3f},{state.y_position:.3f},{state.z_position:.3f}"
        else:
            position_text = f"{attempted[0]:.3f},{attempted[1]:.3f},{attempted[2]:.3f}"

    if in_range:
        log_line(
            log_file,
            f"# move accepted: component {component} required={required_axes_text} homed={homed_axes_text} position={position_text}",
        )
        return

    log_line(log_file, f"# move out of range: component {component} attempted={position_text}")
    thread = threading.Thread(
        target=show_out_of_range,
        args=(fd, state, log_file, stop, attempted),
        daemon=True,
    )
    thread.start()


def reject_move_before_homing(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
    component: int,
) -> None:
    handle_move_request(fd, state, log_file, stop, component)


def show_motor_unlock_alert(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
) -> None:
    with state.lock:
        state.move_warning_id += 1
        warning_id = state.move_warning_id
    log_line(log_file, "# motors unlock requested")
    thread = threading.Thread(
        target=show_move_alert,
        args=(fd, state, log_file, stop, warning_id, 0),
        daemon=True,
    )
    thread.start()


def start_home_xy(fd: int, state: DemoState, log_file: LogFile, stop: threading.Event) -> None:
    log_line(log_file, "# home XY requested")
    thread = threading.Thread(
        target=run_demo_homing,
        args=(fd, state, log_file, stop, {"x", "y"}, 11.0),
        daemon=True,
    )
    thread.start()


def start_home_z(fd: int, state: DemoState, log_file: LogFile, stop: threading.Event) -> None:
    with state.lock:
        probe_error = state.z_home_probe_error
    log_line(log_file, "# home Z requested")
    if probe_error:
        target = run_demo_home_z_probe_error
        args = (fd, state, log_file, stop)
    else:
        target = run_demo_homing
        args = (fd, state, log_file, stop, {"x", "y", "z"}, 53.0)
    thread = threading.Thread(target=target, args=args, daemon=True)
    thread.start()


def show_error_page(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
    text: str,
    *,
    return_to_move_temp: bool = True,
) -> None:
    send_cmd(fd, "page 56", log_file)
    send_cmd(fd, f't0.txt="{quote_tjc_text(text)}"', log_file)
    end_time = time.monotonic() + 3
    while not stop.is_set() and time.monotonic() < end_time:
        time.sleep(0.1)
    if stop.is_set() or not return_to_move_temp:
        return
    send_move_temp_page(fd, state, log_file)


def show_out_of_range(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
    attempted_position: tuple[float, float, float],
) -> None:
    x_position, y_position, z_position = attempted_position
    show_error_page(
        fd,
        state,
        log_file,
        stop,
        f"!! Move out of range: {x_position:.3f} {y_position:.3f} {z_position:.3f} [0.000]",
    )


def run_demo_homing(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
    axes: set[str],
    duration: float,
) -> None:
    send_cmd(fd, "page 51", log_file)
    send_cmd(fd, "vis t2,0", log_file)
    end_time = time.monotonic() + duration
    while not stop.is_set() and time.monotonic() < end_time:
        time.sleep(0.1)
    if stop.is_set():
        return
    with state.lock:
        state.homed_axes.update(axes)
        if {"x", "y"}.issubset(axes):
            state.x_position = 0.0
            state.y_position = 255.0
        homed_axes = ",".join(sorted(state.homed_axes)) or "-"
    log_line(log_file, f"# homing complete: homed_axes={homed_axes}")
    send_move_temp_page(fd, state, log_file)


def run_demo_home_z_probe_error(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
) -> None:
    send_cmd(fd, "page 51", log_file)
    send_cmd(fd, "vis t2,0", log_file)
    end_time = time.monotonic() + 3
    while not stop.is_set() and time.monotonic() < end_time:
        time.sleep(0.1)
    if stop.is_set():
        return
    show_error_page(fd, state, log_file, stop, "!! Probe triggered prior to movement")
