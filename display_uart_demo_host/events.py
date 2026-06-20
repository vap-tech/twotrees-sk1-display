"""Touch/numeric event dispatcher for the modular demo host."""

from __future__ import annotations

import threading

from .pages.calibration import (
    complete_z_tilt,
    nudge_z_tilt,
    send_calibration_page,
    set_z_tilt_step,
    start_calibration_action,
    stop_calibration,
)
from .pages.fans import send_fan_page, set_fan_value
from .pages.files import (
    back_to_files_root,
    change_files_page,
    redraw_files_page,
    select_file_slot,
    select_local_files,
    select_usb_files,
)
from .pages.history import send_history_page
from .pages.home import send_home_state
from .pages.load_unload import (
    adjust_load_unload_temperature,
    send_load_unload_page,
    start_load_unload_process,
)
from .pages.move_temp import (
    DISTANCE_COMPONENTS,
    MOVE_COMPONENTS,
    handle_move_request,
    select_move_distance,
    send_move_temp_page,
    show_motor_unlock_alert,
    start_home_xy,
    start_home_z,
)
from .pages.network import send_network_page
from .pages.print import (
    pause_or_dialog_print,
    pause_print,
    resume_print,
    send_print_page,
    start_demo_print,
    start_stop_print,
)
from .pages.system import (
    cancel_obico_refresh,
    handle_info_pages,
    send_about_page,
    send_export_diary_page,
    send_factory_reset_page,
    send_lower_system_page,
    send_power_menu,
    send_system_page,
    start_demo_reboot,
    start_obico_page,
)
from .protocol import NumericEvent, TouchEvent
from .state import DemoState
from .temperature import handle_temperature_numeric_event
from .uart import LogFile, log_line, send_cmd


NAV_BLOCKED_PAGES = {
    27,  # Print pause/stop dialog
    45,  # Starting
    51,  # Homing overlay
    56,  # Error page
    62,  # Network searching overlay
    67,  # Rebooting overlay
    68,  # Power/emergency menu
    73,  # Print stopping overlay
    74,  # Print resume overlay
    77,  # Print result
}

NAV_NAMES = {
    0: "home",
    1: "print-settings",
    2: "files-local",
    3: "printer-settings",
    4: "info-faq",
}


def handle_touch_event(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
    event: TouchEvent,
) -> None:
    page = event.page
    component = event.component

    if page == 78 and component != 6:
        cancel_obico_refresh(state)

    if page not in NAV_BLOCKED_PAGES and component in NAV_NAMES:
        log_line(log_file, f"# nav button: {NAV_NAMES[component]} from page {page}")
        if component == 0:
            send_home_state(fd, state, log_file)
        elif component == 1:
            send_move_temp_page(fd, state, log_file)
        elif component == 2:
            log_line(log_file, "# files local selected")
            select_local_files(fd, state, log_file)
        elif component == 3:
            send_system_page(fd, log_file)
        elif component == 4:
            send_cmd(fd, "page 21", log_file)
        return

    if handle_info_pages(fd, log_file, page, component):
        return

    if page == 0 and component == 5:
        with state.lock:
            state.caselight_on = not state.caselight_on
            enabled = state.caselight_on
            pic = 3 if enabled else 2
        log_line(log_file, f"# caselight toggle: {'on' if enabled else 'off'}")
        send_cmd(fd, f"b5.picc={pic}", log_file)
        send_cmd(fd, f"b5.picc2={pic}", log_file)
        return

    if page in (0, 2) and component == 6:
        log_line(log_file, "# fan/screw page selected")
        send_fan_page(fd, state, log_file)
        return

    if page in (0, 2) and component == 9:
        log_line(log_file, "# wifi page selected")
        send_network_page(fd, state, log_file)
        return

    if page in (0, 2) and component == 10:
        log_line(log_file, "# power menu selected")
        send_power_menu(fd, log_file)
        return

    if page == 68 and component == 1:
        start_demo_reboot(fd, state, log_file, stop, send_home_state)
        return

    if page == 68 and component == 0:
        log_line(log_file, "# power off selected")
        return

    if page == 51:
        log_line(log_file, f"# emergency stop selected from homing overlay: component {component}")
        send_power_menu(fd, log_file)
        return

    if page == 2 and component == 5:
        pause_or_dialog_print(fd, state, log_file)
        return

    if page == 11 and component == 10:
        log_line(log_file, "# lower system page selected")
        send_lower_system_page(fd, log_file)
        return

    if page == 11 and component == 11:
        log_line(log_file, "# calibration page selected")
        send_calibration_page(fd, state, log_file)
        return

    if page == 11 and component == 6:
        log_line(log_file, "# network page selected from system")
        send_network_page(fd, state, log_file)
        return

    if page == 14 and component == 7:
        log_line(log_file, "# export diary selected")
        send_export_diary_page(fd, log_file)
        return

    if page == 14 and component == 8:
        log_line(log_file, "# about selected")
        send_about_page(fd, log_file)
        return

    if page == 14 and component == 9:
        start_obico_page(fd, state, log_file, stop)
        return

    if page == 14 and component == 10:
        log_line(log_file, "# factory reset selected")
        send_factory_reset_page(fd, log_file)
        return

    if page == 14 and component in (11, 13):
        log_line(log_file, "# upper system page selected")
        send_system_page(fd, log_file)
        return

    if page == 78 and component == 5:
        log_line(log_file, "# Obico back selected")
        send_lower_system_page(fd, log_file)
        return

    if page == 78 and component == 6:
        log_line(log_file, "# Obico relink requested: _OBICO_RELINK")
        return

    if page == 18 and component == 5:
        log_line(log_file, "# system page selected from network")
        send_system_page(fd, log_file)
        return

    if page == 18 and component == 13:
        log_line(log_file, "# calibration page selected from network")
        send_calibration_page(fd, state, log_file)
        return

    if page == 33 and component == 5:
        log_line(log_file, "# system settings page selected from calibration")
        send_system_page(fd, log_file)
        return

    if page == 33 and component == 6:
        log_line(log_file, "# network page selected from calibration")
        send_network_page(fd, state, log_file)
        return

    if page == 33 and component in (8, 9, 10, 11):
        start_calibration_action(fd, state, log_file, stop, component)
        return

    if page == 34 and component == 12:
        stop_calibration(fd, state, log_file, page)
        return

    if page == 35 and component == 19:
        stop_calibration(fd, state, log_file, page)
        return

    if page in (36, 37) and component == 13:
        stop_calibration(fd, state, log_file, page)
        return

    if page == 35 and component in (12, 13, 14):
        step_by_component = {
            12: 0.20,
            13: 0.10,
            14: 0.02,
        }
        set_z_tilt_step(fd, state, log_file, step_by_component[component])
        return

    if page == 35 and component in (15, 17):
        nudge_z_tilt(fd, state, log_file, 1 if component == 15 else -1)
        return

    if page == 35 and component == 16:
        thread = threading.Thread(
            target=complete_z_tilt,
            args=(fd, state, log_file, stop),
            daemon=True,
        )
        thread.start()
        return

    if page == 7 and component == 6:
        log_line(log_file, "# files usb/sda tab selected")
        select_usb_files(fd, state, log_file)
        return

    if page == 54 and component == 5:
        log_line(log_file, "# usb files local tab selected")
        select_local_files(fd, state, log_file)
        return

    if page == 10 and component == 5:
        log_line(log_file, "# history local tab selected")
        select_local_files(fd, state, log_file)
        return

    if page == 10 and component == 6:
        log_line(log_file, "# history usb tab selected")
        select_usb_files(fd, state, log_file)
        return

    if page in (7, 54) and component == 7:
        log_line(log_file, "# files history tab selected")
        send_history_page(fd, state, log_file)
        return

    if page == 54 and component == 6:
        log_line(log_file, "# usb files tab reselected")
        select_usb_files(fd, state, log_file)
        return

    if page in (7, 54) and component in (8, 9, 10):
        select_file_slot(fd, state, log_file, component)
        return

    if page in (7, 54) and component in (11, 12):
        change_files_page(fd, state, log_file, component)
        return

    if page in (7, 54) and component == 13:
        back_to_files_root(fd, state, log_file)
        return

    if page == 9 and component == 5:
        log_line(log_file, "# print settings selected from preview")
        send_move_temp_page(fd, state, log_file)
        return

    if page == 9 and component == 6:
        log_line(log_file, "# preview back selected")
        redraw_files_page(fd, state, log_file)
        return

    if page == 9 and component == 7:
        start_demo_print(fd, state, log_file)
        return

    if page == 27 and component == 0:
        pause_print(fd, state, log_file)
        return

    if page == 27 and component == 1:
        start_stop_print(fd, state, log_file, stop)
        return

    if page == 27 and component == 2:
        log_line(log_file, "# print dialog back selected")
        send_print_page(fd, state, log_file)
        return

    if page == 74 and component == 0:
        resume_print(fd, state, log_file)
        return

    if page == 77 and component == 5:
        log_line(log_file, "# reprint selected")
        start_demo_print(fd, state, log_file)
        return

    if page == 77 and component == 6:
        log_line(log_file, "# print result back selected")
        send_home_state(fd, state, log_file)
        return

    if page == 6 and component == 5:
        log_line(log_file, "# move/temp tab selected")
        send_move_temp_page(fd, state, log_file)
        return

    if (page == 3 and component == 22) or (page == 4 and component == 11):
        log_line(log_file, "# fan tab selected")
        send_fan_page(fd, state, log_file)
        return

    if (page == 3 and component == 21) or (page == 6 and component == 6):
        log_line(log_file, "# load/unload tab selected")
        send_load_unload_page(fd, state, log_file)
        return

    if page == 4 and component == 9:
        log_line(log_file, "# move/temp tab selected")
        send_move_temp_page(fd, state, log_file)
        return

    if page == 4 and component in (7, 8):
        adjust_load_unload_temperature(
            fd,
            state,
            log_file,
            10 if component == 8 else -10,
        )
        return

    if page == 4 and component in (5, 6):
        start_load_unload_process(
            fd,
            state,
            log_file,
            stop,
            "load" if component == 5 else "unload",
        )
        return

    if page == 3 and component in DISTANCE_COMPONENTS:
        select_move_distance(fd, state, log_file, component)
        return

    if page == 3 and component in (16, 17):
        log_line(
            log_file,
            "# move/temp nozzle target input selected" if component == 16 else "# move/temp bed target input selected",
        )
        send_cmd(fd, "page 1", log_file)
        return

    if page == 3 and component in MOVE_COMPONENTS:
        handle_move_request(fd, state, log_file, stop, component)
        return

    if page == 3 and component == 18:
        show_motor_unlock_alert(fd, state, log_file, stop)
        return

    if page == 3 and component == 15:
        start_home_xy(fd, state, log_file, stop)
        return

    if page == 3 and component == 9:
        start_home_z(fd, state, log_file, stop)
        return

    log_line(log_file, f"# unsupported touch: page={page} component={component}")


def handle_numeric_event(
    fd: int,
    state: DemoState,
    log_file: LogFile,
    stop: threading.Event,
    event: NumericEvent,
) -> None:
    if event.page == 6 and event.component in (0, 1, 2):
        set_fan_value(fd, state, log_file, event.component, event.value)
        return

    if handle_temperature_numeric_event(fd, state, log_file, stop, event):
        return

    log_line(
        log_file,
        f"# unsupported numeric: page={event.page} component={event.component} value={event.value}",
    )
