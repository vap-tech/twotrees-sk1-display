use std::collections::VecDeque;
use std::io::{ErrorKind, Read, Write};
use std::time::Duration;

use anyhow::{Context, Result};
use tokio_serial::SerialPort;

use crate::hmi::command::HmiCommand;
use crate::hmi::frame::FrameBuffer;
use crate::hmi::transport::HmiTransport;

const DEFAULT_READ_TIMEOUT: Duration = Duration::from_millis(10);
const READ_BUFFER_SIZE: usize = 256;

pub struct SerialTransport {
    port: Box<dyn SerialPort>,
    frame_buffer: FrameBuffer,
    pending_frames: VecDeque<Vec<u8>>,
}

impl SerialTransport {
    pub fn open(path: impl AsRef<str>, baud: u32) -> Result<Self> {
        Self::open_with_timeout(path, baud, DEFAULT_READ_TIMEOUT)
    }

    pub fn open_with_timeout(path: impl AsRef<str>, baud: u32, timeout: Duration) -> Result<Self> {
        let path = path.as_ref();
        let port = tokio_serial::new(path, baud)
            .timeout(timeout)
            .open()
            .with_context(|| format!("failed to open HMI serial port {path} at {baud} baud"))?;

        Ok(Self::from_port(port))
    }

    pub fn from_port(port: Box<dyn SerialPort>) -> Self {
        Self {
            port,
            frame_buffer: FrameBuffer::new(),
            pending_frames: VecDeque::new(),
        }
    }

    fn read_available_frames(&mut self) -> Result<()> {
        let mut buffer = [0_u8; READ_BUFFER_SIZE];

        loop {
            match self.port.read(&mut buffer) {
                Ok(0) => return Ok(()),
                Ok(read_len) => {
                    self.pending_frames
                        .extend(self.frame_buffer.push_bytes(&buffer[..read_len]));
                }
                Err(error) if error.kind() == ErrorKind::TimedOut => return Ok(()),
                Err(error) if error.kind() == ErrorKind::WouldBlock => return Ok(()),
                Err(error) => return Err(error).context("failed to read HMI serial port"),
            }
        }
    }
}

impl HmiTransport for SerialTransport {
    fn send(&mut self, command: &HmiCommand) -> Result<()> {
        self.port
            .write_all(&command.to_bytes())
            .context("failed to write HMI command")?;
        self.port
            .flush()
            .context("failed to flush HMI serial port")?;
        Ok(())
    }

    fn recv_frame(&mut self) -> Result<Option<Vec<u8>>> {
        if let Some(frame) = self.pending_frames.pop_front() {
            return Ok(Some(frame));
        }

        self.read_available_frames()?;

        Ok(self.pending_frames.pop_front())
    }
}
