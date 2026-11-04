use std::sync::LazyLock;

use glam::IVec3;

use super::piece::{BlockBox, Facing, Piece, PieceKind, PieceWriter};
use super::template::Template;
use crate::block::Block;
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
        salt: 0,
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

pub(super) static TEMPLATE: LazyLock<Template> =
    LazyLock::new(|| Template::parse("swamp_hut.txt", include_str!("templates/swamp_hut.txt")));

pub(super) fn place(w: &mut PieceWriter) {
    w.template(&TEMPLATE, IVec3::ZERO);
    for (x, z) in [(1, 2), (5, 2), (1, 7), (5, 7)] {
        w.fill_down(x, -1, z, Block::Log);
    }
}
