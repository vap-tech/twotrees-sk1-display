use anyhow::Result;
use std::collections::VecDeque;

use crate::hmi::command::HmiCommand;
use crate::hmi::transport::HmiTransport;

#[derive(Debug, Default)]
pub struct MockTransport {
    pub sent: Vec<Vec<u8>>,
    incoming: VecDeque<Vec<u8>>,
}

impl MockTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_incoming_frame(&mut self, frame: Vec<u8>) {
        self.incoming.push_back(frame);
    }
}

impl HmiTransport for MockTransport {
    fn send(&mut self, command: &HmiCommand) -> Result<()> {
        self.sent.push(command.to_bytes());
        Ok(())
    }

    fn recv_frame(&mut self) -> Result<Option<Vec<u8>>> {
        Ok(self.incoming.pop_front())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_stores_bytes() {
        let mut transport = MockTransport::new();

        transport.send(&HmiCommand::page(33)).unwrap();

        assert_eq!(transport.sent[0], b"page 33\xFF\xFF\xFF");
    }

    #[test]
    fn recv_returns_incoming_frame() {
        let mut transport = MockTransport::new();

        transport.push_incoming_frame(vec![0x65, 33, 7, 0xFF, 0xFF, 0xFF]);

        assert_eq!(
            transport.recv_frame().unwrap(),
            Some(vec![0x65, 33, 7, 0xFF, 0xFF, 0xFF])
        );

        assert_eq!(transport.recv_frame().unwrap(), None);
    }
}
