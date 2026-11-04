use std::sync::LazyLock;

use glam::IVec3;

use super::cell::{AIR, Cell, NEG_X, NEG_Z, Selector};
use super::piece::{BlockBox, Facing, Piece, PieceKind, PieceWriter, bx};
use super::swamp_hut::average_ground;
use super::template::Template;
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
const DOME_RADIUS_SQ: i32 = 12;
const HATCH_Z: i32 = 4;
const MIN_SEGMENTS: u32 = 4;
const SEGMENT_SPREAD: u32 = 8;
const EGG_ONE_IN: u32 = 24;
const SALT_LIMIT: u32 = 1 << 30;

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
        salt,
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
            kind: PieceKind::IglooBasement(segments as u8),
            salt,
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

pub(super) static TOP: LazyLock<Template> =
    LazyLock::new(|| Template::parse("igloo_top.txt", include_str!("templates/igloo_top.txt")));

pub(super) fn place_top(w: &mut PieceWriter) {
    w.template(&TOP, IVec3::ZERO);
    for z in 0..WIDTH {
        for x in 0..WIDTH {
            let d2 = (x - DOME_CENTER).pow(2) + (z - DOME_CENTER).pow(2);
            if d2 <= DOME_RADIUS_SQ {
                w.fill_down(x, -1, z, Block::SnowBlock);
            }
        }
    }
}

pub(super) fn place_basement(w: &mut PieceWriter, segments: u8) {
    let floor = SEGMENT_HEIGHT * i32::from(segments) + ROOM_HEIGHT;
    let bricks = Selector::new(&[
        (12, Cell::of(Block::StoneBricks)),
        (5, Cell::of(Block::MossyStoneBricks)),
        (3, Cell::of(Block::CrackedStoneBricks)),
    ]);
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
                    w.put(x, y, z, AIR);
                    continue;
                }
                let brick = bricks.pick(w, x, y, z);
                if w.chance(x, y, z, EGG_ONE_IN) {
                    w.put(
                        x,
                        y,
                        z,
                        Cell::with_data(Block::MonsterEgg, egg_variant(brick.block)),
                    );
                } else {
                    w.put(x, y, z, brick);
                }
            }
        }
    }
    w.fill_random(
        bx(
            DOME_CENTER,
            1,
            HATCH_Z + 1,
            DOME_CENTER,
            ROOM_HEIGHT - 2,
            ROOM_DEPTH - 2,
        ),
        &bricks,
    );
    for x in [1, WIDTH - 2] {
        w.fill(
            bx(x, 1, HATCH_Z + 1, x, ROOM_HEIGHT - 2, HATCH_Z + 1),
            Block::IronBars,
        );
    }
    w.fill_random(
        bx(
            DOME_CENTER - 1,
            ROOM_HEIGHT,
            HATCH_Z - 1,
            DOME_CENTER + 1,
            floor,
            HATCH_Z + 1,
        ),
        &bricks,
    );
    w.fill(
        bx(DOME_CENTER, 1, HATCH_Z, DOME_CENTER, floor, HATCH_Z),
        Block::Ladder.front(NEG_Z),
    );
    w.put(
        DOME_CENTER,
        floor + 1,
        HATCH_Z,
        Block::OakTrapdoor.front(NEG_Z),
    );
    w.put(WIDTH - 2, 1, 2, Cell::chest(NEG_X));
    w.put(1, 1, 2, Block::BrewingStand);
}

/// `data` яйца, которое выглядит как кирпич `brick`.
fn egg_variant(brick: Block) -> u8 {
    match brick {
        Block::MossyStoneBricks => 3,
        Block::CrackedStoneBricks => 4,
        _ => 2,
    }
}
