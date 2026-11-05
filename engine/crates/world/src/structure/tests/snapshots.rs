use glam::IVec3;

use super::super::*;
use super::dump::{assert_snapshot, dump, dump_below};

const ORIGIN: IVec3 = IVec3::new(10, 5, 12);
const BASEMENT_SALT: u32 = 123_456;

fn part(kind: PieceKind, size: IVec3, facing: Facing) -> Piece {
    Piece {
        bounds: BlockBox::from_local(ORIGIN, size, facing),
        facing,
        kind,
        salt: if let PieceKind::IglooBasement(_) = kind {
            BASEMENT_SALT
        } else {
            0
        },
    }
}

pub(super) fn hut(facing: Facing) -> Piece {
    part(PieceKind::SwampHut, IVec3::new(7, 7, 9), facing)
}

pub(super) fn igloo_top(facing: Facing) -> Piece {
    part(PieceKind::Igloo, IVec3::new(7, 5, 8), facing)
}

pub(super) fn basement(segments: u8, facing: Facing) -> Piece {
    let depth = 3 * i32::from(segments) + 6;
    part(
        PieceKind::IglooBasement(segments),
        IVec3::new(7, depth + 2, 9),
        facing,
    )
}

#[test]
fn swamp_hut_snapshot() {
    assert_snapshot("swamp_hut", &dump(&hut(Facing::South)));
}

#[test]
fn igloo_top_snapshot() {
    assert_snapshot("igloo_top", &dump(&igloo_top(Facing::South)));
}

#[test]
fn igloo_basement_snapshots() {
    assert_snapshot("igloo_basement_min", &dump(&basement(4, Facing::South)));
    assert_snapshot("igloo_basement_max", &dump(&basement(11, Facing::South)));
}

const DESERT_TEMPLE_DEPTH_BELOW: i32 = 14;

pub(super) fn desert_temple(facing: Facing) -> Piece {
    Piece {
        bounds: BlockBox::from_local(IVec3::new(10, 30, 12), IVec3::new(21, 15, 21), facing),
        facing,
        kind: PieceKind::DesertTemple,
        salt: 0,
    }
}

#[test]
fn desert_temple_snapshot() {
    assert_snapshot(
        "desert_temple",
        &dump_below(&desert_temple(Facing::South), DESERT_TEMPLE_DEPTH_BELOW),
    );
}

const JUNGLE_TEMPLE_DEPTH_BELOW: i32 = 4;

pub(super) fn jungle_temple(facing: Facing) -> Piece {
    Piece {
        bounds: BlockBox::from_local(IVec3::new(10, 30, 12), IVec3::new(12, 10, 15), facing),
        facing,
        kind: PieceKind::JungleTemple,
        salt: 4242,
    }
}

#[test]
fn jungle_temple_snapshot() {
    assert_snapshot(
        "jungle_temple",
        &dump_below(&jungle_temple(Facing::South), JUNGLE_TEMPLE_DEPTH_BELOW),
    );
}
