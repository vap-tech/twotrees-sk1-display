"""Demo state model for the modular display host."""

from __future__ import annotations

from dataclasses import dataclass, field
import threading


FAN_NAMES = {
    0: "Model fan",
    1: "Auxiliary fan",
    2: "Case fan",
}


@dataclass
class FileEntry:
    name: str
    kind: str
    has_preview: bool = False


@dataclass
class HistoryEntry:
    name: str
    timestamp: str
    duration: str
    status: str = "completed"


@dataclass
class DemoState:
    lock: threading.Lock = field(default_factory=threading.Lock)
    init_lock: threading.Lock = field(default_factory=threading.Lock)
    init_running: bool = False
    last_display_init_signal: float = 0.0
    nozzle_current: int = 21
    bed_current: int = 23
    nozzle_target: int = 0
    bed_target: int = 0
    caselight_on: bool = False
    fan_values: list[int] = field(default_factory=lambda: [0, 0, 0])
    load_unload_temp: int = 240
    move_distance: int = 1
    homed_axes: set[str] = field(default_factory=set)
    x_position: float = 0.0
    y_position: float = 255.0
    z_position: float = 5.0
    move_warning_id: int = 0
    z_home_probe_error: bool = False
    wifi_signal_bars: int = 0
    ip_address: str = "192.168.0.29"
    wifi_networks: list[str] = field(
        default_factory=lambda: [
            "Lesnaya-7",
            "RT-GPON-0872",
            "RT-GPON-A690",
            "RT-GPON-5102",
        ]
    )
    nozzle_ramp_id: int = 0
    bed_ramp_id: int = 0
    load_unload_process_id: int = 0
    calibration_run_id: int = 0
    calibration_mode: str | None = None
    z_tilt_step: float = 0.02
    z_tilt_manual_ready: bool = False
    z_max: float = 0.04
    z_min: float = -0.31
    z_offset: float = 0.319
    tilt_tolerance: float = 0.0000
    shaper_freq_x: float = 57.8
    shaper_freq_y: float = 73.2
    files_path: str = "/"
    files_page: int = 0
    files_view: str = "local"
    files_root_entries: list[FileEntry] = field(
        default_factory=lambda: [
            FileEntry("sda", "folder"),
            FileEntry("support_ecran_two_...", "gcode", True),
            FileEntry("scraper_PLA_40m1s....", "gcode", True),
        ]
    )
    files_sda_entries: list[FileEntry] = field(
        default_factory=lambda: [
            FileEntry("SK1", "folder"),
            FileEntry("SK1 Machine Docume...", "folder"),
            FileEntry("scraper_PETG_8m11s...", "gcode", True),
            FileEntry("VASO GUFO mld_PETG...", "gcode", True),
            FileEntry("1_PLA_7h18m.gcode", "gcode", True),
        ]
    )
    history_entries: list[HistoryEntry] = field(
        default_factory=lambda: [
            HistoryEntry("..._PETG_37m30s.gcode", "2026-06-03 23:57:52\n", "40m48s", "cancelled"),
            HistoryEntry("...l_PETG_3h13m.gcode", "2026-06-03 14:21:52\n", "181m55s"),
            HistoryEntry("...er_PLA_40m1s.gcode", "2026-05-31 23:19:28\n", "44m8s"),
            HistoryEntry("...mld_PLA_9h2m.gcode", "2026-05-30 21:15:44\n", "478m11s"),
            HistoryEntry("...v3_PLA_4h43m.gcode", "2026-05-30 10:48:32\n", "285m22s"),
        ]
    )
    print_filename: str = "support_ecran_two_trees_SK1_PETG_37m30s.gcode"
    print_active: bool = False
    print_paused: bool = False
    print_progress: int = 0
    print_run_id: int = 0
    obico_refresh_id: int = 0


def shorten_file_name(name: str, limit: int = 22) -> str:
    if len(name) <= limit:
        return name
    return name[: max(0, limit - 3)] + "..."

