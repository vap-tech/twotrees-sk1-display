"""Captured thumbnail payload helpers for the demo host."""

from __future__ import annotations

from pathlib import Path

from .protocol import WRITE_RE, iter_hex_log_frames


def load_demo_thumbnail_payloads() -> dict[int, list[str]]:
    capture_path = Path(__file__).resolve().parents[2] / "captures" / "display-ft232-manual-files.log"
    payloads: dict[int, list[str]] = {}
    active_component: int | None = None

    for frame in iter_hex_log_frames(capture_path):
        try:
            command = frame.decode("ascii")
        except UnicodeDecodeError:
            continue
        match = WRITE_RE.match(command)
        if match is None:
            active_component = None
            continue

        component = int(match.group(1))
        if component in payloads and active_component != component:
            continue

        payloads.setdefault(component, []).append(match.group(2))
        active_component = component
        if len(payloads) == 3 and all(payloads.get(index) for index in range(3)):
            if component == 2 and len(payloads[2]) >= 4:
                break

    return payloads
