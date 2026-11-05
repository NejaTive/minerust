use glam::IVec3;

use super::cell::{AIR, Cell, NEG_X, NEG_Z, Selector, X, Z};
use super::piece::{BlockBox, Facing, Piece, PieceKind, PieceWriter, bx};
use super::swamp_hut::average_ground;
use crate::block::Block;
use crate::puzzle::MARKER;
use crate::stream::Stream;
use crate::terrain::Terrain;

const WIDTH: i32 = 12;
const HEIGHT: i32 = 10;
const DEPTH: i32 = 15;
const VINE_ONE_IN: u32 = 3;
const SALT_LIMIT: u32 = 1 << 24;

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
        kind: PieceKind::JungleTemple,
        salt: rng.below(SALT_LIMIT),
    }]
}

/// `JunglePyramid.addComponentParts` по памяти: сверка с 1.12 не проводилась.
pub(super) fn place(w: &mut PieceWriter) {
    let stones = Selector::new(&[
        (4, Cell::of(Block::Cobblestone)),
        (6, Cell::of(Block::MossyCobblestone)),
    ]);
    let stairs = |front| Block::CobblestoneStairs.front(front);

    w.fill_random(bx(0, -4, 0, 11, 0, 14), &stones);
    w.fill_random(bx(2, 1, 2, 9, 2, 2), &stones);
    w.fill_random(bx(2, 1, 12, 9, 2, 12), &stones);
    w.fill_random(bx(2, 1, 3, 2, 2, 11), &stones);
    w.fill_random(bx(9, 1, 3, 9, 2, 11), &stones);
    w.fill_random(bx(1, 3, 1, 10, 6, 1), &stones);
    w.fill_random(bx(1, 3, 13, 10, 6, 13), &stones);
    w.fill_random(bx(1, 3, 2, 1, 6, 12), &stones);
    w.fill_random(bx(10, 3, 2, 10, 6, 12), &stones);
    w.fill_random(bx(2, 3, 2, 9, 3, 12), &stones);
    w.fill_random(bx(2, 6, 2, 9, 6, 12), &stones);
    w.fill_random(bx(3, 7, 3, 8, 7, 11), &stones);
    w.fill_random(bx(4, 8, 4, 7, 8, 10), &stones);

    for area in [
        bx(3, 1, 3, 8, 2, 11),
        bx(4, 3, 6, 7, 3, 9),
        bx(2, 4, 2, 9, 5, 12),
        bx(4, 6, 5, 7, 6, 9),
        bx(5, 7, 6, 6, 7, 8),
        bx(5, 1, 0, 6, 2, 2),
        bx(5, 2, 12, 6, 2, 12),
        bx(5, 5, 1, 6, 5, 1),
        bx(5, 5, 13, 6, 5, 13),
    ] {
        w.fill(area, AIR);
    }
    for (x, z) in [(1, 5), (10, 5), (1, 9), (10, 9)] {
        w.put(x, 5, z, AIR);
    }

    for z in [0, 14] {
        for x in [2, 4, 7, 9] {
            w.fill_random(bx(x, 4, z, x, 5, z), &stones);
        }
    }
    w.fill_random(bx(5, 6, 0, 6, 6, 0), &stones);
    w.fill_random(bx(5, 6, 14, 6, 6, 14), &stones);
    for x in [0, 11] {
        for z in (2..=12).step_by(2) {
            w.fill_random(bx(x, 4, z, x, 5, z), &stones);
        }
        w.fill_random(bx(x, 2, 5, x, 2, 6), &stones);
        w.fill_random(bx(x, 2, 9, x, 2, 10), &stones);
    }

    for x in 4..=7 {
        w.put(x, 0, 0, stairs(Z));
    }
    w.put(5, 9, 6, stairs(Z));
    w.put(6, 9, 6, stairs(Z));
    w.fill_random(bx(5, 9, 7, 6, 9, 7), &stones);
    w.put(5, 9, 8, stairs(NEG_Z));
    w.put(6, 9, 8, stairs(NEG_Z));

    place_vines(w);
    place_basement(w);
}

fn place_vines(w: &mut PieceWriter) {
    let pillar_rows = 4..=5;
    for y in 3..=6 {
        for x in 1..=10 {
            let pillar = pillar_rows.contains(&y) && [2, 4, 7, 9].contains(&x);
            let lintel = y == 6 && (5..=6).contains(&x);
            if !pillar && !lintel {
                vine(w, x, y, 0, Z);
                vine(w, x, y, 14, NEG_Z);
            }
        }
        for z in 2..=12 {
            if !(pillar_rows.contains(&y) && z % 2 == 0) {
                vine(w, 0, y, z, X);
                vine(w, 11, y, z, NEG_X);
            }
        }
    }
}

fn vine(w: &mut PieceWriter, x: i32, y: i32, z: i32, toward_wall: IVec3) {
    if w.chance(x, y, z, VINE_ONE_IN) {
        w.put(x, y, z, Cell::vine(toward_wall));
    }
}

/// Лестница, комната с растяжкой и сундуком, тайник за дверью, которую открывают три рычага.
fn place_basement(w: &mut PieceWriter) {
    let cobble = Cell::of(Block::Cobblestone);
    let stairs = Block::CobblestoneStairs.front(NEG_Z);

    w.fill(bx(5, -2, 4, 6, 0, 6), AIR);
    for x in [5, 6] {
        for (y, z) in [(0, 3), (-1, 4), (-2, 5), (-3, 6)] {
            w.put(x, y, z, stairs);
        }
    }
    w.fill(bx(4, -3, 7, 7, -2, 13), AIR);

    w.put(3, -2, 8, MARKER);
    for z in 7..=9 {
        w.put(4, -2, z, Cell::lever(X, false));
    }
    w.fill(bx(9, -3, 7, 10, -2, 9), AIR);
    w.put(8, -3, 8, cobble);
    w.put(8, -2, 8, cobble);
    w.put(10, -3, 8, Cell::chest(NEG_X));

    w.put(4, -3, 10, Block::TripwireHook.front(X));
    w.put(7, -3, 10, Block::TripwireHook.front(NEG_X));
    w.put(5, -3, 10, Block::Tripwire);
    w.put(6, -3, 10, Block::Tripwire);
    w.put(3, -2, 11, Block::Dispenser.front(X));
    w.put(8, -2, 11, Block::Dispenser.front(NEG_X));
    w.put(6, -3, 13, Cell::chest(NEG_Z));
}
