use glam::IVec3;

use crate::block::facing_dir_of;
use crate::textures::DecodedImage;
use crate::tile::Tile;

pub const CHEST_SLOTS: usize = 27;
pub const FACING_MASK: u8 = 0b11;
pub const HALF_SHIFT: u32 = 2;
pub const HALF_MASK: u8 = 0b11;
pub const HALF_NONE: u8 = 0;
pub const HALF_LEFT: u8 = 1;
pub const HALF_RIGHT: u8 = 2;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Face {
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
}

pub const SINGLE_TILES: [(Face, Tile); 6] = [
    (Face::Front, Tile::ChestFront),
    (Face::Back, Tile::ChestBack),
    (Face::Left, Tile::ChestLeft),
    (Face::Right, Tile::ChestRight),
    (Face::Top, Tile::ChestTop),
    (Face::Bottom, Tile::ChestBottom),
];

/// `(плитка, граница, половина)`: половина 0 — левая во взгляде спереди, 1 — правая.
pub const DOUBLE_TILES: [(Tile, Face, u8); 10] = [
    (Tile::ChestDoubleFrontL, Face::Front, 0),
    (Tile::ChestDoubleFrontR, Face::Front, 1),
    (Tile::ChestDoubleBackL, Face::Back, 0),
    (Tile::ChestDoubleBackR, Face::Back, 1),
    (Tile::ChestDoubleTopL, Face::Top, 0),
    (Tile::ChestDoubleTopR, Face::Top, 1),
    (Tile::ChestDoubleBottomL, Face::Bottom, 0),
    (Tile::ChestDoubleBottomR, Face::Bottom, 1),
    (Tile::ChestDoubleEndL, Face::Left, 0),
    (Tile::ChestDoubleEndR, Face::Right, 1),
];

pub const ENTITY_SINGLE: &str = "entity/chest/normal.png";
pub const ENTITY_DOUBLE: &str = "entity/chest/normal_double.png";

pub fn data(front: IVec3, half: u8) -> u8 {
    crate::block::facing_code(front) | half << HALF_SHIFT
}

pub fn half_of(data: u8) -> u8 {
    data >> HALF_SHIFT & HALF_MASK
}

pub fn front_of(data: u8) -> IVec3 {
    facing_dir_of(data & FACING_MASK)
}

pub fn part(tile: Tile) -> (Face, Option<u8>) {
    if let Some(&(_, face, half)) = DOUBLE_TILES.iter().find(|&&(t, ..)| t == tile) {
        return (face, Some(half));
    }
    let face = SINGLE_TILES
        .iter()
        .find_map(|&(f, t)| (t == tile).then_some(f))
        .expect("chest tile");
    (face, None)
}

pub fn is_double_tile(tile: Tile) -> bool {
    DOUBLE_TILES.iter().any(|&(t, ..)| t == tile)
}

pub fn is_chest_tile(tile: Tile) -> bool {
    is_double_tile(tile) || SINGLE_TILES.iter().any(|&(_, t)| t == tile)
}

/// Вправо от взгляда на лицо сундука; `front` — куда смотрит лицо.
pub fn right_of(front: IVec3) -> IVec3 {
    IVec3::new(front.z, 0, -front.x)
}

pub fn face_of(front: IVec3, normal: IVec3) -> Face {
    let right = right_of(front);
    if normal == IVec3::Y {
        Face::Top
    } else if normal == IVec3::NEG_Y {
        Face::Bottom
    } else if normal == front {
        Face::Front
    } else if normal == -front {
        Face::Back
    } else if normal == right {
        Face::Right
    } else {
        Face::Left
    }
}

pub fn tile_for(data: u8, normal: IVec3) -> Tile {
    let face = face_of(facing_dir_of(data & FACING_MASK), normal);
    let half = data >> HALF_SHIFT & HALF_MASK;
    if half == HALF_NONE {
        return SINGLE_TILES
            .iter()
            .find_map(|&(f, tile)| (f == face).then_some(tile))
            .expect("every face has a tile");
    }
    let side = u8::from(half == HALF_RIGHT);
    let end = |f: Face| f == Face::Left || f == Face::Right;
    DOUBLE_TILES
        .iter()
        .find_map(|&(tile, f, s)| {
            let same = if end(face) {
                end(f) && s == side
            } else {
                f == face && s == side
            };
            same.then_some(tile)
        })
        .expect("every face has a tile")
}

#[derive(Clone, Copy)]
struct ModelBox {
    a: (u32, u32),
    y: (u32, u32),
    c: (u32, u32),
    tex: (u32, u32),
}

impl ModelBox {
    fn dims(self) -> (u32, u32, u32) {
        (
            self.a.1 - self.a.0,
            self.y.1 - self.y.0,
            self.c.1 - self.c.0,
        )
    }
}

const LATCH: usize = 2;

fn boxes(double: bool) -> [ModelBox; 3] {
    let (a0, a1, latch) = if double {
        (1, 31, (15, 17))
    } else {
        (1, 15, (7, 9))
    };
    [
        ModelBox {
            a: (a0, a1),
            y: (0, 10),
            c: (1, 15),
            tex: (0, 19),
        },
        ModelBox {
            a: (a0, a1),
            y: (9, 14),
            c: (1, 15),
            tex: (0, 0),
        },
        ModelBox {
            a: latch,
            y: (7, 11),
            c: (0, 1),
            tex: (0, 0),
        },
    ]
}

/// Текстура сущности в единицах 1/64 (одинарный) или 1/128 по ширине (двойной), высота 64.
struct Sheet<'a> {
    image: &'a DecodedImage,
    scale: f32,
}

impl Sheet<'_> {
    fn texel(&self, u: f32, v: f32) -> Option<[u8; 4]> {
        let (x, y) = ((u * self.scale) as i64, (v * self.scale) as i64);
        let (w, h) = (i64::from(self.image.width), i64::from(self.image.height));
        if u < 0.0 || v < 0.0 || x >= w || y >= h {
            return None;
        }
        let i = ((y * w + x) * 4) as usize;
        self.image
            .rgba
            .get(i..i + 4)
            .map(|p| [p[0], p[1], p[2], p[3]])
    }
}

/// Рисует грань сундука в плитку блока: пиксель плитки соответствует точке грани полного
/// блока, как у куба в `templates.rs`; `half` — какая половина двойного сундука.
pub fn compose(face: Face, half: Option<u8>, entity: &DecodedImage) -> DecodedImage {
    let double = half.is_some();
    let reference = if double { 128 } else { 64 };
    let scale = entity.width as f32 / reference as f32;
    let size = (16.0 * scale).round().max(1.0) as u32;
    let sheet = Sheet {
        image: entity,
        scale,
    };
    let offset = half.map_or(0.0, |h| 16.0 * f32::from(h));
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let unit = 16.0 / size as f32;
    for py in 0..size {
        for px in 0..size {
            let (u, v) = ((px as f32 + 0.5) * unit, (py as f32 + 0.5) * unit);
            let mut colour = None;
            for (index, b) in boxes(double).into_iter().enumerate() {
                let hidden_latch = index == LATCH && face == Face::Back;
                if let Some(c) = sample(&sheet, b, face, offset, u, v).filter(|_| !hidden_latch) {
                    colour = Some(c);
                }
            }
            if let Some(c) = colour.filter(|c| c[3] > 0) {
                let i = ((py * size + px) * 4) as usize;
                rgba[i..i + 4].copy_from_slice(&c);
            }
        }
    }
    DecodedImage {
        width: size,
        height: size,
        rgba,
    }
}

fn sample(sheet: &Sheet, b: ModelBox, face: Face, offset: f32, u: f32, v: f32) -> Option<[u8; 4]> {
    let (w, _, d) = b.dims();
    let (w, d) = (w as f32, d as f32);
    let (tu, tv) = (b.tex.0 as f32, b.tex.1 as f32);
    let (a0, a1) = (b.a.0 as f32, b.a.1 as f32);
    let (y0, y1) = (b.y.0 as f32, b.y.1 as f32);
    let (c0, c1) = (b.c.0 as f32, b.c.1 as f32);
    let within = |value: f32, lo: f32, hi: f32| value >= lo && value < hi;
    match face {
        Face::Front | Face::Back => {
            let local = if face == Face::Front { u } else { 16.0 - u };
            let a = offset + local;
            let y = 16.0 - v;
            if !within(a, a0, a1) || !within(y, y0, y1) {
                return None;
            }
            if face == Face::Front {
                sheet.texel(tu + d + (a - a0), tv + d + (y1 - y))
            } else {
                sheet.texel(tu + 2.0 * d + w + (a1 - a), tv + d + (y1 - y))
            }
        }
        Face::Left | Face::Right => {
            let c = if face == Face::Right { u } else { 16.0 - u };
            let y = 16.0 - v;
            let plane = if face == Face::Right { a1 } else { a0 };
            let in_half = if face == Face::Right {
                plane > offset && plane <= offset + 16.0
            } else {
                plane >= offset && plane < offset + 16.0
            };
            if !in_half || !within(c, c0, c1) || !within(y, y0, y1) {
                return None;
            }
            if face == Face::Right {
                sheet.texel(tu + d + w + (c - c0), tv + d + (y1 - y))
            } else {
                sheet.texel(tu + d - (c - c0), tv + d + (y1 - y))
            }
        }
        Face::Top | Face::Bottom => {
            let local = 16.0 - u;
            let a = offset + local;
            if !within(a, a0, a1) {
                return None;
            }
            if face == Face::Top {
                let c = v;
                if !within(c, c0, c1) {
                    return None;
                }
                sheet.texel(tu + d + (a - a0), tv + d - (c - c0))
            } else {
                let c = 16.0 - v;
                if !within(c, c0, c1) {
                    return None;
                }
                sheet.texel(tu + d + w + (a - a0), tv + (c - c0))
            }
        }
    }
}

#[cfg(test)]
mod tests;
