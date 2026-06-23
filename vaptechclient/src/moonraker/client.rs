use std::time::Duration;

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::{sync::mpsc, time};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::{
    app::event::AppEvent,
    moonraker::{event::MoonrakerEvent, parser::parse_moonraker_message},
};

const SUBSCRIBE_ID: u64 = 1;

/// Read-only Moonraker websocket client.
///
/// Клиент сам отправляет только objects.subscribe. Ui/MoonrakerRequest сюда
/// пока не подключены, чтобы во время тестов не отправить G-code в принтер.
#[derive(Debug)]
pub struct MoonrakerClient {
    url: String,
    app_event_tx: mpsc::Sender<AppEvent>,
    reconnect_delay: Duration,
}

impl MoonrakerClient {
    pub fn new(url: impl Into<String>, app_event_tx: mpsc::Sender<AppEvent>) -> Self {
        Self {
            url: url.into(),
            app_event_tx,
            reconnect_delay: Duration::from_secs(2),
        }
    }

    pub async fn run(self) -> Result<()> {
        loop {
            if let Err(error) = self.run_once().await {
                tracing::warn!(?error, url = %self.url, "Moonraker websocket disconnected");
                self.send_moonraker_event(MoonrakerEvent::disconnected())
                    .await?;
            }

            time::sleep(self.reconnect_delay).await;
        }
    }

    async fn run_once(&self) -> Result<()> {
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

        while let Some(message) = websocket.next().await {
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

        Ok(())
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
                "toolhead": null
            }
        },
        "id": SUBSCRIBE_ID
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
    }
}
