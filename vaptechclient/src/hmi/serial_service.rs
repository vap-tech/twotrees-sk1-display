use anyhow::{Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf, split};
use tokio::sync::mpsc;
use tokio_serial::{SerialPortBuilderExt, SerialStream};

use crate::hmi::command::HmiCommand;
use crate::hmi::event::HmiEvent;
use crate::hmi::frame::FrameBuffer;
use crate::hmi::parser::parse_frame;

const READ_BUFFER_SIZE: usize = 256;
const WRITE_CHUNK_SIZE: usize = 512;

/// Async UART adapter для дисплея.
///
/// RX и TX разнесены в отдельные async task:
/// - reader быстро читает UART, режет frames и кладет HmiEvent в очередь;
/// - writer последовательно пишет HmiCommand в UART.
///
/// Это сохраняет отзывчивость touch-событий даже во время длинных cp.write().
pub struct HmiSerialService {
    port: SerialStream,
    command_rx: mpsc::Receiver<HmiCommand>,
    event_tx: mpsc::Sender<HmiEvent>,
}

impl HmiSerialService {
    pub fn open(
        path: impl AsRef<str>,
        baud: u32,
        command_rx: mpsc::Receiver<HmiCommand>,
        event_tx: mpsc::Sender<HmiEvent>,
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
        event_tx: mpsc::Sender<HmiEvent>,
    ) -> Self {
        Self {
            port,
            command_rx,
            event_tx,
        }
    }

    pub async fn run(self) -> Result<()> {
        let (reader, writer) = split(self.port);

        let mut reader_task = tokio::spawn(run_reader(reader, self.event_tx, FrameBuffer::new()));
        let mut writer_task = tokio::spawn(run_writer(writer, self.command_rx));

        tokio::select! {
            result = &mut reader_task => {
                writer_task.abort();
                result.context("HMI serial reader task panicked")??;
            }

            result = &mut writer_task => {
                reader_task.abort();
                result.context("HMI serial writer task panicked")??;
            }
        }

        Ok(())
    }
}

async fn run_reader(
    mut reader: ReadHalf<SerialStream>,
    event_tx: mpsc::Sender<HmiEvent>,
    mut frame_buffer: FrameBuffer,
) -> Result<()> {
    let mut read_buffer = [0_u8; READ_BUFFER_SIZE];

    loop {
        let read_len = reader
            .read(&mut read_buffer)
            .await
            .context("failed to read HMI serial port")?;

        if read_len == 0 {
            continue;
        }

        tracing::debug!(
            read_len,
            chunk = %format_hex(&read_buffer[..read_len]),
            "HMI serial bytes read"
        );

        for frame in frame_buffer.push_bytes(&read_buffer[..read_len]) {
            publish_frame(&event_tx, frame).await?;
        }

        if !frame_buffer.is_empty() {
            tracing::debug!(
                pending_len = frame_buffer.pending_len(),
                "HMI frame buffer has partial frame"
            );
        }
    }
}

async fn run_writer(
    mut writer: WriteHalf<SerialStream>,
    mut command_rx: mpsc::Receiver<HmiCommand>,
) -> Result<()> {
    while let Some(command) = command_rx.recv().await {
        tracing::debug!(%command, "HMI command queued for serial write");

        let bytes = command.to_bytes();

        for chunk in bytes.chunks(WRITE_CHUNK_SIZE) {
            writer
                .write_all(chunk)
                .await
                .context("failed to write HMI command chunk")?;

            tokio::task::yield_now().await;
        }

        writer
            .flush()
            .await
            .context("failed to flush HMI serial port")?;

        tracing::debug!(%command, "HMI command written");
    }

    Ok(())
}

async fn publish_frame(event_tx: &mpsc::Sender<HmiEvent>, frame: Vec<u8>) -> Result<()> {
    tracing::debug!(frame = %format_hex(&frame), "HMI frame received");

    let hmi_event = parse_frame(&frame).unwrap_or_else(|_| HmiEvent::unknown(frame));
    tracing::debug!(?hmi_event, "HMI event parsed");

    event_tx
        .send(hmi_event)
        .await
        .context("failed to publish HMI event")?;

    Ok(())
}

fn format_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}
