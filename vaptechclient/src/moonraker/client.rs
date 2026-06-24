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

        loop {
            tokio::select! {
                message = websocket.next() => {
                    let Some(message) = message else {
                        break;
                    };

                    let message = message.context("failed to read Moonraker websocket message")?;

                    match message {
                        Message::Text(raw) => self.handle_raw_message(&raw).await?,
                        Message::Binary(raw) => {
                            let raw = String::from_utf8(raw).context("invalid UTF-8 Moonraker binary")?;
                            self.handle_raw_message(&raw).await?;
                        }
                        Message::Ping(_) | Message::Pong(_) => {}
                        Message::Close(frame) => {
                            tracing::info!(?frame, "Moonraker websocket closed by peer");
                            break;
                        }
                        Message::Frame(_) => {}
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
            }
        }

        Ok(())
    }

    fn request_message(&mut self, request: MoonrakerRequest) -> Option<String> {
        match request {
            MoonrakerRequest::SetCaseLight(enabled) => {
                let id = self.next_command_id;
                self.next_command_id += 1;

                tracing::info!(enabled, "sending caselight command to Moonraker");

                Some(caselight_command_message(id, enabled))
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
                "output_pin caselight": null
            }
        },
        "id": SUBSCRIBE_ID
    })
    .to_string()
}

fn caselight_command_message(id: u64, enabled: bool) -> String {
    let value = if enabled { 1 } else { 0 };

    json!({
        "jsonrpc": "2.0",
        "method": "printer.gcode.script",
        "params": {
            "script": format!("SET_PIN PIN=caselight VALUE={value}")
        },
        "id": id
    })
    .to_string()
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
}
