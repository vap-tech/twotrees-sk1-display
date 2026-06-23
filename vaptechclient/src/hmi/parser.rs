use anyhow::{Result, bail};

use crate::hmi::event::{HmiEvent, HmiStatus};
use crate::hmi::frame::payload_without_terminator;

/// Превращает сырой UART frame в событие приложения.
///
/// Важно: startup дисплея - только bare `0x91`. Вариант `91 ff ff ff`
/// специально не принимается, потому что реальный дисплей так не шлет init.
pub fn parse_frame(frame: &[u8]) -> Result<HmiEvent> {
    if frame == [0x91] {
        return Ok(HmiEvent::Startup);
    }

    let payload = payload_without_terminator(frame)?;

    if payload.is_empty() {
        bail!("HMI frame payload is empty");
    }

    match payload[0] {
        0x91 => bail!("startup frame must be bare 0x91 without terminator"),
        0x65 => parse_touch(payload),
        0x71 => parse_numeric(payload),
        0x70 => parse_text(payload),
        0x1A => parse_status(payload, HmiStatus::Ack),
        0x1C => parse_status(payload, HmiStatus::Error),
        code => Ok(HmiEvent::Status(HmiStatus::Code(code))),
    }
}

fn parse_touch(payload: &[u8]) -> Result<HmiEvent> {
    if payload.len() != 3 {
        bail!("invalid touch frame length: {}", payload.len());
    }

    Ok(HmiEvent::touch(payload[1], payload[2]))
}

fn parse_numeric(payload: &[u8]) -> Result<HmiEvent> {
    if payload.len() != 5 {
        bail!("invalid numeric frame length: {}", payload.len());
    }

    // Numeric ответы HMI приходят little-endian после кода 0x71.
    let value = u32::from_le_bytes([payload[1], payload[2], payload[3], payload[4]]);

    Ok(HmiEvent::numeric(value))
}

fn parse_text(payload: &[u8]) -> Result<HmiEvent> {
    if payload.len() < 1 {
        bail!("invalid text frame length: {}", payload.len());
    }

    let text_bytes = &payload[1..];

    let text = String::from_utf8(text_bytes.to_vec())?;

    Ok(HmiEvent::text(text))
}

fn parse_status(payload: &[u8], status: HmiStatus) -> Result<HmiEvent> {
    if payload.len() != 1 {
        bail!("invalid status frame length: {}", payload.len());
    }

    Ok(HmiEvent::Status(status))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bare_startup_frame() {
        let event = parse_frame(&[0x91]).unwrap();

        assert_eq!(event, HmiEvent::Startup);
    }

    #[test]
    fn reject_terminated_startup_frame() {
        let result = parse_frame(&[0x91, 0xFF, 0xFF, 0xFF]);

        assert!(result.is_err());
    }

    #[test]
    fn parse_touch_frame() {
        let event = parse_frame(&[0x65, 33, 7, 0xFF, 0xFF, 0xFF]).unwrap();

        assert_eq!(event, HmiEvent::touch(33, 7));
    }

    #[test]
    fn parse_numeric_frame() {
        let event = parse_frame(&[0x71, 0x78, 0x56, 0x34, 0x12, 0xFF, 0xFF, 0xFF]).unwrap();

        assert_eq!(event, HmiEvent::numeric(0x12345678));
    }

    #[test]
    fn parse_text_frame() {
        let event = parse_frame(b"\x70hello\xFF\xFF\xFF").unwrap();

        assert_eq!(event, HmiEvent::text("hello"));
    }

    #[test]
    fn parse_empty_text_frame() {
        let event = parse_frame(&[0x70, 0xFF, 0xFF, 0xFF]).unwrap();

        assert_eq!(event, HmiEvent::text(""));
    }

    #[test]
    fn parse_ack_status_frame() {
        let event = parse_frame(&[0x1A, 0xFF, 0xFF, 0xFF]).unwrap();

        assert_eq!(event, HmiEvent::Status(HmiStatus::Ack));
    }

    #[test]
    fn parse_error_status_frame() {
        let event = parse_frame(&[0x1C, 0xFF, 0xFF, 0xFF]).unwrap();

        assert_eq!(event, HmiEvent::Status(HmiStatus::Error));
    }

    #[test]
    fn parse_unknown_status_code() {
        let event = parse_frame(&[0x86, 0xFF, 0xFF, 0xFF]).unwrap();

        assert_eq!(event, HmiEvent::Status(HmiStatus::Code(0x86)));
    }

    #[test]
    fn reject_frame_without_terminator() {
        let result = parse_frame(&[0x65, 33, 7]);

        assert!(result.is_err());
    }

    #[test]
    fn reject_invalid_touch_length() {
        let result = parse_frame(&[0x65, 33, 7, 1, 0xFF, 0xFF, 0xFF]);

        assert!(result.is_err());
    }

    #[test]
    fn reject_invalid_numeric_length() {
        let result = parse_frame(&[0x71, 1, 2, 0xFF, 0xFF, 0xFF]);

        assert!(result.is_err());
    }

    #[test]
    fn reject_invalid_utf8_text() {
        let result = parse_frame(&[0x70, 0xFF, 0xFE, 0xFF, 0xFF, 0xFF]);

        assert!(result.is_err());
    }
}
