use anyhow::{Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio_serial::{SerialPortBuilderExt, SerialStream};

use crate::app::event::AppEvent;
use crate::hmi::command::HmiCommand;
use crate::hmi::event::HmiEvent;
use crate::hmi::frame::FrameBuffer;
use crate::hmi::parser::parse_frame;

const READ_BUFFER_SIZE: usize = 256;

pub struct HmiSerialService {
    port: SerialStream,
    command_rx: mpsc::Receiver<HmiCommand>,
    event_tx: mpsc::Sender<AppEvent>,
    frame_buffer: FrameBuffer,
}

impl HmiSerialService {
    pub fn open(
        path: impl AsRef<str>,
        baud: u32,
        command_rx: mpsc::Receiver<HmiCommand>,
        event_tx: mpsc::Sender<AppEvent>,
    ) -> Result<Self> {
        let path = path.as_ref();
        let port = tokio_serial::new(path, baud)
            .open_native_async()
            .with_context(|| format!("failed to open HMI serial port {path} at {baud} baud"))?;

        Ok(Self::new(port, command_rx, event_tx))
    }

    pub fn new(
        port: SerialStream,
        command_rx: mpsc::Receiver<HmiCommand>,
        event_tx: mpsc::Sender<AppEvent>,
    ) -> Self {
        Self {
            port,
            command_rx,
            event_tx,
            frame_buffer: FrameBuffer::new(),
        }
    }

    pub async fn run(mut self) -> Result<()> {
        let mut read_buffer = [0_u8; READ_BUFFER_SIZE];

        loop {
            tokio::select! {
                read_result = self.port.read(&mut read_buffer) => {
                    let read_len = read_result.context("failed to read HMI serial port")?;

                    if read_len == 0 {
                        continue;
                    }

                    tracing::debug!(
                        read_len,
                        chunk = %format_hex(&read_buffer[..read_len]),
                        "HMI serial bytes read"
                    );

                    for frame in self.frame_buffer.push_bytes(&read_buffer[..read_len]) {
                        self.publish_frame(frame).await?;
                    }

                    if !self.frame_buffer.is_empty() {
                        tracing::debug!(
                            pending_len = self.frame_buffer.pending_len(),
                            "HMI frame buffer has partial frame"
                        );
                    }
                }

                command = self.command_rx.recv() => {
                    let Some(command) = command else {
                        return Ok(());
                    };

                    tracing::debug!(%command, "HMI command queued for serial write");

                    self.port
                        .write_all(&command.to_bytes())
                        .await
                        .context("failed to write HMI command")?;
                    self.port.flush().await.context("failed to flush HMI serial port")?;

                    tracing::debug!(%command, "HMI command written");
                }
            }
        }
    }

    async fn publish_frame(&self, frame: Vec<u8>) -> Result<()> {
        tracing::debug!(frame = %format_hex(&frame), "HMI frame received");

        let hmi_event = parse_frame(&frame).unwrap_or_else(|_| HmiEvent::unknown(frame));
        tracing::debug!(?hmi_event, "HMI event parsed");

        self.event_tx
            .send(AppEvent::hmi(hmi_event))
            .await
            .context("failed to publish HMI event")?;

        Ok(())
    }
}

fn format_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}
