use anyhow::Result;

use crate::hmi::command::HmiCommand;

pub trait HmiTransport {
    fn send(&mut self, command: &HmiCommand) -> Result<()>;
    fn recv_frame(&mut self) -> Result<Option<Vec<u8>>>;
}
