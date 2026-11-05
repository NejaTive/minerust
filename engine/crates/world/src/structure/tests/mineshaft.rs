use glam::IVec3;

use super::super::mineshaft::CorridorSpec;
use super::super::*;
use super::dump::{assert_snapshot, dump};
use super::{count_in, snapshot, start_columns};
use crate::block::Block;
use crate::column::{Column, WORLD_HEIGHT};
use crate::terrain::{WATER_LEVEL, WorldType};

const SEED: i32 = 1337;
const SALT: u32 = 77_031;

fn piece(kind: PieceKind, size: IVec3, facing: Facing) -> Piece {
    Piece {
        bounds: BlockBox::from_local(IVec3::new(10, 30, 12), size, facing),
        facing,
        kind,
        salt: SALT,
    }
}

fn corridor(kind: MineshaftKind, rails: bool, spiders: bool, facing: Facing) -> Piece {
    let spec = CorridorSpec {
        kind,
        sections: 2,
        rails,
        spiders,
        supports: 0b11,
        spawner: spiders.then_some(4),
    };
    piece(
        PieceKind::MineshaftCorridor(spec),
        IVec3::new(3, 3, 10),
        facing,
    )
}

#[test]
fn corridor_snapshots() {
    assert_snapshot(
        "mineshaft_corridor_rails",
        &dump(&corridor(MineshaftKind::Normal, true, false, Facing::South)),
    );
    assert_snapshot(
        "mineshaft_corridor_spiders",
        &dump(&corridor(MineshaftKind::Normal, false, true, Facing::South)),
    );
    assert_snapshot(
        "mineshaft_corridor_mesa",
        &dump(&corridor(MineshaftKind::Mesa, true, false, Facing::South)),
    );
}

#[test]
fn cross_stairs_and_room_snapshots() {
    let cross = |height| {
        piece(
            PieceKind::MineshaftCross(MineshaftKind::Normal),
            IVec3::new(5, height, 5),
            Facing::South,
        )
    };
    assert_snapshot("mineshaft_cross_single", &dump(&cross(3)));
    assert_snapshot("mineshaft_cross_double", &dump(&cross(7)));
    assert_snapshot(
        "mineshaft_stairs",
        &dump(&piece(
            PieceKind::MineshaftStairs,
            IVec3::new(3, 8, 9),
            Facing::South,
        )),
    );
    assert_snapshot(
        "mineshaft_room",
        &dump(&piece(
            PieceKind::MineshaftRoom,
            IVec3::new(9, 6, 9),
            Facing::South,
        )),
    );
}

#[test]
fn rotated_corridor_matches_the_south_one() {
    let south = dump(&corridor(MineshaftKind::Normal, true, false, Facing::South));
    for facing in [Facing::North, Facing::East, Facing::West] {
        let rotated = dump(&corridor(MineshaftKind::Normal, true, false, facing));
        let strip = |text: &str| {
            text.lines()
                .filter(|line| !line.starts_with("data") && !line.contains("rail"))
                .count()
        };
        assert_eq!(strip(&south), strip(&rotated), "{facing:?}");
    }
}

fn showcase_start(kind: MineshaftKind) -> (Terrain, StructureStart) {
    let terrain = Terrain::of(SEED, WorldType::Structures);
    let plot = plots()
        .find(|plot| plot.variant == Variant::Mineshaft(kind))
        .unwrap();
    let start = build(&terrain, plot.kind, plot.column()).unwrap();
    (terrain, start)
}

#[test]
fn showcase_shafts_grow_a_tree_of_separate_pieces() {
    for kind in [MineshaftKind::Normal, MineshaftKind::Mesa] {
        let (_, start) = showcase_start(kind);
        assert_eq!(start.variant, Variant::Mineshaft(kind));
        let shaft: Vec<&Piece> = start
            .pieces
            .iter()
            .filter(|piece| piece.kind != PieceKind::MineshaftOpening)
            .collect();
        assert!(shaft.len() > 6, "{kind:?} {}", shaft.len());
        assert_eq!(shaft[0].kind, PieceKind::MineshaftRoom);
        for (i, a) in shaft.iter().enumerate() {
            for b in &shaft[i + 1..] {
                assert!(!a.bounds.intersects(&b.bounds), "{kind:?} {a:?} {b:?}");
            }
        }
        let room = shaft[0].bounds;
        for piece in &shaft {
            assert!((piece.bounds.min.x - room.min.x).abs() <= 100);
            assert!((piece.bounds.min.z - room.min.z).abs() <= 100);
        }
        let top = start.bounds.max.y;
        match kind {
            MineshaftKind::Normal => assert!(top < WATER_LEVEL, "{top}"),
            MineshaftKind::Mesa => assert!(top > WATER_LEVEL, "{top}"),
        }
    }
}

#[test]
fn shaft_layout_is_deterministic() {
    let (_, a) = showcase_start(MineshaftKind::Normal);
    let (_, b) = showcase_start(MineshaftKind::Normal);
    assert_eq!(a.pieces, b.pieces);
}

fn shaft_columns(terrain: &Terrain, start: &StructureStart, reverse: bool) -> Vec<Column> {
    let mut keys = start_columns(start);
    if reverse {
        keys.reverse();
    }
    let mut columns: Vec<_> = keys
        .into_iter()
        .map(|key| (key, terrain.generate(key)))
        .collect();
    columns.sort_by_key(|(key, _)| (key.x(), key.z()));
    columns.into_iter().map(|(_, column)| column).collect()
}

#[test]
fn shafts_are_seamless_and_hold_their_blocks() {
    for kind in [MineshaftKind::Normal, MineshaftKind::Mesa] {
        let (terrain, start) = showcase_start(kind);
        let forward = shaft_columns(&terrain, &start, false);
        let backward = shaft_columns(&terrain, &start, true);
        for (a, b) in forward.iter().zip(&backward) {
            assert!(snapshot(a) == snapshot(b));
        }
        let (planks, fence) = match kind {
            MineshaftKind::Normal => (Block::OakPlanks, Block::OakFence),
            MineshaftKind::Mesa => (Block::DarkOakPlanks, Block::DarkOakFence),
        };
        assert!(count_in(&forward, planks) > 20, "{kind:?}");
        assert!(count_in(&forward, fence) > 10, "{kind:?}");
        let other = match kind {
            MineshaftKind::Normal => Block::DarkOakPlanks,
            MineshaftKind::Mesa => Block::OakPlanks,
        };
        assert_eq!(count_in(&forward, other), 0, "{kind:?}");
        if kind == MineshaftKind::Normal {
            assert!(count_in(&forward, Block::Dirt) > 40);
        }
    }
}

#[test]
fn the_room_interior_is_open_across_column_seams() {
    let (terrain, start) = showcase_start(MineshaftKind::Normal);
    let room = start.pieces[0].bounds;
    let floor = room.min.y + 1;
    for z in room.min.z..=room.max.z {
        for x in room.min.x..=room.max.x {
            let key = ColumnPos::of_block(IVec3::new(x, 0, z));
            let column = terrain.generate(key);
            let at = IVec3::new(x, floor, z) - key.origin();
            assert_eq!(column.get(at), Block::Air, "{x} {z}");
        }
    }
}

#[test]
fn shaft_blocks_never_leave_the_world_height() {
    let (terrain, start) = showcase_start(MineshaftKind::Mesa);
    assert!(start.bounds.min.y >= 1 && start.bounds.max.y < WORLD_HEIGHT);
    let _ = terrain;
}

#[test]
fn mineshafts_appear_at_a_sensible_rate() {
    let terrain = Terrain::new(SEED);
    let center = ColumnPos::new(1000, 1000);
    let origins = terrain.starts_near(StructureKind::Mineshaft, center, 50);
    let built = origins.iter().filter(|s| s.get().is_some()).count();
    assert!((15..=90).contains(&built), "{built}");
}

#[test]
fn locate_finds_a_mineshaft() {
    let terrain = Terrain::new(SEED);
    let target = LocateName::find("Mineshaft").unwrap();
    assert!(target.ready() && terrain.structure_built(target.kind));
    let (x, z, distance) = terrain
        .nearest_structure(target, 2000, 2000, 20_000)
        .unwrap();
    assert!(distance < 20_000);
    let start = terrain
        .starts_near(StructureKind::Mineshaft, ColumnPos::new(x >> 4, z >> 4), 0)
        .into_iter()
        .find_map(|s| s.get().map(|s| s.anchor));
    assert_eq!(start, Some((x, z)));
}

#[test]
fn mesa_biomes_give_the_mesa_variant() {
    let terrain = Terrain::new(SEED);
    let mut seen = [false; 2];
    for cz in -150..150 {
        for cx in -150..150 {
            if !placement::mineshaft_at(SEED, cx, cz) {
                continue;
            }
            let origin = ColumnPos::new(cx, cz);
            let Some(Variant::Mineshaft(kind)) =
                placement::variant(&terrain, StructureKind::Mineshaft, origin)
            else {
                panic!();
            };
            let (x, z) = placement::anchor(&terrain, origin);
            assert_eq!(
                kind == MineshaftKind::Mesa,
                terrain.biome_at(x, z).is_mesa()
            );
            seen[usize::from(kind == MineshaftKind::Mesa)] = true;
        }
    }
    assert!(seen[0]);
}
