//! Извлечение thumbnail из G-code и нормализация картинки под HMI target.
//!
//! Orca/Prusa кладут preview прямо в G-code блоком вида:
//! `; thumbnail begin ...`, затем base64 строки, затем `; thumbnail end`.
//! Здесь мы достаём готовый PNG/JPEG payload и приводим его к точному размеру
//! canvas-компонента. Конвертация в TJC/ColPic формат живёт отдельно в
//! `tjc_encoder`.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use image::RgbaImage;
use image::imageops::FilterType;

#[derive(Debug, Clone)]
struct ThumbnailBlock {
    area: u32,
    encoded: String,
}

pub fn extract_thumbnail_bytes_from_gcode_file(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    let path = path.as_ref();
    let gcode = fs::read_to_string(path)
        .with_context(|| format!("failed to read G-code file {}", path.display()))?;

    extract_thumbnail_bytes_from_gcode_str(&gcode)
}

pub fn extract_thumbnail_bytes_from_gcode_str(gcode: &str) -> Result<Vec<u8>> {
    let block = extract_thumbnail_blocks(gcode)
        .into_iter()
        .max_by_key(|block| block.area)
        .context("G-code does not contain Orca/Prusa thumbnail block")?;

    STANDARD
        .decode(block.encoded.as_bytes())
        .context("failed to decode thumbnail base64")
}

pub fn decode_resize_rgba(bytes: &[u8], width: u16, height: u16) -> Result<RgbaImage> {
    if width == 0 || height == 0 {
        bail!("thumbnail target size must be non-zero");
    }

    let image = image::load_from_memory(bytes).context("failed to decode thumbnail image")?;

    Ok(image
        .resize_exact(width as u32, height as u32, FilterType::Lanczos3)
        .to_rgba8())
}

fn extract_thumbnail_blocks(gcode: &str) -> Vec<ThumbnailBlock> {
    let mut blocks = Vec::new();
    let mut current: Option<ThumbnailBlock> = None;

    for line in gcode.lines() {
        let line = strip_gcode_comment_prefix(line);

        if let Some(area) = parse_thumbnail_begin_area(line) {
            current = Some(ThumbnailBlock {
                area,
                encoded: String::new(),
            });
            continue;
        }

        if line.starts_with("thumbnail end") {
            if let Some(block) = current.take() {
                if !block.encoded.is_empty() {
                    blocks.push(block);
                }
            }
            continue;
        }

        if let Some(block) = current.as_mut() {
            block.encoded.push_str(line.trim());
        }
    }

    blocks
}

fn strip_gcode_comment_prefix(line: &str) -> &str {
    line.trim_start()
        .strip_prefix(';')
        .unwrap_or(line)
        .trim_start()
}

fn parse_thumbnail_begin_area(line: &str) -> Option<u32> {
    let line = line.strip_prefix("thumbnail begin")?.trim();
    let size = line.split_whitespace().next()?;
    let (width, height) = size.split_once('x')?;
    let width = width.parse::<u32>().ok()?;
    let height = height.parse::<u32>().ok()?;

    Some(width.saturating_mul(height))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GCODE_FIXTURE: &str = include_str!("../../fixtures/thumbnails/orca_thumbnail.gcode");

    #[test]
    fn extracts_orca_thumbnail_from_gcode() {
        let bytes = extract_thumbnail_bytes_from_gcode_str(GCODE_FIXTURE).unwrap();

        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    }

    #[test]
    fn decodes_and_resizes_thumbnail_to_target() {
        let bytes = extract_thumbnail_bytes_from_gcode_str(GCODE_FIXTURE).unwrap();
        let rgba = decode_resize_rgba(&bytes, 155, 155).unwrap();

        assert_eq!(rgba.width(), 155);
        assert_eq!(rgba.height(), 155);
    }

    #[test]
    fn picks_largest_thumbnail_block() {
        let gcode = format!(
            "; thumbnail begin 1x1 1\n; {}\n; thumbnail end\n{}",
            "aW52YWxpZA==", GCODE_FIXTURE
        );

        let bytes = extract_thumbnail_bytes_from_gcode_str(&gcode).unwrap();

        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    }
}
