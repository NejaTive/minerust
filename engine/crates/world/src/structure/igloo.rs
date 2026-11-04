use glam::IVec3;

use super::piece::{BlockBox, Facing, Piece, PieceKind, PieceWriter};
use super::swamp_hut::average_ground;
use crate::block::Block;
use crate::stream::Stream;
use crate::terrain::{Terrain, WorldType};

const WIDTH: i32 = 7;
const TOP_HEIGHT: i32 = 5;
const TOP_DEPTH: i32 = 8;
const ROOM_HEIGHT: i32 = 6;
const ROOM_DEPTH: i32 = 9;
const SEGMENT_HEIGHT: i32 = 3;
const DOME_CENTER: i32 = 3;
const HATCH_Z: i32 = 4;
const RED: u8 = 14;
const MIN_SEGMENTS: u32 = 4;
const SEGMENT_SPREAD: u32 = 8;
const EGG_ONE_IN: u32 = 24;
const SALT_LIMIT: u32 = 1 << 30;

/// Квадраты расстояния от оси купола: `(внешний, внутренний)` по слоям, внутри внутреннего — воздух.
const DOME: [(i32, i32); TOP_HEIGHT as usize] = [(12, -1), (12, 5), (10, 4), (8, 2), (4, -1)];

pub(super) fn layout(terrain: &Terrain, anchor: (i32, i32), rng: &mut Stream) -> Vec<Piece> {
    let facing = Facing::ALL[rng.below(4) as usize];
    let basement = rng.flip();
    let segments = MIN_SEGMENTS + rng.below(SEGMENT_SPREAD);
    let salt = rng.below(SALT_LIMIT);
    let (size_x, size_z) = match facing {
        Facing::North | Facing::South => (WIDTH, TOP_DEPTH),
        Facing::East | Facing::West => (TOP_DEPTH, WIDTH),
    };
    let min_x = anchor.0 - size_x / 2;
    let min_z = anchor.1 - size_z / 2;
    let floor = average_ground(terrain, min_x, min_z, size_x, size_z) - 1;
    let top = BlockBox::new(
        IVec3::new(min_x, floor, min_z),
        IVec3::new(
            min_x + size_x - 1,
            floor + TOP_HEIGHT - 1,
            min_z + size_z - 1,
        ),
    );
    let mut pieces = vec![Piece {
        bounds: top,
        facing,
        kind: PieceKind::Igloo,
    }];
    if basement || terrain.world_type() == WorldType::Structures {
        let segments = segments as i32;
        let depth = SEGMENT_HEIGHT * segments + ROOM_HEIGHT;
        let bounds = BlockBox::from_local(
            origin(&top, facing, floor - depth),
            IVec3::new(WIDTH, depth + 2, ROOM_DEPTH),
            facing,
        );
        pieces.push(Piece {
            bounds,
            facing,
            kind: PieceKind::IglooBasement(segments as u8, salt),
        });
    }
    pieces
}

/// Мировая клетка локального начала части на высоте `y`.
fn origin(bounds: &BlockBox, facing: Facing, y: i32) -> IVec3 {
    let (min, max) = (bounds.min, bounds.max);
    match facing {
        Facing::South | Facing::East => IVec3::new(min.x, y, min.z),
        Facing::North => IVec3::new(min.x, y, max.z),
        Facing::West => IVec3::new(max.x, y, min.z),
    }
}

pub(super) fn place_top(w: &mut PieceWriter) {
    let at = IVec3::new;
    for (y, &(outer, inner)) in DOME.iter().enumerate() {
        for z in 0..WIDTH {
            for x in 0..WIDTH {
                let d2 = (x - DOME_CENTER).pow(2) + (z - DOME_CENTER).pow(2);
                if d2 > outer {
                    continue;
                }
                w.set(
                    x,
                    y as i32,
                    z,
                    if d2 <= inner {
                        Block::Air
                    } else {
                        Block::SnowBlock
                    },
                );
            }
        }
    }
    for z in 0..WIDTH {
        for x in 0..WIDTH {
            let d2 = (x - DOME_CENTER).pow(2) + (z - DOME_CENTER).pow(2);
            if d2 <= DOME[0].0 {
                w.fill_down(x, -1, z, Block::SnowBlock);
            }
            if d2 <= DOME[1].1 {
                w.set(x, 1, z, Block::Carpet);
            }
        }
    }
    for (x, z) in [(0, DOME_CENTER), (WIDTH - 1, DOME_CENTER), (DOME_CENTER, 0)] {
        w.set(x, 2, z, Block::Ice);
    }
    w.fill(at(2, 0, 7), at(4, 0, 7), Block::SnowBlock);
    for y in 1..=2 {
        w.fill(at(3, y, 6), at(3, y, 7), Block::Air);
        for x in [2, 4] {
            w.set(x, y, 7, Block::SnowBlock);
        }
    }
    w.fill(at(2, 3, 6), at(4, 3, 7), Block::SnowBlock);
    w.bed(3, 1, 2, IVec3::NEG_Z, RED);
    w.set(1, 1, 3, Block::CraftingTable);
    w.set(1, 2, 3, Block::RedstoneTorch);
    w.furnace(5, 1, 3, IVec3::NEG_X, false);
}

pub(super) fn place_basement(w: &mut PieceWriter, segments: u8, salt: u32) {
    let floor = SEGMENT_HEIGHT * i32::from(segments) + ROOM_HEIGHT;
    let brick = |x, y, z| brick(salt, x, y, z);
    for y in 0..ROOM_HEIGHT {
        for z in 0..ROOM_DEPTH {
            for x in 0..WIDTH {
                let shell = x == 0
                    || x == WIDTH - 1
                    || y == 0
                    || y == ROOM_HEIGHT - 1
                    || z == 0
                    || z == ROOM_DEPTH - 1;
                if !shell {
                    w.set(x, y, z, Block::Air);
                } else if egg(salt, x, y, z) {
                    w.set_with_data(x, y, z, Block::MonsterEgg, egg_variant(brick(x, y, z)));
                } else {
                    w.set(x, y, z, brick(x, y, z));
                }
            }
        }
    }
    for y in 1..ROOM_HEIGHT - 1 {
        for z in HATCH_Z + 1..ROOM_DEPTH - 1 {
            w.set(DOME_CENTER, y, z, brick(DOME_CENTER, y, z));
        }
        for x in [1, WIDTH - 2] {
            w.set(x, y, HATCH_Z + 1, Block::IronBars);
        }
    }
    for y in ROOM_HEIGHT..=floor {
        for z in HATCH_Z - 1..=HATCH_Z + 1 {
            for x in DOME_CENTER - 1..=DOME_CENTER + 1 {
                let hole = x == DOME_CENTER && z == HATCH_Z;
                w.set(x, y, z, if hole { Block::Air } else { brick(x, y, z) });
            }
        }
    }
    w.set(DOME_CENTER, ROOM_HEIGHT - 1, HATCH_Z, Block::Air);
    for y in 1..=floor {
        w.set_facing(DOME_CENTER, y, HATCH_Z, Block::Ladder, IVec3::NEG_Z);
    }
    w.trapdoor(DOME_CENTER, floor + 1, HATCH_Z, IVec3::NEG_Z);
    w.chest(WIDTH - 2, 1, 2, IVec3::NEG_X);
    w.set(1, 1, 2, Block::BrewingStand);
}

fn brick(salt: u32, x: i32, y: i32, z: i32) -> Block {
    match mix(salt, x, y, z) % 20 {
        0..=11 => Block::StoneBricks,
        12..=16 => Block::MossyStoneBricks,
        _ => Block::CrackedStoneBricks,
    }
}

/// `data` яйца, которое выглядит как кирпич `brick`.
fn egg_variant(brick: Block) -> u8 {
    match brick {
        Block::MossyStoneBricks => 3,
        Block::CrackedStoneBricks => 4,
        _ => 2,
    }
}

fn egg(salt: u32, x: i32, y: i32, z: i32) -> bool {
    mix(!salt, x, y, z).is_multiple_of(EGG_ONE_IN)
}

/// Хэш клетки части: не зависит от порядка генерации колонок.
fn mix(salt: u32, x: i32, y: i32, z: i32) -> u32 {
    let mut h = salt
        ^ (x as u32).wrapping_mul(0x9E37_79B1)
        ^ (y as u32).wrapping_mul(0x85EB_CA77)
        ^ (z as u32).wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^ h >> 15
}
