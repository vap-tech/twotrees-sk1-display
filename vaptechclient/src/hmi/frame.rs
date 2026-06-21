use anyhow::{bail, Result};

pub const TERMINATOR: [u8; 3] = [0xFF, 0xFF, 0xFF];

pub fn has_terminator(bytes: &[u8]) -> bool {
    bytes.ends_with(&TERMINATOR)
}

pub fn require_terminator(bytes: &[u8]) -> Result<()> {
    if has_terminator(bytes) {
        Ok(())
    } else {
        bail!("HMI frame has no FF FF FF terminator")
    }
}

pub fn payload_without_terminator(bytes: &[u8]) -> Result<&[u8]> {
    require_terminator(bytes)?;

    if bytes.len() < TERMINATOR.len() {
        bail!("HMI frame is too short")
    }

    Ok(&bytes[..bytes.len() - TERMINATOR.len()])
}

#[derive(Debug, Default)]
pub struct FrameBuffer {
    buffer: Vec<u8>,
}

impl FrameBuffer {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    pub fn push_byte(&mut self, byte: u8) -> Option<Vec<u8>> {
        self.buffer.push(byte);

        if has_terminator(&self.buffer) {
            let frame = self.buffer.clone();
            self.buffer.clear();
            Some(frame)
        } else {
            None
        }
    }

    pub fn push_bytes(&mut self, bytes: &[u8]) -> Vec<Vec<u8>> {
        let mut frames = Vec::new();

        for &byte in bytes {
            if let Some(frame) = self.push_byte(byte) {
                frames.push(frame);
            }
        }

        frames
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn pending_len(&self) -> usize {
        self.buffer.len()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_terminator() {
        assert!(has_terminator(b"\x65\x21\x07\xFF\xFF\xFF"));
        assert!(!has_terminator(b"\x65\x21\x07"));
    }

    #[test]
    fn require_terminator_accepts_valid_frame() {
        assert!(require_terminator(b"\x65\x21\x07\xFF\xFF\xFF").is_ok());
    }

    #[test]
    fn require_terminator_rejects_invalid_frame() {
        assert!(require_terminator(b"\x65\x21\x07").is_err());
    }

    #[test]
    fn strips_terminator() {
        let payload = payload_without_terminator(
            b"\x65\x21\x07\xFF\xFF\xFF"
        )
        .unwrap();

        assert_eq!(payload, b"\x65\x21\x07");
    }

    #[test]
    fn frame_buffer_returns_none_until_terminator() {
        let mut buffer = FrameBuffer::new();

        assert_eq!(buffer.push_byte(0x65), None);
        assert_eq!(buffer.push_byte(0x21), None);
        assert_eq!(buffer.push_byte(0x07), None);
        assert_eq!(buffer.push_byte(0xFF), None);
        assert_eq!(buffer.push_byte(0xFF), None);

        let frame = buffer.push_byte(0xFF);

        assert_eq!(
            frame,
            Some(vec![0x65, 0x21, 0x07, 0xFF, 0xFF, 0xFF])
        );
        assert!(buffer.is_empty());
    }

    #[test]
    fn frame_buffer_splits_multiple_frames() {
        let mut buffer = FrameBuffer::new();

        let frames = buffer.push_bytes(&[
            0x91, 0xFF, 0xFF, 0xFF,
            0x65, 33, 7, 0xFF, 0xFF, 0xFF,
        ]);

        assert_eq!(
            frames,
            vec![
                vec![0x91, 0xFF, 0xFF, 0xFF],
                vec![0x65, 33, 7, 0xFF, 0xFF, 0xFF],
            ]
        );
    }

    #[test]
    fn frame_buffer_keeps_partial_frame() {
        let mut buffer = FrameBuffer::new();

        let frames = buffer.push_bytes(&[0x65, 33]);

        assert!(frames.is_empty());
        assert_eq!(buffer.pending_len(), 2);
    }

    #[test]
    fn frame_buffer_clear_drops_partial_frame() {
        let mut buffer = FrameBuffer::new();

        buffer.push_bytes(&[0x65, 33]);
        buffer.clear();

        assert!(buffer.is_empty());
    }
}
