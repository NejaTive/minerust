use glam::IVec3;

use super::cell::{AIR, Cell, NEG_X, NEG_Z, X, Z};
use super::piece::{BlockBox, Facing, Piece, PieceKind, PieceWriter, bx};
use super::swamp_hut::average_ground;
use crate::block::Block;
use crate::stream::Stream;
use crate::terrain::Terrain;

const WIDTH: i32 = 21;
const HEIGHT: i32 = 15;
const DEPTH: i32 = 21;
const CENTER: i32 = 10;
const VAULT_FLOOR: i32 = -11;

pub(super) fn layout(terrain: &Terrain, anchor: (i32, i32), rng: &mut Stream) -> Vec<Piece> {
    let facing = Facing::ALL[rng.below(4) as usize];
    let min_x = anchor.0 - WIDTH / 2;
    let min_z = anchor.1 - DEPTH / 2;
    let ground = average_ground(terrain, min_x, min_z, WIDTH, DEPTH);
    let bounds = BlockBox::new(
        IVec3::new(min_x, ground, min_z),
        IVec3::new(min_x + WIDTH - 1, ground + HEIGHT - 1, min_z + DEPTH - 1),
    );
    vec![Piece {
        bounds,
        facing,
        kind: PieceKind::DesertTemple,
        salt: 0,
    }]
}

/// `DesertPyramid.addComponentParts`.
pub(super) fn place(w: &mut PieceWriter) {
    let sandstone = Cell::of(Block::Sandstone);
    let smooth = Cell::of(Block::SmoothSandstone);
    let chiseled = Cell::of(Block::ChiseledSandstone);
    let orange = Cell::of(Block::OrangeTerracotta);
    let blue = Cell::of(Block::BlueTerracotta);
    let toward_south = Block::SandstoneStairs.front(Z);
    let toward_north = Block::SandstoneStairs.front(NEG_Z);
    let toward_east = Block::SandstoneStairs.front(X);
    let toward_west = Block::SandstoneStairs.front(NEG_X);
    let (width, depth) = (WIDTH, DEPTH);

    w.fill(bx(0, -4, 0, width - 1, 0, depth - 1), sandstone);
    for i in 1..=9 {
        w.fill(bx(i, i, i, width - 1 - i, i, depth - 1 - i), sandstone);
        w.fill(bx(i + 1, i, i + 1, width - 2 - i, i, depth - 2 - i), AIR);
    }
    for x in 0..width {
        for z in 0..depth {
            w.fill_down(x, -5, z, Block::Sandstone);
        }
    }

    w.hollow(bx(0, 0, 0, 4, 9, 4), sandstone, AIR);
    w.fill(bx(1, 10, 1, 3, 10, 3), sandstone);
    w.put(2, 10, 0, toward_south);
    w.put(2, 10, 4, toward_north);
    w.put(0, 10, 2, toward_east);
    w.put(4, 10, 2, toward_west);
    w.hollow(bx(width - 5, 0, 0, width - 1, 9, 4), sandstone, AIR);
    w.fill(bx(width - 4, 10, 1, width - 2, 10, 3), sandstone);
    w.put(width - 3, 10, 0, toward_south);
    w.put(width - 3, 10, 4, toward_north);
    w.put(width - 5, 10, 2, toward_east);
    w.put(width - 1, 10, 2, toward_west);

    w.hollow(bx(8, 0, 0, 12, 4, 4), sandstone, AIR);
    w.fill(bx(9, 1, 0, 11, 3, 4), AIR);
    w.put(9, 1, 1, smooth);
    w.put(9, 2, 1, smooth);
    w.put(9, 3, 1, smooth);
    w.put(10, 3, 1, smooth);
    w.put(11, 3, 1, smooth);
    w.put(11, 2, 1, toward_south);
    w.put(11, 1, 1, toward_south);

    w.hollow(bx(4, 1, 1, 8, 3, 3), sandstone, AIR);
    w.fill(bx(4, 1, 2, 8, 2, 2), AIR);
    w.hollow(bx(12, 1, 1, 16, 3, 3), sandstone, AIR);
    w.fill(bx(12, 1, 2, 16, 2, 2), AIR);
    w.fill(bx(5, 4, 5, width - 6, 4, depth - 6), sandstone);
    w.fill(bx(9, 4, 9, 11, 4, 11), AIR);
    for (x, z) in [(8, 8), (12, 8), (8, 12), (12, 12)] {
        w.fill(bx(x, 1, z, x, 3, z), chiseled);
    }
    w.fill(bx(1, 1, 5, 4, 4, 11), sandstone);
    w.fill(bx(width - 5, 1, 5, width - 2, 4, 11), sandstone);
    w.fill(bx(6, 7, 9, 6, 7, 11), sandstone);
    w.fill(bx(width - 7, 7, 9, width - 7, 7, 11), sandstone);
    w.fill(bx(5, 5, 9, 5, 7, 11), smooth);
    w.fill(bx(width - 6, 5, 9, width - 6, 7, 11), smooth);
    for (x, y) in [
        (5, 5),
        (5, 6),
        (6, 6),
        (width - 6, 5),
        (width - 6, 6),
        (width - 7, 6),
    ] {
        w.put(x, y, 10, AIR);
    }
    w.fill(bx(2, 4, 4, 2, 6, 4), AIR);
    w.fill(bx(width - 3, 4, 4, width - 3, 6, 4), AIR);
    w.put(2, 4, 5, toward_south);
    w.put(2, 3, 4, toward_south);
    w.put(width - 3, 4, 5, toward_south);
    w.put(width - 3, 3, 4, toward_south);
    w.fill(bx(1, 1, 3, 2, 2, 3), sandstone);
    w.fill(bx(width - 3, 1, 3, width - 2, 2, 3), sandstone);
    w.put(1, 1, 2, sandstone);
    w.put(width - 2, 1, 2, sandstone);
    w.put(1, 2, 2, Block::SandstoneSlab);
    w.put(width - 2, 2, 2, Block::SandstoneSlab);
    w.put(2, 1, 2, toward_east);
    w.put(width - 3, 1, 2, toward_west);
    w.fill(bx(4, 3, 5, 4, 3, 18), sandstone);
    w.fill(bx(width - 5, 3, 5, width - 5, 3, 17), sandstone);
    w.fill(bx(3, 1, 5, 4, 2, 16), AIR);
    w.fill(bx(width - 6, 1, 5, width - 5, 2, 16), AIR);
    for z in (5..=17).step_by(2) {
        w.put(4, 1, z, smooth);
        w.put(4, 2, z, chiseled);
        w.put(width - 5, 1, z, smooth);
        w.put(width - 5, 2, z, chiseled);
    }

    for (x, z) in [
        (10, 7),
        (10, 8),
        (9, 9),
        (11, 9),
        (8, 10),
        (12, 10),
        (7, 10),
        (13, 10),
        (9, 11),
        (11, 11),
        (10, 12),
        (10, 13),
    ] {
        w.put(x, 0, z, orange);
    }
    w.put(CENTER, 0, CENTER, blue);

    for x in [0, width - 1] {
        w.put(x, 2, 1, sandstone);
        w.put(x, 2, 2, orange);
        w.put(x, 2, 3, sandstone);
        w.put(x, 3, 1, sandstone);
        w.put(x, 3, 2, orange);
        w.put(x, 3, 3, sandstone);
        w.put(x, 4, 1, orange);
        w.put(x, 4, 2, chiseled);
        w.put(x, 4, 3, orange);
        w.put(x, 5, 1, sandstone);
        w.put(x, 5, 2, orange);
        w.put(x, 5, 3, sandstone);
        w.put(x, 6, 1, orange);
        w.put(x, 6, 2, chiseled);
        w.put(x, 6, 3, orange);
        w.put(x, 7, 1, orange);
        w.put(x, 7, 2, orange);
        w.put(x, 7, 3, orange);
        w.put(x, 8, 1, sandstone);
        w.put(x, 8, 2, sandstone);
        w.put(x, 8, 3, sandstone);
    }
    for x in [2, width - 3] {
        w.put(x - 1, 2, 0, toward_west);
        w.put(x, 2, 0, sandstone);
        w.put(x + 1, 2, 0, toward_east);
        w.put(x - 1, 3, 0, toward_west);
        w.put(x, 3, 0, orange);
        w.put(x + 1, 3, 0, toward_east);
        w.put(x - 1, 4, 0, sandstone);
        w.put(x, 4, 0, chiseled);
        w.put(x + 1, 4, 0, sandstone);
        w.put(x - 1, 5, 0, orange);
        w.put(x, 5, 0, orange);
        w.put(x + 1, 5, 0, orange);
        w.put(x - 1, 6, 0, orange);
        w.put(x, 6, 0, chiseled);
        w.put(x + 1, 6, 0, orange);
        w.put(x - 1, 7, 0, sandstone);
        w.put(x, 7, 0, sandstone);
        w.put(x + 1, 7, 0, sandstone);
        w.put(x - 1, 8, 0, sandstone);
        w.put(x, 8, 0, sandstone);
        w.put(x + 1, 8, 0, sandstone);
    }
    w.fill(bx(8, 4, 0, 12, 6, 0), sandstone);
    w.put(8, 6, 0, AIR);
    w.put(12, 6, 0, AIR);
    w.put(9, 5, 0, orange);
    w.put(10, 5, 0, chiseled);
    w.put(11, 5, 0, orange);

    place_vault(w);
}

fn place_vault(w: &mut PieceWriter) {
    let smooth = Cell::of(Block::SmoothSandstone);
    let chiseled = Cell::of(Block::ChiseledSandstone);
    let sandstone = Cell::of(Block::Sandstone);

    w.fill(bx(8, -14, 8, 12, -11, 12), smooth);
    w.fill(bx(8, -10, 8, 12, -10, 12), chiseled);
    w.fill(bx(8, -9, 8, 12, -9, 12), smooth);
    w.fill(bx(8, -8, 8, 12, -1, 12), sandstone);
    w.fill(bx(9, VAULT_FLOOR, 9, 11, -1, 11), AIR);
    w.put(CENTER, VAULT_FLOOR, CENTER, Block::StonePressurePlate);
    w.fill(bx(9, -13, 9, 11, -13, 11), Block::Tnt);

    for (front, dx, dz) in [(X, -1, 0), (NEG_X, 1, 0), (Z, 0, -1), (NEG_Z, 0, 1)] {
        let (x, z) = (CENTER + 2 * dx, CENTER + 2 * dz);
        let (back_x, back_z) = (CENTER + 3 * dx, CENTER + 3 * dz);
        w.put(x, VAULT_FLOOR + 1, z, AIR);
        w.put(back_x, VAULT_FLOOR + 1, back_z, chiseled);
        w.put(back_x, VAULT_FLOOR, back_z, smooth);
        w.put(x, VAULT_FLOOR, z, Cell::chest(front));
    }
}
