use anyhow::{Context, Result};
use serde_json::Value;

use crate::moonraker::event::{FanKind, HeaterKind, KlippyState, MoonrakerEvent, PrinterStatus};

/// Разбирает одно JSON-RPC сообщение Moonraker в набор доменных событий.
///
/// Один websocket пакет может обновлять сразу несколько объектов, поэтому
/// parser возвращает Vec<MoonrakerEvent>.
pub fn parse_moonraker_message(raw: &str) -> Result<Vec<MoonrakerEvent>> {
    let json: Value = serde_json::from_str(raw).context("failed to parse moonraker json")?;

    let mut events = Vec::new();

    if let Some(method) = json.get("method").and_then(Value::as_str) {
        match method {
            "notify_status_update" => parse_notify_status_update(&json, &mut events),
            "notify_gcode_response" => parse_gcode_response(&json, &mut events),
            _ => events.push(MoonrakerEvent::Unknown),
        }
    } else if let Some(result) = json.get("result") {
        parse_result(result, &mut events);
    } else {
        events.push(MoonrakerEvent::Unknown);
    }

    Ok(events)
}

fn parse_result(result: &Value, events: &mut Vec<MoonrakerEvent>) {
    if let Some(state) = result.get("klippy_state").and_then(Value::as_str) {
        events.push(MoonrakerEvent::klippy_state(KlippyState::from_moonraker(
            state,
        )));
    }

    if let Some(status) = result.get("status") {
        parse_status_object(status, events);
    }
}

fn parse_notify_status_update(json: &Value, events: &mut Vec<MoonrakerEvent>) {
    let Some(status) = json
        .get("params")
        .and_then(Value::as_array)
        .and_then(|params| params.first())
    else {
        events.push(MoonrakerEvent::Unknown);
        return;
    };

    parse_status_object(status, events);
}

fn parse_status_object(status: &Value, events: &mut Vec<MoonrakerEvent>) {
    parse_heater(status, "extruder", HeaterKind::Nozzle, events);
    parse_heater(status, "heater_bed", HeaterKind::Bed, events);
    parse_case_light(status, events);
    parse_fan(status, "fan", FanKind::Part, events);
    parse_fan(status, "fan_generic Side_fan", FanKind::Side, events);
    parse_fan(status, "fan_generic Filter_fan", FanKind::Filter, events);
    parse_print_stats(status, events);
    parse_print_progress(status, events);
}

fn parse_heater(status: &Value, key: &str, heater: HeaterKind, events: &mut Vec<MoonrakerEvent>) {
    let Some(obj) = status.get(key) else {
        return;
    };

    let current = obj
        .get("temperature")
        .and_then(Value::as_f64)
        .map(|v| v as f32);

    let target = obj.get("target").and_then(Value::as_f64).map(|v| v as f32);

    if current.is_some() || target.is_some() {
        events.push(MoonrakerEvent::HeaterUpdate {
            heater,
            current,
            target,
        });
    }
}

fn parse_case_light(status: &Value, events: &mut Vec<MoonrakerEvent>) {
    let Some(caselight) = status.get("output_pin caselight") else {
        return;
    };

    if let Some(value) = caselight.get("value").and_then(Value::as_f64) {
        events.push(MoonrakerEvent::CaseLightChanged(value > 0.5));
    }
}

fn parse_fan(status: &Value, key: &str, fan: FanKind, events: &mut Vec<MoonrakerEvent>) {
    let Some(obj) = status.get(key) else {
        return;
    };

    if let Some(speed) = obj.get("speed").and_then(Value::as_f64) {
        events.push(MoonrakerEvent::FanChanged {
            fan,
            percent: speed_to_percent(speed),
        });
    }
}

fn speed_to_percent(speed: f64) -> u8 {
    let percent = (speed * 100.0).round();

    if percent < 0.0 {
        0
    } else if percent > 100.0 {
        100
    } else {
        percent as u8
    }
}

fn parse_print_stats(status: &Value, events: &mut Vec<MoonrakerEvent>) {
    let Some(print_stats) = status.get("print_stats") else {
        return;
    };

    if let Some(state) = print_stats.get("state").and_then(Value::as_str) {
        events.push(MoonrakerEvent::printer_status(
            PrinterStatus::from_print_stats(state),
        ));
    }
}

fn parse_print_progress(status: &Value, events: &mut Vec<MoonrakerEvent>) {
    let filename = status
        .get("print_stats")
        .and_then(|v| v.get("filename"))
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty())
        .map(String::from);

    let elapsed_seconds = status
        .get("print_stats")
        .and_then(|v| v.get("print_duration"))
        .and_then(Value::as_f64)
        .map(|v| v.round() as u32);

    let progress_ratio = status
        .get("virtual_sdcard")
        .and_then(|v| v.get("progress"))
        .and_then(Value::as_f64)
        .or_else(|| {
            status
                .get("display_status")
                .and_then(|v| v.get("progress"))
                .and_then(Value::as_f64)
        });

    let progress_percent = progress_ratio.map(progress_ratio_to_percent);

    let remaining_seconds = match (elapsed_seconds, progress_ratio) {
        (Some(elapsed), Some(progress)) => estimate_remaining_seconds(elapsed, progress),
        _ => None,
    };

    if filename.is_some()
        || progress_percent.is_some()
        || elapsed_seconds.is_some()
        || remaining_seconds.is_some()
    {
        // Remaining считаем приблизительно по progress + elapsed. Это не команда
        // принтеру, только отображение ETA на HMI.
        events.push(MoonrakerEvent::PrintProgress {
            filename,
            progress_percent,
            elapsed_seconds,
            remaining_seconds,
        });
    }
}

fn progress_ratio_to_percent(progress: f64) -> u8 {
    let percent = (progress * 100.0).round();

    if percent < 0.0 {
        0
    } else if percent > 100.0 {
        100
    } else {
        percent as u8
    }
}

fn estimate_remaining_seconds(elapsed_seconds: u32, progress: f64) -> Option<u32> {
    if progress <= 0.0 || progress >= 1.0 {
        return None;
    }

    let total_estimated = elapsed_seconds as f64 / progress;
    let remaining = total_estimated - elapsed_seconds as f64;

    Some(remaining.max(0.0).round() as u32)
}

fn parse_gcode_response(json: &Value, events: &mut Vec<MoonrakerEvent>) {
    let Some(params) = json.get("params").and_then(Value::as_array) else {
        events.push(MoonrakerEvent::Unknown);
        return;
    };

    for value in params {
        if let Some(text) = value.as_str() {
            events.push(MoonrakerEvent::gcode_response(text));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_server_info_klippy_ready() {
        let raw = r#"{
            "result": {
                "klippy_connected": true,
                "klippy_state": "ready"
            }
        }"#;

        let events = parse_moonraker_message(raw).unwrap();

        assert_eq!(
            events,
            vec![MoonrakerEvent::klippy_state(KlippyState::Ready)]
        );
    }

    #[test]
    fn parses_initial_snapshot_result_status() {
        let raw = r#"{
            "jsonrpc": "2.0",
            "result": {
                "eventtime": 16724.305130724,
                "status": {
                    "print_stats": {
                        "filename": "Single Drawer_PLA_8h47m.gcode",
                        "state": "printing"
                    },
                    "extruder": {
                        "temperature": 209.91,
                        "target": 210.0
                    },
                    "heater_bed": {
                        "temperature": 50.0,
                        "target": 50.0
                    }
                }
            },
            "id": 1
        }"#;

        let events = parse_moonraker_message(raw).unwrap();

        assert_eq!(
            events,
            vec![
                MoonrakerEvent::HeaterUpdate {
                    heater: HeaterKind::Nozzle,
                    current: Some(209.91),
                    target: Some(210.0),
                },
                MoonrakerEvent::HeaterUpdate {
                    heater: HeaterKind::Bed,
                    current: Some(50.0),
                    target: Some(50.0),
                },
                MoonrakerEvent::printer_status(PrinterStatus::Printing),
                MoonrakerEvent::PrintProgress {
                    filename: Some("Single Drawer_PLA_8h47m.gcode".to_string()),
                    progress_percent: None,
                    elapsed_seconds: None,
                    remaining_seconds: None,
                },
            ]
        );
    }

    #[test]
    fn parses_extruder_partial_temperature_update() {
        let raw = r#"{
            "method": "notify_status_update",
            "params": [
                {
                    "extruder": {
                        "temperature": 209.91
                    }
                },
                16728.319308143
            ]
        }"#;

        let events = parse_moonraker_message(raw).unwrap();

        assert_eq!(
            events,
            vec![MoonrakerEvent::HeaterUpdate {
                heater: HeaterKind::Nozzle,
                current: Some(209.91),
                target: None,
            }]
        );
    }

    #[test]
    fn ignores_heater_power_only_update() {
        let raw = r#"{
            "method": "notify_status_update",
            "params": [
                {
                    "heater_bed": {
                        "power": 0.16303507782301946
                    }
                },
                16728.319308143
            ]
        }"#;

        let events = parse_moonraker_message(raw).unwrap();

        assert!(events.is_empty());
    }

    #[test]
    fn parses_bed_target_only_update() {
        let raw = r#"{
            "method": "notify_status_update",
            "params": [
                {
                    "heater_bed": {
                        "target": 60.0
                    }
                }
            ]
        }"#;

        let events = parse_moonraker_message(raw).unwrap();

        assert_eq!(
            events,
            vec![MoonrakerEvent::HeaterUpdate {
                heater: HeaterKind::Bed,
                current: None,
                target: Some(60.0),
            }]
        );
    }

    #[test]
    fn parses_case_light_status_update() {
        let raw = r#"{
            "method": "notify_status_update",
            "params": [
                {
                    "output_pin caselight": {
                        "value": 1.0
                    }
                }
            ]
        }"#;

        let events = parse_moonraker_message(raw).unwrap();

        assert_eq!(events, vec![MoonrakerEvent::CaseLightChanged(true)]);
    }

    #[test]
    fn parses_fan_status_update() {
        let raw = r#"{
            "method": "notify_status_update",
            "params": [
                {
                    "fan": {
                        "speed": 1.0
                    },
                    "fan_generic Side_fan": {
                        "speed": 0.42
                    },
                    "fan_generic Filter_fan": {
                        "speed": 0.0
                    }
                }
            ]
        }"#;

        let events = parse_moonraker_message(raw).unwrap();

        assert_eq!(
            events,
            vec![
                MoonrakerEvent::FanChanged {
                    fan: FanKind::Part,
                    percent: 100,
                },
                MoonrakerEvent::FanChanged {
                    fan: FanKind::Side,
                    percent: 42,
                },
                MoonrakerEvent::FanChanged {
                    fan: FanKind::Filter,
                    percent: 0,
                },
            ]
        );
    }

    #[test]
    fn parses_print_stats_state() {
        let raw = r#"{
            "method": "notify_status_update",
            "params": [
                {
                    "print_stats": {
                        "state": "printing"
                    }
                }
            ]
        }"#;

        let events = parse_moonraker_message(raw).unwrap();

        assert_eq!(
            events,
            vec![MoonrakerEvent::printer_status(PrinterStatus::Printing)]
        );
    }

    #[test]
    fn parses_print_stats_duration_only_update_as_elapsed_time() {
        let raw = r#"{
            "method": "notify_status_update",
            "params": [
                {
                    "print_stats": {
                        "total_duration": 9908.086828475,
                        "print_duration": 9706.923499712
                    }
                }
            ]
        }"#;

        let events = parse_moonraker_message(raw).unwrap();

        assert_eq!(
            events,
            vec![MoonrakerEvent::PrintProgress {
                filename: None,
                progress_percent: None,
                elapsed_seconds: Some(9707),
                remaining_seconds: None,
            }]
        );
    }

    #[test]
    fn parses_multiple_status_updates() {
        let raw = r#"{
            "method": "notify_status_update",
            "params": [
                {
                    "extruder": {
                        "temperature": 215.0,
                        "target": 220.0
                    },
                    "heater_bed": {
                        "temperature": 59.5,
                        "target": 60.0
                    },
                    "print_stats": {
                        "state": "printing"
                    }
                }
            ]
        }"#;

        let events = parse_moonraker_message(raw).unwrap();

        assert_eq!(
            events,
            vec![
                MoonrakerEvent::HeaterUpdate {
                    heater: HeaterKind::Nozzle,
                    current: Some(215.0),
                    target: Some(220.0),
                },
                MoonrakerEvent::HeaterUpdate {
                    heater: HeaterKind::Bed,
                    current: Some(59.5),
                    target: Some(60.0),
                },
                MoonrakerEvent::printer_status(PrinterStatus::Printing),
            ]
        );
    }

    #[test]
    fn parses_gcode_response() {
        let raw = r#"{
            "method": "notify_gcode_response",
            "params": ["ok"]
        }"#;

        let events = parse_moonraker_message(raw).unwrap();

        assert_eq!(events, vec![MoonrakerEvent::gcode_response("ok")]);
    }

    #[test]
    fn unknown_method_does_not_fail() {
        let raw = r#"{
            "method": "notify_proc_stat_update",
            "params": []
        }"#;

        let events = parse_moonraker_message(raw).unwrap();

        assert_eq!(events, vec![MoonrakerEvent::Unknown]);
    }

    #[test]
    fn invalid_json_returns_error() {
        let result = parse_moonraker_message("{ bad json");

        assert!(result.is_err());
    }
}
