use glam::IVec2;

use super::*;
use crate::terrain::{Terrain, WATER_LEVEL, lake_key};

fn local(pos: IVec3) -> IVec3 {
    IVec3::new(pos.x.rem_euclid(16), pos.y, pos.z.rem_euclid(16))
}

fn keys(center: ColumnPos, radius: i32) -> impl Iterator<Item = ColumnPos> {
    (-radius..=radius).flat_map(move |z| (-radius..=radius).map(move |x| center + IVec2::new(x, z)))
}

fn cells() -> impl Iterator<Item = IVec3> {
    (0..WORLD_HEIGHT)
        .flat_map(|y| (0..16).flat_map(move |z| (0..16).map(move |x| IVec3::new(x, y, z))))
}

fn ground_and_carved(terrain: &Terrain, key: ColumnPos) -> (Column, Column) {
    (terrain.generate_ground(key), terrain.generate_carved(key))
}

#[test]
fn same_seed_same_caves_other_seed_other() {
    let key = ColumnPos::new(-3, 7);
    let first = Terrain::new(42).generate_carved(key);
    let second = Terrain::new(42).generate_carved(key);
    let other = Terrain::new(43).generate_carved(key);
    assert!(cells().all(|p| first.get(p) == second.get(p)));
    assert!(cells().any(|p| first.get(p) != other.get(p)));
}

#[test]
fn plan_does_not_depend_on_cache() {
    let caves = Caves::new(5);
    let fresh = Caves::new(5);
    let source = (3, -9);
    let a = caves.plan(source);
    caves.plans.lock().unwrap().clear();
    let b = caves.plan(source);
    let c = fresh.plan(source);
    for other in [&b, &c] {
        assert_eq!(a.caves.len(), other.caves.len());
        assert!(
            a.caves
                .iter()
                .zip(&other.caves)
                .all(|(l, r)| l.center == r.center)
        );
    }
}

#[test]
fn carved_share_of_underground_is_moderate() {
    for seed in [1337, 42] {
        let terrain = Terrain::new(seed);
        let (mut solid, mut hollow) = (0, 0);
        for key in keys(ColumnPos::new(5, -3), 4) {
            let (ground, carved) = ground_and_carved(&terrain, key);
            for p in cells().filter(|p| p.y > 4) {
                if replaceable(ground.get(p), Block::Air) {
                    solid += 1;
                    hollow += usize::from(matches!(carved.get(p), Block::Air | Block::Lava));
                }
            }
        }
        let share = hollow as f32 / solid as f32;
        assert!((0.01..=0.12).contains(&share), "seed {seed}: {share}");
    }
}

#[test]
fn lava_only_below_lava_level_and_air_only_above() {
    let terrain = Terrain::new(1337);
    let (mut lava, mut high_air) = (0, 0);
    for key in keys(ColumnPos::ZERO, 5) {
        let (ground, carved) = ground_and_carved(&terrain, key);
        for pos in cells() {
            let (before, after) = (ground.get(pos), carved.get(pos));
            if after == Block::Lava && before != Block::Lava {
                assert!(pos.y < LAVA_LEVEL, "{key} {pos}");
                assert!(replaceable(before, Block::Air), "{key} {pos}");
                lava += 1;
            } else if before != after && after == Block::Air && replaceable(before, Block::Air) {
                assert!(pos.y >= LAVA_LEVEL, "{key} {pos}");
                high_air += 1;
            }
        }
    }
    assert!(lava > 0 && high_air > 0, "lava {lava}, air {high_air}");
}

#[test]
fn bedrock_survives() {
    let terrain = Terrain::new(1337);
    for key in keys(ColumnPos::new(2, 2), 3) {
        let (ground, carved) = ground_and_carved(&terrain, key);
        for p in cells().filter(|p| ground.get(*p) == Block::Bedrock) {
            assert_eq!(carved.get(p), Block::Bedrock);
        }
    }
}

#[test]
fn ravines_occur_at_vanilla_rate() {
    let caves = Caves::new(1337);
    let sources = 120 * 120;
    let ravines: usize = (0..120)
        .flat_map(|x| (0..120).map(move |z| (x, z)))
        .map(|source| caves.plan(source).ravines.len())
        .sum();
    let rate = ravines as f32 / sources as f32;
    assert!((0.01..=0.03).contains(&rate), "{rate}");
}

#[test]
fn ravines_cut_tall_narrow_gaps() {
    let caves = Caves::new(1337);
    let terrain = Terrain::new(1337);
    let source = (0..120)
        .flat_map(|x| (0..120).map(move |z| (x, z)))
        .find(|&source| !caves.plan(source).ravines.is_empty())
        .expect("no ravine");
    let ravine = &caves.plan(source).ravines[0];
    let middle = ravine.segments[ravine.segments.len() / 2].center;
    let key = ColumnPos::new(
        (middle.x.floor() as i32).div_euclid(16),
        (middle.z.floor() as i32).div_euclid(16),
    );
    let tallest = keys(key, 1)
        .map(|key| {
            let (ground, carved) = ground_and_carved(&terrain, key);
            (0..16)
                .flat_map(|z| (0..16).map(move |x| (x, z)))
                .map(|(x, z)| {
                    let (mut run, mut best) = (0, 0);
                    for y in 0..WORLD_HEIGHT {
                        let p = IVec3::new(x, y, z);
                        if ground.get(p) != Block::Air && carved.get(p) == Block::Air {
                            run += 1;
                            best = best.max(run);
                        } else {
                            run = 0;
                        }
                    }
                    best
                })
                .max()
                .unwrap_or(0)
        })
        .max()
        .unwrap_or(0);
    assert!(tallest >= 10, "{tallest}");
}

#[test]
fn tunnels_continue_across_column_borders() {
    let terrain = Terrain::new(1337);
    let (mut open_pairs, mut total) = (0, 0);
    for key in keys(ColumnPos::new(5, -3), 4) {
        let east = key + IVec2::X;
        let (a, b) = (terrain.generate_carved(key), terrain.generate_carved(east));
        for y in 11..WORLD_HEIGHT {
            for z in 0..16 {
                let (left, right) = (IVec3::new(15, y, z), IVec3::new(0, y, z));
                if a.get(left) == Block::Air && b.get(right) == Block::Air {
                    open_pairs += 1;
                }
                total += 1;
            }
        }
    }
    assert!(open_pairs > 50, "{open_pairs} of {total}");
}

#[test]
fn carving_never_opens_into_water() {
    let terrain = Terrain::new(1337);
    let lake = lake_key(&terrain);
    let (mut water, mut leaks) = (0, 0);
    for key in keys(lake, 3) {
        let (ground, carved) = ground_and_carved(&terrain, key);
        for p in cells().filter(|p| carved.get(*p) == Block::Water) {
            water += 1;
            for d in [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z, IVec3::NEG_Y] {
                let n = p + d;
                if Column::contains(n) && (0..16).contains(&n.x) && (0..16).contains(&n.z) {
                    let (before, after) = (ground.get(n), carved.get(n));
                    leaks +=
                        usize::from(before != after && matches!(after, Block::Air | Block::Lava));
                }
            }
        }
    }
    assert!(water > 1000, "{water} water blocks");
    assert_eq!(leaks, 0);
}

#[test]
fn surface_is_not_riddled() {
    let terrain = Terrain::new(1337);
    let (mut columns, mut opened) = (0, 0);
    for key in keys(ColumnPos::new(8, 8), 6) {
        let (ground, carved) = ground_and_carved(&terrain, key);
        let origin = key.origin();
        for z in 0..16 {
            for x in 0..16 {
                let h = terrain.surface_height(origin.x + x, origin.z + z);
                if h <= WATER_LEVEL + 1 {
                    continue;
                }
                columns += 1;
                let p = IVec3::new(x, h, z);
                opened += usize::from(ground.get(p) != carved.get(p));
            }
        }
    }
    let share = opened as f32 / columns as f32;
    assert!(share < 0.04, "{share}");
}

#[test]
fn trees_stand_on_ground() {
    let terrain = Terrain::new(1337);
    let mut trees = 0;
    for key in keys(ColumnPos::new(40, 10), 6) {
        let column = terrain.generate(key);
        let origin = key.origin();
        for z in origin.z..origin.z + 16 {
            for x in origin.x..origin.x + 16 {
                let Some(tree) = crate::trees::tree_at(&terrain, x, z) else {
                    continue;
                };
                if terrain.lake_covers(x, z) {
                    continue;
                }
                trees += 1;
                for dy in 1..=ROOF {
                    let p = local(tree.base - IVec3::Y * dy);
                    assert!(column.get(p).is_solid(), "tree at {x}, {z}, dy {dy}");
                }
            }
        }
    }
    assert!(trees > 10, "{trees} trees");
}
