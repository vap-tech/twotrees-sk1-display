import asyncio
import json
from pathlib import Path

import websockets

URI = "ws://192.168.0.20:7125/websocket"
OUT = Path("moonraker_ws_samples.jsonl")

SUBSCRIBE = {
    "jsonrpc": "2.0",
    "method": "printer.objects.subscribe",
    "params": {
        "objects": {
            "print_stats": None,
            "extruder": None,
            "heater_bed": None,
            "display_status": None,
            "virtual_sdcard": None,
            "fan": None,
            "toolhead": None,
        }
    },
    "id": 1,
}


async def main():
    async with websockets.connect(URI) as ws:
        await ws.send(json.dumps(SUBSCRIBE))

        end = asyncio.get_event_loop().time() + 10

        with OUT.open("w", encoding="utf-8") as f:
            while asyncio.get_event_loop().time() < end:
                msg = await ws.recv()
                obj = json.loads(msg)

                if (
                    obj.get("method")
                    in {
                        "notify_status_update",
                        "notify_gcode_response",
                    }
                    or "result" in obj
                ):
                    f.write(json.dumps(obj, ensure_ascii=False) + "\n")


asyncio.run(main())
