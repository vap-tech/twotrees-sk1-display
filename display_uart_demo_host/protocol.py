"""TJC/Nextion-like UART protocol helpers for the SK1 display."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import re


FRAME_END = b"\xff\xff\xff"

STATUS_CODES = {
    0x1A: "status 0x1a, observed after valid thumbnail/page drawing commands too",
    0x1C: "status/error 0x1c, observed after invalid picture id",
}

WRITE_RE = re.compile(r'^cp([0-2])\.write\("(.+)"\)$')


@dataclass(frozen=True)
class TouchEvent:
    page: int
    component: int


@dataclass(frozen=True)
class NumericEvent:
    page: int
    component: int
    value: int


def encode_command(command: str) -> bytes:
    return command.encode("ascii") + FRAME_END


def hex_bytes(data: bytes | bytearray) -> str:
    return " ".join(f"{byte:02x}" for byte in data)


def scoped_component(scope: str | None, component: str) -> str:
    return f"{scope}.{component}" if scope else component


def quote_tjc_text(text: str) -> str:
    return text.replace("\\", "\\\\").replace('"', '\\"')


def split_frames(buffer: bytearray) -> list[bytes]:
    frames: list[bytes] = []
    while True:
        index = buffer.find(FRAME_END)
        if index < 0:
            return frames
        frame = bytes(buffer[: index + len(FRAME_END)])
        del buffer[: index + len(FRAME_END)]
        frames.append(frame)


def iter_hex_log_frames(path: Path) -> list[bytes]:
    if not path.exists():
        return []

    data: list[int] = []
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if not re.match(r"^\d\d:\d\d:\d\d\s+", line):
            continue
        for match in re.finditer(r"\b[0-9a-fA-F]{2}\b", line[9:]):
            data.append(int(match.group(0), 16))

    frames: list[bytes] = []
    current: list[int] = []
    index = 0
    while index < len(data):
        if index + 2 < len(data) and data[index : index + 3] == [0xFF, 0xFF, 0xFF]:
            frames.append(bytes(current))
            current = []
            index += 3
        else:
            current.append(data[index])
            index += 1
    return frames


def decode_frame(frame: bytes) -> str:
    payload = frame[:-3] if frame.endswith(FRAME_END) else frame
    hex_text = hex_bytes(frame)
    if not payload:
        return f"{hex_text} empty"
    if len(payload) == 1 and payload[0] in STATUS_CODES:
        return f"{hex_text} {STATUS_CODES[payload[0]]}"
    if payload and all(byte == 0x91 for byte in payload):
        return f"{hex_text} display-init-signal"
    if payload[0] == 0x70:
        try:
            text = payload[1:].decode("ascii")
        except UnicodeDecodeError:
            return f"{hex_text} string raw"
        return f"{hex_text} string {text!r}"
    if payload[0] == 0x65 and len(payload) >= 3:
        return f"{hex_text} touch page={payload[1]} component={payload[2]}"
    if payload[0] == 0x71 and len(payload) >= 5:
        get_value = int.from_bytes(payload[1:5], "little")
        input_value = payload[3] | (payload[4] << 8)
        return (
            f"{hex_text} numeric get_value={get_value} "
            f"or input page={payload[1]} component={payload[2]} value={input_value}"
        )
    try:
        text = payload.decode("ascii")
    except UnicodeDecodeError:
        return f"{hex_text} raw"
    if all(char.isprintable() for char in text):
        return f"{hex_text} ascii {text!r}"
    return f"{hex_text} raw"


def parse_numeric_frame(frame: bytes) -> NumericEvent | None:
    payload = frame[:-3] if frame.endswith(FRAME_END) else frame
    if len(payload) >= 5 and payload[0] == 0x71:
        return NumericEvent(payload[1], payload[2], payload[3] | (payload[4] << 8))
    return None


def parse_touch_frame(frame: bytes) -> TouchEvent | None:
    payload = frame[:-3] if frame.endswith(FRAME_END) else frame
    if len(payload) >= 3 and payload[0] == 0x65:
        return TouchEvent(payload[1], payload[2])
    return None

