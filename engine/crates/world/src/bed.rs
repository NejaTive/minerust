use glam::IVec3;

use crate::block::{COLOR_COUNT, facing_code, facing_dir_of};
use crate::textures::DecodedImage;

pub const FACING_MASK: u8 = 0b11;
pub const HEAD: u8 = 0b100;
pub const COLOR_SHIFT: u32 = 3;

const SHEET_UNITS: f32 = 64.0;
const HEAD_TOP: (f32, f32) = (6.0, 6.0);
const FOOT_TOP: (f32, f32) = (6.0, 28.0);
const HEAD_SIDE: (f32, f32) = (0.0, 6.0);
const FOOT_SIDE: (f32, f32) = (0.0, 28.0);
const SIDE_DEPTH: f32 = 6.0;
const SIDE_TOP_ROW: f32 = 7.0;

/// `toward_head` — направление от ножной половины к изголовью.
pub fn data(toward_head: IVec3, head: bool, color: u8) -> u8 {
    facing_code(toward_head) | if head { HEAD } else { 0 } | (color % COLOR_COUNT) << COLOR_SHIFT
}

pub fn toward_head(data: u8) -> IVec3 {
    facing_dir_of(data & FACING_MASK)
}

pub fn is_head(data: u8) -> bool {
    data & HEAD != 0
}

pub fn color_of(data: u8) -> u8 {
    (data >> COLOR_SHIFT) % COLOR_COUNT
}

/// Смещение к другой половине кровати.
pub fn partner_offset(data: u8) -> IVec3 {
    if is_head(data) {
        -toward_head(data)
    } else {
        toward_head(data)
    }
}

/// Верх кровати лежит в плитке так, что изголовье смотрит на север; остальные повороты делает мешер.
pub fn compose_top(head: bool, sheet: &DecodedImage) -> DecodedImage {
    compose(sheet, |u, v| {
        let (x, y) = if head { HEAD_TOP } else { FOOT_TOP };
        Some((x + u, y + v))
    })
}

/// Боковая полоса высотой 6/16: от верхней грани матраса вниз; выше и ниже полосы плитка повторяет её край.
pub fn compose_side(head: bool, sheet: &DecodedImage) -> DecodedImage {
    compose(sheet, |u, v| {
        let depth = (v - SIDE_TOP_ROW).clamp(0.01, SIDE_DEPTH - 0.01);
        let (x, y) = if head { HEAD_SIDE } else { FOOT_SIDE };
        Some((x + SIDE_DEPTH - depth, y + u))
    })
}

fn compose(sheet: &DecodedImage, at: impl Fn(f32, f32) -> Option<(f32, f32)>) -> DecodedImage {
    let scale = sheet.width as f32 / SHEET_UNITS;
    let size = (16.0 * scale).round().max(1.0) as u32;
    let unit = 16.0 / size as f32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    for py in 0..size {
        for px in 0..size {
            let (u, v) = ((px as f32 + 0.5) * unit, (py as f32 + 0.5) * unit);
            let Some((sx, sy)) = at(u, v) else { continue };
            let (x, y) = ((sx * scale) as i64, (sy * scale) as i64);
            if x < 0 || y < 0 || x >= i64::from(sheet.width) || y >= i64::from(sheet.height) {
                continue;
            }
            let from = ((y * i64::from(sheet.width) + x) * 4) as usize;
            let to = ((py * size + px) * 4) as usize;
            if let Some(pixel) = sheet.rgba.get(from..from + 4) {
                rgba[to..to + 4].copy_from_slice(pixel);
            }
        }
    }
    DecodedImage {
        width: size,
        height: size,
        rgba,
    }
}

#[cfg(test)]
mod tests;
