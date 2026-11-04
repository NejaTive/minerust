use glam::IVec3;

use super::piece::{BlockBox, Facing, Piece, PieceKind, PieceWriter};
use crate::block::{Block, facing_code};
use crate::stream::Stream;
use crate::terrain::Terrain;

const WIDTH: i32 = 7;
const HEIGHT: i32 = 7;
const DEPTH: i32 = 9;
const MIN_GROUND: i32 = 63;

pub(super) fn layout(terrain: &Terrain, anchor: (i32, i32), rng: &mut Stream) -> Vec<Piece> {
    let facing = Facing::ALL[rng.below(4) as usize];
    let (size_x, size_z) = match facing {
        Facing::North | Facing::South => (WIDTH, DEPTH),
        Facing::East | Facing::West => (DEPTH, WIDTH),
    };
    let min_x = anchor.0 - size_x / 2;
    let min_z = anchor.1 - size_z / 2;
    let ground = average_ground(terrain, min_x, min_z, size_x, size_z);
    let bounds = BlockBox::new(
        IVec3::new(min_x, ground, min_z),
        IVec3::new(min_x + size_x - 1, ground + HEIGHT - 1, min_z + size_z - 1),
    );
    vec![Piece {
        bounds,
        facing,
        kind: PieceKind::SwampHut,
    }]
}

/// `offsetToAverageGroundLevel`: среднее по рамке из верха твёрдого блока или воды, не ниже уровня моря.
pub(super) fn average_ground(
    terrain: &Terrain,
    min_x: i32,
    min_z: i32,
    size_x: i32,
    size_z: i32,
) -> i32 {
    let mut total = 0;
    for z in min_z..min_z + size_z {
        for x in min_x..min_x + size_x {
            total += (terrain.surface_height(x, z) + 1).max(MIN_GROUND);
        }
    }
    total / (size_x * size_z)
}

pub(super) fn place(w: &mut PieceWriter) {
    let planks = Block::SprucePlanks;
    let at = IVec3::new;
    w.fill(at(1, 1, 1), at(5, 1, 7), planks);
    w.fill(at(1, 4, 2), at(5, 4, 7), planks);
    w.fill(at(2, 1, 0), at(4, 1, 0), planks);
    w.fill(at(2, 2, 2), at(3, 3, 2), planks);
    w.fill(at(1, 2, 3), at(1, 3, 6), planks);
    w.fill(at(5, 2, 3), at(5, 3, 6), planks);
    w.fill(at(2, 2, 7), at(4, 3, 7), planks);
    for (x, z) in [(1, 2), (5, 2), (1, 7), (5, 7)] {
        w.fill(at(x, 0, z), at(x, 3, z), Block::Log);
    }
    for (x, y, z) in [(2, 3, 2), (3, 3, 7), (1, 2, 1), (5, 2, 1)] {
        w.set(x, y, z, Block::OakFence);
    }
    for (x, y, z) in [(1, 3, 4), (5, 3, 4), (5, 3, 5)] {
        w.set(x, y, z, Block::Air);
    }
    w.flower_pot(1, 3, 5, Some(Block::RedMushroom));
    w.set(3, 2, 6, Block::CraftingTable);
    w.cauldron(4, 2, 6, 0);
    roof(w);
    for (x, z) in [(1, 2), (5, 2), (1, 7), (5, 7)] {
        w.fill_down(x, -1, z, Block::Log);
    }
}

fn roof(w: &mut PieceWriter) {
    let rows = [
        ((0, 1), (6, 1), IVec3::Z),
        ((0, 2), (0, 7), IVec3::X),
        ((6, 2), (6, 7), IVec3::NEG_X),
        ((0, 8), (6, 8), IVec3::NEG_Z),
    ];
    for ((x0, z0), (x1, z1), rise) in rows {
        let data = facing_code(w.world_dir(rise));
        for z in z0..=z1 {
            for x in x0..=x1 {
                w.set_with_data(x, 4, z, Block::SpruceStairs, data);
            }
        }
    }
}
