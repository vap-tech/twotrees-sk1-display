use crate::hmi::command::HmiCommand;
use crate::thumbnail::ThumbnailTarget;
use image::RgbaImage;
use std::collections::{BTreeMap, HashMap};

pub const ENCODER_VERSION: u8 = 1;
pub const DEFAULT_CHUNK_SIZE: usize = 1024;

const HEADER_SIZE: usize = 32;
const COLORS_MAX: usize = 1024;
// Vendor gene4.py отправляет в libColPic уже RGB-картинку. Прозрачные области
// штатно выглядят чёрными, поэтому alpha здесь композитим в black, а не в цвет
// страницы. Иначе вокруг Orca thumbnails появляется голубоватая подложка.
const BACKGROUND_RGB: (u8, u8, u8) = (0x00, 0x00, 0x00);
const COLPIC_MAGIC: u32 = 98_419_516;

#[derive(Debug, Clone, Copy)]
struct PaletteEntry {
    color16: u16,
    count: u32,
}

pub fn encode_image_to_tjc_chunks(image: &RgbaImage) -> Vec<String> {
    encode_image_to_tjc_chunks_with_size(image, DEFAULT_CHUNK_SIZE)
}

pub fn encode_image_to_tjc_chunks_with_size(image: &RgbaImage, chunk_size: usize) -> Vec<String> {
    let encoded = encode_image_to_colpic_string(image);

    encoded
        .as_bytes()
        .chunks(chunk_size.max(1))
        .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
        .collect()
}

fn encode_image_to_colpic_string(image: &RgbaImage) -> String {
    let pixels = rgba_to_rgb565_pixels(image);
    let raw = encode_colpic_raw(&pixels, image.width(), image.height());

    encode_nextion_base64(&raw)
}

fn rgba_to_rgb565_pixels(image: &RgbaImage) -> Vec<u16> {
    let (background_red, background_green, background_blue) = BACKGROUND_RGB;

    image
        .pixels()
        .map(|pixel| {
            let [red, green, blue, alpha] = pixel.0;
            let alpha = alpha as f32 / 255.0;
            let inv_alpha = 1.0 - alpha;

            let red = (red as f32 * alpha + background_red as f32 * inv_alpha) as u8;
            let green = (green as f32 * alpha + background_green as f32 * inv_alpha) as u8;
            let blue = (blue as f32 * alpha + background_blue as f32 * inv_alpha) as u8;

            rgb_to_rgb565(red, green, blue)
        })
        .collect()
}

fn rgb_to_rgb565(red: u8, green: u8, blue: u8) -> u16 {
    ((red as u16 >> 3) << 11) | ((green as u16 >> 2) << 5) | (blue as u16 >> 3)
}

fn encode_colpic_raw(pixels: &[u16], width: u32, height: u32) -> Vec<u8> {
    let (pixels, palette) = reduce_palette(pixels);
    let palette_size = palette.len() * 2;
    let mut raw = vec![0_u8; HEADER_SIZE + palette_size];

    raw[0] = 3;
    write_u32_le(&mut raw[4..8], width);
    write_u32_le(&mut raw[8..12], height);
    write_u32_le(&mut raw[12..16], COLPIC_MAGIC);
    write_u32_le(&mut raw[16..20], palette_size as u32);

    for (index, entry) in palette.iter().enumerate() {
        let offset = HEADER_SIZE + index * 2;
        raw[offset..offset + 2].copy_from_slice(&entry.color16.to_le_bytes());
    }

    let color_to_index = palette
        .iter()
        .enumerate()
        .map(|(index, entry)| (entry.color16, index as u16))
        .collect::<HashMap<_, _>>();
    let rle = encode_rle(&pixels, &color_to_index);

    write_u32_le(&mut raw[20..24], rle.len() as u32);
    raw.extend_from_slice(&rle);

    while raw.len() % 3 != 0 {
        raw.push(0);
    }

    raw
}

fn reduce_palette(pixels: &[u16]) -> (Vec<u16>, Vec<PaletteEntry>) {
    let mut counts = BTreeMap::<u16, u32>::new();
    for pixel in pixels {
        *counts.entry(*pixel).or_default() += 1;
    }

    let mut palette = counts
        .into_iter()
        .map(|(color16, count)| PaletteEntry { color16, count })
        .collect::<Vec<_>>();

    palette.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.color16.cmp(&right.color16))
    });

    if palette.len() <= COLORS_MAX {
        return (pixels.to_vec(), palette);
    }

    let keep = palette[..COLORS_MAX].to_vec();
    let discard = &palette[COLORS_MAX..];
    let mut remap = HashMap::<u16, u16>::with_capacity(discard.len());

    for entry in discard {
        let nearest = keep
            .iter()
            .min_by_key(|candidate| rgb565_distance(entry.color16, candidate.color16))
            .expect("kept palette is non-empty");
        remap.insert(entry.color16, nearest.color16);
    }

    let pixels = pixels
        .iter()
        .map(|pixel| remap.get(pixel).copied().unwrap_or(*pixel))
        .collect();

    (pixels, keep)
}

fn rgb565_distance(left: u16, right: u16) -> u16 {
    let left_red = ((left >> 11) & 0x1f) as i16;
    let left_green = ((left >> 5) & 0x3f) as i16;
    let left_blue = (left & 0x1f) as i16;

    let right_red = ((right >> 11) & 0x1f) as i16;
    let right_green = ((right >> 5) & 0x3f) as i16;
    let right_blue = (right & 0x1f) as i16;

    (left_red - right_red).unsigned_abs()
        + (left_green - right_green).unsigned_abs()
        + (left_blue - right_blue).unsigned_abs()
}

fn encode_rle(pixels: &[u16], color_to_index: &HashMap<u16, u16>) -> Vec<u8> {
    let mut output = Vec::new();
    let mut last_segment = 0_u16;
    let mut index = 0;

    while index < pixels.len() {
        let color = pixels[index];
        let mut run = 1_usize;

        while index + run < pixels.len() && pixels[index + run] == color && run < 255 {
            run += 1;
        }

        let mapped = color_to_index.get(&color).copied().unwrap_or(0);
        let segment = (mapped >> 5) & 0x1f;
        let entry = (mapped & 0x1f) as u8;

        if segment != last_segment {
            output.push(((7_u8) << 5) | segment as u8);
            last_segment = segment;
        }

        if run <= 6 {
            output.push(((run as u8) << 5) | entry);
        } else {
            output.push(entry);
            output.push(run as u8);
        }

        index += run;
    }

    output
}

fn encode_nextion_base64(raw: &[u8]) -> String {
    let mut output = String::with_capacity(raw.len() * 4 / 3 + 4);

    for chunk in raw.chunks(3) {
        let first = chunk[0];
        let second = *chunk.get(1).unwrap_or(&0);
        let third = *chunk.get(2).unwrap_or(&0);

        let values = [
            first >> 2,
            ((first & 0x03) << 4) | (second >> 4),
            ((second & 0x0f) << 2) | (third >> 6),
            third & 0x3f,
        ];

        for value in values {
            let byte = value + 48;
            output.push(if byte == b'\\' { '~' } else { byte as char });
        }
    }

    output
}

fn write_u32_le(target: &mut [u8], value: u32) {
    target.copy_from_slice(&value.to_le_bytes());
}

pub fn commands_from_chunks(target: &ThumbnailTarget, chunks: &[String]) -> Vec<HmiCommand> {
    let mut commands = Vec::with_capacity(chunks.len() + 2);

    commands.push(HmiCommand::raw(close_command(target)));
    commands.push(HmiCommand::raw(visible_command(target)));

    let write_component = write_component(target);

    for chunk in chunks {
        commands.push(HmiCommand::raw(format!(
            "{}.write(\"{}\")",
            write_component,
            escape_chunk(chunk)
        )));
    }

    commands
}

fn close_command(target: &ThumbnailTarget) -> String {
    match target {
        ThumbnailTarget::PrintPage => "Print_Trun_1.cp0.close()".to_string(),
        ThumbnailTarget::FileSlot { slot } => format!("cp{slot}.close()"),
        ThumbnailTarget::PreviewPage => "preview.cp0.close()".to_string(),
        ThumbnailTarget::ResultPage => "print_done.cp0.close()".to_string(),
    }
}

fn visible_command(target: &ThumbnailTarget) -> String {
    match target {
        ThumbnailTarget::PrintPage | ThumbnailTarget::PreviewPage => "vis cp0,1".to_string(),
        ThumbnailTarget::FileSlot { slot } => format!("vis cp{slot},1"),
        ThumbnailTarget::ResultPage => "vis print_done.cp0,1".to_string(),
    }
}

fn write_component(target: &ThumbnailTarget) -> String {
    match target {
        ThumbnailTarget::PrintPage => "cp0".to_string(),
        ThumbnailTarget::FileSlot { slot } => format!("cp{slot}"),
        ThumbnailTarget::PreviewPage => "preview.cp0".to_string(),
        ThumbnailTarget::ResultPage => "print_done.cp0".to_string(),
    }
}

fn escape_chunk(chunk: &str) -> String {
    chunk.replace('\\', r"\\").replace('"', "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thumbnail::decoder::{decode_resize_rgba, extract_thumbnail_bytes_from_gcode_str};

    const GCODE_FIXTURE: &str = include_str!("../../fixtures/thumbnails/orca_thumbnail.gcode");

    #[test]
    fn print_thumbnail_chunks_become_hmi_commands() {
        let commands = commands_from_chunks(
            &ThumbnailTarget::PrintPage,
            &["abc".to_string(), "def".to_string()],
        );

        assert_eq!(
            commands,
            vec![
                HmiCommand::raw("Print_Trun_1.cp0.close()"),
                HmiCommand::raw("vis cp0,1"),
                HmiCommand::raw("cp0.write(\"abc\")"),
                HmiCommand::raw("cp0.write(\"def\")"),
            ]
        );
    }

    #[test]
    fn file_slot_thumbnail_chunks_use_slot_component() {
        let commands =
            commands_from_chunks(&ThumbnailTarget::FileSlot { slot: 2 }, &["abc".to_string()]);

        assert_eq!(
            commands,
            vec![
                HmiCommand::raw("cp2.close()"),
                HmiCommand::raw("vis cp2,1"),
                HmiCommand::raw("cp2.write(\"abc\")"),
            ]
        );
    }

    #[test]
    fn result_thumbnail_chunks_use_print_done_component() {
        let commands = commands_from_chunks(&ThumbnailTarget::ResultPage, &["abc".to_string()]);

        assert_eq!(
            commands,
            vec![
                HmiCommand::raw("print_done.cp0.close()"),
                HmiCommand::raw("vis print_done.cp0,1"),
                HmiCommand::raw("print_done.cp0.write(\"abc\")"),
            ]
        );
    }

    #[test]
    fn encodes_known_thumbnail_to_tjc_chunks() {
        let bytes = extract_thumbnail_bytes_from_gcode_str(GCODE_FIXTURE).unwrap();
        let rgba = decode_resize_rgba(&bytes, 155, 155).unwrap();
        let chunks = encode_image_to_tjc_chunks(&rgba);

        assert!(!chunks.is_empty());
        assert!(chunks[0].starts_with("0`0009"));
    }

    #[test]
    fn tjc_chunks_use_requested_chunk_size() {
        let bytes = extract_thumbnail_bytes_from_gcode_str(GCODE_FIXTURE).unwrap();
        let rgba = decode_resize_rgba(&bytes, 155, 155).unwrap();
        let chunks = encode_image_to_tjc_chunks_with_size(&rgba, 64);

        assert!(chunks.len() > 1);
        assert!(chunks.iter().all(|chunk| chunk.len() <= 64));
    }
}
