use std::time::Duration;

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::{sync::mpsc, time};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::{
    app::event::AppEvent,
    moonraker::{event::MoonrakerEvent, parser::parse_moonraker_message},
    ui::effect::MoonrakerRequest,
};

const SUBSCRIBE_ID: u64 = 1;
const FIRST_COMMAND_ID: u64 = 1000;
const HEARTBEAT_IDLE_TIMEOUT: Duration = Duration::from_secs(10);
const HEARTBEAT_RESPONSE_TIMEOUT: Duration = Duration::from_secs(2);

/// Moonraker websocket client.
///
/// По умолчанию клиент только подписывается на objects.subscribe. Запись в
/// Moonraker включается отдельным request_rx и сейчас ограничена подсветкой.
#[derive(Debug)]
pub struct MoonrakerClient {
    url: String,
    app_event_tx: mpsc::Sender<AppEvent>,
    request_rx: Option<mpsc::Receiver<MoonrakerRequest>>,
    reconnect_delay: Duration,
    next_command_id: u64,
}

impl MoonrakerClient {
    pub fn new(url: impl Into<String>, app_event_tx: mpsc::Sender<AppEvent>) -> Self {
        Self {
            url: url.into(),
            app_event_tx,
            request_rx: None,
            reconnect_delay: Duration::from_secs(2),
            next_command_id: FIRST_COMMAND_ID,
        }
    }

    pub fn with_request_rx(mut self, request_rx: mpsc::Receiver<MoonrakerRequest>) -> Self {
        self.request_rx = Some(request_rx);
        self
    }

    pub async fn run(mut self) -> Result<()> {
        loop {
            if let Err(error) = self.run_once().await {
                tracing::warn!(?error, url = %self.url, "Moonraker websocket disconnected");
                self.send_moonraker_event(MoonrakerEvent::disconnected())
                    .await?;
            }

            time::sleep(self.reconnect_delay).await;
        }
    }

    async fn run_once(&mut self) -> Result<()> {
        tracing::info!(url = %self.url, "connecting to Moonraker websocket");

        let (mut websocket, _) = connect_async(&self.url)
            .await
            .with_context(|| format!("failed to connect Moonraker websocket: {}", self.url))?;

        tracing::info!(url = %self.url, "Moonraker websocket connected");
        self.send_moonraker_event(MoonrakerEvent::connected())
            .await?;

        // Подписка read-only: просим Moonraker присылать изменения объектов,
        // которые нужны для экрана печати и температур.
        websocket
            .send(Message::Text(objects_subscribe_message()))
            .await
            .context("failed to send Moonraker object subscription")?;

        let mut last_rx = time::Instant::now();

        loop {
            let heartbeat_deadline = last_rx + HEARTBEAT_IDLE_TIMEOUT;

            tokio::select! {
                message = websocket.next() => {
                    let Some(message) = message else {
                        break;
                    };

                    let message = message.context("failed to read Moonraker websocket message")?;
                    last_rx = time::Instant::now();

                    if !self.handle_websocket_message(message).await? {
                        break;
                    }
                }

                request = recv_request(&mut self.request_rx), if self.request_rx.is_some() => {
                    let Some(request) = request else {
                        self.request_rx = None;
                        continue;
                    };

                    if let Some(message) = self.request_message(request) {
                        websocket
                            .send(Message::Text(message))
                            .await
                            .context("failed to send Moonraker request")?;
                    }
                }

                _ = time::sleep_until(heartbeat_deadline) => {
                    tracing::debug!(
                        idle_ms = last_rx.elapsed().as_millis(),
                        "Moonraker websocket idle; sending server.info heartbeat"
                    );

                    websocket
                        .send(Message::Text(self.server_info_message()))
                        .await
                        .context("failed to send Moonraker heartbeat")?;

                    let message = time::timeout(HEARTBEAT_RESPONSE_TIMEOUT, websocket.next())
                        .await
                        .context("Moonraker heartbeat timed out")?;

                    let Some(message) = message else {
                        break;
                    };

                    let message = message.context("failed to read Moonraker heartbeat response")?;
                    last_rx = time::Instant::now();

                    if !self.handle_websocket_message(message).await? {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_websocket_message(&self, message: Message) -> Result<bool> {
        match message {
            Message::Text(raw) => self.handle_raw_message(&raw).await?,
            Message::Binary(raw) => {
                let raw = String::from_utf8(raw).context("invalid UTF-8 Moonraker binary")?;
                self.handle_raw_message(&raw).await?;
            }
            Message::Ping(_) | Message::Pong(_) => {}
            Message::Close(frame) => {
                tracing::info!(?frame, "Moonraker websocket closed by peer");
                return Ok(false);
            }
            Message::Frame(_) => {}
        }

        Ok(true)
    }

    fn request_message(&mut self, request: MoonrakerRequest) -> Option<String> {
        match request {
            MoonrakerRequest::SetCaseLight(enabled) => {
                let id = self.next_command_id;
                self.next_command_id += 1;

                tracing::info!(enabled, "sending caselight command to Moonraker");

                Some(caselight_command_message(id, enabled))
            }

            MoonrakerRequest::SetPartFan(percent) => {
                let id = self.next_command_id;
                self.next_command_id += 1;

                tracing::info!(percent, "sending part fan command to Moonraker");

                Some(gcode_script_message(id, part_fan_gcode(percent)))
            }

            MoonrakerRequest::SetSideFan(percent) => {
                let id = self.next_command_id;
                self.next_command_id += 1;

                tracing::info!(percent, "sending side fan command to Moonraker");

                Some(gcode_script_message(
                    id,
                    generic_fan_gcode("Side_fan", percent),
                ))
            }

            MoonrakerRequest::SetFilterFan(percent) => {
                let id = self.next_command_id;
                self.next_command_id += 1;

                tracing::info!(percent, "sending filter fan command to Moonraker");

                Some(gcode_script_message(
                    id,
                    generic_fan_gcode("Filter_fan", percent),
                ))
            }

            MoonrakerRequest::SetNozzleTarget(celsius) => {
                let id = self.next_command_id;
                self.next_command_id += 1;

                tracing::info!(celsius, "sending nozzle target command to Moonraker");

                Some(gcode_script_message(id, format!("M104 S{celsius}")))
            }

            MoonrakerRequest::SetBedTarget(celsius) => {
                let id = self.next_command_id;
                self.next_command_id += 1;

                tracing::info!(celsius, "sending bed target command to Moonraker");

                Some(gcode_script_message(id, format!("M140 S{celsius}")))
            }

            MoonrakerRequest::PausePrint => {
                let id = self.next_command_id;
                self.next_command_id += 1;

                tracing::info!("sending pause print command to Moonraker");

                Some(jsonrpc_method_message(id, "printer.print.pause"))
            }

            MoonrakerRequest::ResumePrint => {
                let id = self.next_command_id;
                self.next_command_id += 1;

                tracing::info!("sending resume print command to Moonraker");

                Some(jsonrpc_method_message(id, "printer.print.resume"))
            }

            MoonrakerRequest::ClearPrintResult => {
                let id = self.next_command_id;
                self.next_command_id += 1;

                tracing::info!("sending print result clear command to Moonraker");

                Some(gcode_script_message(id, "SDCARD_RESET_FILE"))
            }

            MoonrakerRequest::StartPrint { filename } => {
                let id = self.next_command_id;
                self.next_command_id += 1;

                tracing::info!(filename, "sending start print command to Moonraker");

                Some(start_print_message(id, filename))
            }

            other => {
                tracing::debug!(
                    ?other,
                    "Moonraker request dropped by client: command is not enabled"
                );
                None
            }
        }
    }

    fn server_info_message(&mut self) -> String {
        let id = self.next_command_id;
        self.next_command_id += 1;

        jsonrpc_method_message(id, "server.info")
    }

    async fn handle_raw_message(&self, raw: &str) -> Result<()> {
        tracing::trace!(raw, "Moonraker websocket message received");

        let events = parse_moonraker_message(raw)?;

        for event in events {
            tracing::debug!(?event, "Moonraker event parsed");
            // Дальше события идут тем же путем, что и HMI events: через Runtime
            // в AppRunner, reducer и renderer.
            self.send_moonraker_event(event).await?;
        }

        Ok(())
    }

    async fn send_moonraker_event(&self, event: MoonrakerEvent) -> Result<()> {
        self.app_event_tx
            .send(AppEvent::moonraker(event))
            .await
            .context("failed to send Moonraker event to app runtime")
    }
}

async fn recv_request(
    request_rx: &mut Option<mpsc::Receiver<MoonrakerRequest>>,
) -> Option<MoonrakerRequest> {
    match request_rx {
        Some(rx) => rx.recv().await,
        None => std::future::pending().await,
    }
}

fn objects_subscribe_message() -> String {
    json!({
        "jsonrpc": "2.0",
        "method": "printer.objects.subscribe",
        "params": {
            "objects": {
                "print_stats": null,
                "virtual_sdcard": null,
                "extruder": null,
                "heater_bed": null,
                "toolhead": null,
                "output_pin caselight": null,
                "fan": null,
                "fan_generic Side_fan": null,
                "fan_generic Filter_fan": null
            }
        },
        "id": SUBSCRIBE_ID
    })
    .to_string()
}

fn caselight_command_message(id: u64, enabled: bool) -> String {
    let value = if enabled { 1 } else { 0 };

    gcode_script_message(id, format!("SET_PIN PIN=caselight VALUE={value}"))
}

fn gcode_script_message(id: u64, script: impl Into<String>) -> String {
    json!({
        "jsonrpc": "2.0",
        "method": "printer.gcode.script",
        "params": {
            "script": script.into()
        },
        "id": id
    })
    .to_string()
}

fn jsonrpc_method_message(id: u64, method: &'static str) -> String {
    json!({
        "jsonrpc": "2.0",
        "method": method,
        "id": id
    })
    .to_string()
}

fn start_print_message(id: u64, filename: impl Into<String>) -> String {
    json!({
        "jsonrpc": "2.0",
        "method": "printer.print.start",
        "params": {
            "filename": filename.into()
        },
        "id": id
    })
    .to_string()
}

fn part_fan_gcode(percent: u8) -> String {
    let speed = ((percent.min(100) as u16 * 255) + 50) / 100;

    format!("M106 S{speed}")
}

fn generic_fan_gcode(name: &str, percent: u8) -> String {
    let speed = f32::from(percent.min(100)) / 100.0;

    format!("SET_FAN_SPEED FAN={name} SPEED={speed:.2}")
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    #[test]
    fn subscribe_message_is_read_only_objects_subscribe() {
        let message: Value = serde_json::from_str(&objects_subscribe_message()).unwrap();

        assert_eq!(message["jsonrpc"], "2.0");
        assert_eq!(message["method"], "printer.objects.subscribe");
        assert_eq!(message["id"], SUBSCRIBE_ID);
        assert!(message["params"]["objects"]["print_stats"].is_null());
        assert!(message["params"]["objects"]["virtual_sdcard"].is_null());
        assert!(message["params"]["objects"]["extruder"].is_null());
        assert!(message["params"]["objects"]["heater_bed"].is_null());
        assert!(message["params"]["objects"]["toolhead"].is_null());
        assert!(message["params"]["objects"]["output_pin caselight"].is_null());
        assert!(message["params"]["objects"]["fan"].is_null());
        assert!(message["params"]["objects"]["fan_generic Side_fan"].is_null());
        assert!(message["params"]["objects"]["fan_generic Filter_fan"].is_null());
    }

    #[test]
    fn caselight_command_uses_gcode_script() {
        let message: Value = serde_json::from_str(&caselight_command_message(42, true)).unwrap();

        assert_eq!(message["jsonrpc"], "2.0");
        assert_eq!(message["method"], "printer.gcode.script");
        assert_eq!(message["id"], 42);
        assert_eq!(message["params"]["script"], "SET_PIN PIN=caselight VALUE=1");

        let message: Value = serde_json::from_str(&caselight_command_message(43, false)).unwrap();
        assert_eq!(message["params"]["script"], "SET_PIN PIN=caselight VALUE=0");
    }

    #[test]
    fn print_control_commands_use_moonraker_print_methods() {
        let pause: Value =
            serde_json::from_str(&jsonrpc_method_message(42, "printer.print.pause")).unwrap();

        assert_eq!(pause["jsonrpc"], "2.0");
        assert_eq!(pause["method"], "printer.print.pause");
        assert_eq!(pause["id"], 42);
        assert!(pause.get("params").is_none());

        let resume: Value =
            serde_json::from_str(&jsonrpc_method_message(43, "printer.print.resume")).unwrap();

        assert_eq!(resume["method"], "printer.print.resume");
        assert_eq!(resume["id"], 43);
        assert!(resume.get("params").is_none());
    }

    #[test]
    fn clear_print_result_uses_sdcard_reset_file_gcode() {
        let message: Value =
            serde_json::from_str(&gcode_script_message(42, "SDCARD_RESET_FILE")).unwrap();

        assert_eq!(message["jsonrpc"], "2.0");
        assert_eq!(message["method"], "printer.gcode.script");
        assert_eq!(message["id"], 42);
        assert_eq!(message["params"]["script"], "SDCARD_RESET_FILE");
    }

    #[test]
    fn start_print_uses_moonraker_print_start_method() {
        let message: Value = serde_json::from_str(&start_print_message(42, "cube.gcode")).unwrap();

        assert_eq!(message["jsonrpc"], "2.0");
        assert_eq!(message["method"], "printer.print.start");
        assert_eq!(message["id"], 42);
        assert_eq!(message["params"]["filename"], "cube.gcode");
    }

    #[test]
    fn heartbeat_uses_read_only_server_info_method() {
        let message: Value =
            serde_json::from_str(&jsonrpc_method_message(42, "server.info")).unwrap();

        assert_eq!(message["jsonrpc"], "2.0");
        assert_eq!(message["method"], "server.info");
        assert_eq!(message["id"], 42);
        assert!(message.get("params").is_none());
    }
}
