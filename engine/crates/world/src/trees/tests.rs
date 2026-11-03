use glam::IVec2;

use std::collections::HashSet;
use std::sync::OnceLock;

use super::*;
use crate::block::{AXIS_X, AXIS_Z};
use crate::column::WORLD_HEIGHT;

fn forest_trees() -> &'static [Tree] {
    static TREES: OnceLock<Vec<Tree>> = OnceLock::new();
    TREES.get_or_init(|| {
        let terrain = Terrain::new(1337);
        (-1024..1024)
            .flat_map(|z| (-1024..1024).map(move |x| (x, z)))
            .filter_map(|(x, z)| tree_at(&terrain, x, z))
            .collect()
    })
}

fn of_kind(kind: TreeKind) -> impl Iterator<Item = &'static Tree> {
    forest_trees().iter().filter(move |t| t.kind == kind)
}

fn spread(tree: &Tree) -> i32 {
    tree.leaves()
        .iter()
        .map(|l| (l.x - tree.base.x).abs().max((l.z - tree.base.z).abs()))
        .max()
        .unwrap()
}

#[test]
fn same_seed_same_trees() {
    let (a, b) = (Terrain::new(42), Terrain::new(42));
    for z in 0..200 {
        for x in 0..200 {
            assert_eq!(tree_at(&a, x, z), tree_at(&b, x, z));
        }
    }
    let other = Terrain::new(43);
    let differs = (0..200)
        .flat_map(|z| (0..200).map(move |x| (x, z)))
        .filter(|&(x, z)| tree_at(&a, x, z) != tree_at(&other, x, z))
        .count();
    assert!(differs > 0);
}

#[test]
fn tree_geometry_is_deterministic() {
    for tree in forest_trees().iter().take(300) {
        let copy = *tree;
        assert_eq!(tree.logs(), copy.logs());
        assert_eq!(tree.leaves(), copy.leaves());
        assert_eq!(tree.branches(), copy.branches());
    }
}

#[test]
fn every_kind_grows() {
    for kind in [
        TreeKind::Oak,
        TreeKind::Birch,
        TreeKind::Spruce,
        TreeKind::BigOak,
    ] {
        assert!(of_kind(kind).count() > 50, "{kind:?}");
    }
}

#[test]
fn trees_stand_on_the_surface_with_a_vertical_trunk() {
    let terrain = Terrain::new(1337);
    for tree in forest_trees() {
        assert_eq!(
            tree.base.y,
            terrain.surface_height(tree.base.x, tree.base.z) + 1
        );
        let trunk: Vec<_> = tree.logs().into_iter().take(tree.height as usize).collect();
        assert_eq!(trunk.len(), tree.height as usize);
        for (dy, (pos, axis)) in trunk.into_iter().enumerate() {
            assert_eq!((pos, axis), (tree.base + IVec3::Y * dy as i32, AXIS_Y));
        }
    }
}

#[test]
fn oak_has_a_round_crown_of_radius_two() {
    for tree in of_kind(TreeKind::Oak) {
        assert!((4..=6).contains(&tree.height));
        assert!(tree.branches().is_empty());
        assert_eq!(tree.logs().len(), tree.height as usize);
        let leaves = tree.leaves();
        assert_eq!(spread(tree), 2);
        let top = tree.top();
        assert!(leaves.contains(&IVec3::new(tree.base.x, top + 1, tree.base.z)));
        assert!(leaves.iter().all(|l| (top - 2..=top + 1).contains(&l.y)));
    }
}

#[test]
fn birch_is_taller_with_a_narrower_crown() {
    let radii: HashSet<i32> = of_kind(TreeKind::Birch).map(spread).collect();
    assert_eq!(radii, HashSet::from([1, 2]));
    for tree in of_kind(TreeKind::Birch) {
        assert!((5..=7).contains(&tree.height));
        assert!(tree.branches().is_empty());
        let top = tree.top();
        assert!(
            tree.leaves()
                .contains(&IVec3::new(tree.base.x, top + 1, tree.base.z))
        );
        assert!(
            tree.leaves()
                .iter()
                .all(|l| (top - 2..=top + 1).contains(&l.y))
        );
    }
    let heights: HashSet<i32> = of_kind(TreeKind::Birch).map(|t| t.height).collect();
    assert_eq!(heights, HashSet::from([5, 6, 7]));
}

#[test]
fn spruce_is_a_cone_of_layers_with_gaps() {
    for tree in of_kind(TreeKind::Spruce) {
        assert!((7..=10).contains(&tree.height));
        assert!(tree.branches().is_empty());
        let leaves = tree.leaves();
        let radius_at = |y: i32| {
            leaves
                .iter()
                .filter(|l| l.y == y)
                .map(|l| (l.x - tree.base.x).abs().max((l.z - tree.base.z).abs()))
                .max()
        };
        let top = tree.top();
        let caps = leaves.iter().filter(|l| l.y > top).count();
        assert!((1..=2).contains(&caps), "{caps} cap blocks");
        assert!(leaves.contains(&IVec3::new(tree.base.x, top + 1, tree.base.z)));
        let layers: Vec<i32> = (tree.base.y..=top)
            .filter(|&y| radius_at(y).is_some())
            .collect();
        assert_eq!(layers.last(), Some(&top));
        assert!(layers.windows(2).all(|w| w[1] - w[0] >= 2), "{layers:?}");
        assert!(layers.iter().all(|&y| y >= tree.base.y + 2));
        let radii: Vec<i32> = layers.iter().map(|&y| radius_at(y).unwrap()).collect();
        assert!(radii.windows(2).all(|w| w[0] >= w[1]), "{radii:?}");
        assert_eq!(radii.last(), Some(&1));
        assert!((2..=3).contains(&radii[0]), "{radii:?}");
    }
    let bottoms: HashSet<i32> = of_kind(TreeKind::Spruce).map(spread).collect();
    assert_eq!(bottoms, HashSet::from([2, 3]));
}

#[test]
fn big_oak_has_diagonal_branches_with_leaf_balls() {
    let mut branch_counts = HashSet::new();
    for tree in of_kind(TreeKind::BigOak) {
        assert!((6..=9).contains(&tree.height));
        let branches = tree.branches();
        assert!((2..=4).contains(&branches.len()));
        branch_counts.insert(branches.len());
        let dirs: HashSet<IVec3> = branches.iter().map(|b| b.dir).collect();
        assert_eq!(dirs.len(), branches.len());
        let logs = tree.logs();
        let leaves: HashSet<IVec3> = tree.leaves().into_iter().collect();
        for branch in &branches {
            let expected = if branch.dir.x != 0 { AXIS_X } else { AXIS_Z };
            assert_eq!(branch.axis, expected);
            assert!((2..=3).contains(&branch.length));
            assert!((2..=3).contains(&branch.radius));
            assert!(branch.start.y >= tree.base.y + tree.height / 2);
            assert!(branch.start.y < tree.top());
            let mut previous = branch.start;
            for (log, axis) in branch.logs() {
                assert_eq!(axis, expected);
                assert!(logs.contains(&(log, axis)));
                assert_eq!(log.y - previous.y, 1);
                assert_eq!((log - previous).abs().x + (log - previous).abs().z, 1);
                previous = log;
            }
            assert_eq!(previous, branch.tip());
            let r = branch.radius;
            for offset in [IVec3::Y * r, IVec3::NEG_Y * r, branch.dir * r] {
                assert!(
                    leaves.contains(&(branch.tip() + offset))
                        || logs.iter().any(|l| l.0 == branch.tip() + offset)
                );
            }
        }
        let top_center = IVec3::new(tree.base.x, tree.base.y + tree.height, tree.base.z);
        assert!(leaves.contains(&top_center));
        assert!(leaves.contains(&(top_center + IVec3::Y * 2)));
        assert!(spread(tree) <= MAX_REACH);
    }
    assert_eq!(branch_counts, HashSet::from([2, 3, 4]));
}

#[test]
fn trees_keep_their_distance() {
    let trees = forest_trees();
    assert!(trees.len() > 500, "only {} trees", trees.len());
    let by_pos: std::collections::HashMap<(i32, i32), TreeKind> = trees
        .iter()
        .map(|t| ((t.base.x, t.base.z), t.kind))
        .collect();
    for tree in trees {
        let (x, z) = (tree.base.x, tree.base.z);
        for dz in -(MAX_SPACING - 1)..MAX_SPACING {
            for dx in -(MAX_SPACING - 1)..MAX_SPACING {
                let Some(&other) = by_pos.get(&(x + dx, z + dz)) else {
                    continue;
                };
                if (dx, dz) != (0, 0) {
                    let near = dx.abs().max(dz.abs());
                    assert!(near >= spacing(tree.kind, other), "{x}, {z} +{dx}, {dz}");
                }
            }
        }
    }
}

fn trees_in(biome: Biome) -> Vec<&'static Tree> {
    let terrain = Terrain::new(1337);
    forest_trees()
        .iter()
        .filter(|t| terrain.biome_at(t.base.x, t.base.z) == biome)
        .collect()
}

#[test]
fn forest_mix_matches_the_shares() {
    let forest = trees_in(Biome::Forest);
    assert!(forest.len() > 500, "{} forest trees", forest.len());
    let share =
        |kind| forest.iter().filter(|t| t.kind == kind).count() as f32 / forest.len() as f32;
    let (oak, birch, spruce, big) = (
        share(TreeKind::Oak),
        share(TreeKind::Birch),
        share(TreeKind::Spruce),
        share(TreeKind::BigOak),
    );
    assert!((0.48..0.62).contains(&oak), "oak {oak}");
    assert!((0.26..0.40).contains(&birch), "birch {birch}");
    assert_eq!(spruce, 0.0);
    assert!((0.05..0.19).contains(&big), "big oak {big}");
}

#[test]
fn plains_grow_only_oak() {
    let plains = trees_in(Biome::Plains);
    assert!(plains.len() > 5, "{} plains trees", plains.len());
    assert!(plains.iter().all(|t| t.kind == TreeKind::Oak));
}

#[test]
fn birch_forest_grows_only_birch() {
    let birch = trees_in(Biome::BirchForest);
    assert!(birch.len() > 100, "{} birch forest trees", birch.len());
    assert!(birch.iter().all(|t| t.kind == TreeKind::Birch));
}

#[test]
fn spruce_grows_only_in_taiga_like_biomes() {
    let terrain = Terrain::new(1337);
    let spruces: Vec<&Tree> = of_kind(TreeKind::Spruce).collect();
    assert!(spruces.len() > 100, "{} spruces", spruces.len());
    assert!(spruces.iter().all(|t| matches!(
        terrain.biome_at(t.base.x, t.base.z),
        Biome::Taiga
            | Biome::TaigaHills
            | Biome::TaigaM
            | Biome::ColdTaiga
            | Biome::ColdTaigaHills
            | Biome::ColdTaigaM
            | Biome::MegaTaiga
            | Biome::MegaTaigaHills
            | Biome::MegaSpruceTaiga
            | Biome::MegaSpruceTaigaHills
            | Biome::ExtremeHillsPlus
            | Biome::ExtremeHillsPlusM
            | Biome::IcePlains
            | Biome::IceMountains
    )));
}

#[test]
fn taiga_grows_only_spruce() {
    let taiga = trees_in(Biome::Taiga);
    assert!(taiga.len() > 100, "{} taiga trees", taiga.len());
    assert!(taiga.iter().all(|t| t.kind == TreeKind::Spruce));
}

#[test]
fn only_wooded_biomes_grow_trees() {
    let terrain = Terrain::new(1337);
    for tree in forest_trees() {
        let biome = terrain.biome_at(tree.base.x, tree.base.z);
        assert!(trees_per_chunk(biome) > 0.0, "{biome:?}");
    }
}

#[test]
fn ice_plains_trees_are_rarer_than_taiga_trees() {
    let terrain = Terrain::new(1337);
    let cells = |biome| {
        (-1024..1024)
            .flat_map(|z| (-1024..1024).map(move |x| (x, z)))
            .filter(|&(x, z)| terrain.biome_at(x, z) == biome)
            .count() as f32
    };
    let ice = trees_in(Biome::IcePlains);
    assert!(ice.iter().all(|t| t.kind == TreeKind::Spruce));
    let ice_density = ice.len() as f32 / cells(Biome::IcePlains).max(1.0);
    let taiga_density = trees_in(Biome::Taiga).len() as f32 / cells(Biome::Taiga).max(1.0);
    assert!(
        ice_density * 10.0 < taiga_density,
        "ice {ice_density}, taiga {taiga_density}"
    );
}

#[test]
fn snow_covers_leaves_only_in_snowy_biomes() {
    let terrain = Terrain::new(1337);
    let key = (-64..64)
        .flat_map(|z| (-64..64).map(move |x| ColumnPos::new(x, z)))
        .find(|key| {
            let o = key.origin();
            matches!(
                terrain.sample_biome(o.x + 8, o.z + 8),
                Biome::ColdTaiga | Biome::IcePlains
            )
        });
    if let Some(key) = key {
        let column = terrain.generate_trees(key);
        let snow = (0..WORLD_HEIGHT).any(|y| {
            (0..16).any(|z| (0..16).any(|x| column.get(IVec3::new(x, y, z)) == Block::Snow))
        });
        assert!(snow);
    }
}

#[test]
fn no_trees_in_water_or_on_beaches() {
    let terrain = Terrain::new(1337);
    let mut low = 0;
    for z in -1024..1024 {
        for x in -1024..1024 {
            if terrain.surface_height(x, z) <= WATER_LEVEL + 1 {
                low += 1;
                assert_eq!(tree_at(&terrain, x, z), None, "{x}, {z}");
            }
        }
    }
    assert!(low > 10_000, "only {low} low columns");
}

#[test]
fn forest_is_denser_than_plains_and_desert_is_empty() {
    use std::collections::HashMap;
    let terrain = Terrain::new(1337);
    let mut counts: HashMap<Biome, (u32, u32)> = HashMap::new();
    for z in -1024..1024 {
        for x in -1024..1024 {
            if terrain.surface_height(x, z) <= WATER_LEVEL + 1 {
                continue;
            }
            let entry = counts.entry(terrain.biome_at(x, z)).or_default();
            entry.0 += 1;
            entry.1 += u32::from(tree_at(&terrain, x, z).is_some());
        }
    }
    let density = |biome| {
        let (area, trees) = counts.get(&biome).copied().unwrap_or_default();
        assert!(area > 5_000, "{biome:?}: {area} cells");
        trees as f32 / area as f32
    };
    let (forest, plains) = (density(Biome::Forest), density(Biome::Plains));
    assert!(forest > 0.01 && plains > 0.0003, "{forest} {plains}");
    assert!(forest > plains * 5.0, "{forest} {plains}");
    for biome in [Biome::Desert, Biome::Beach, Biome::River] {
        if let Some(&(_, trees)) = counts.get(&biome) {
            assert_eq!(trees, 0, "{biome:?}");
        }
    }
}

const KINDS: [TreeKind; 4] = [
    TreeKind::Oak,
    TreeKind::Birch,
    TreeKind::Spruce,
    TreeKind::BigOak,
];

#[test]
fn crowns_cross_columns_seamlessly() {
    for kind in KINDS {
        assert_seamless(kind);
    }
}

fn assert_seamless(kind: TreeKind) {
    let terrain = Terrain::new(1337);
    let tree = border_tree(&terrain, kind);
    let center = ColumnPos::of_block(tree.base);
    let leaf_columns: HashSet<_> = tree
        .leaves()
        .iter()
        .map(|l| (l.x.div_euclid(SECTION_SIZE), l.z.div_euclid(SECTION_SIZE)))
        .collect();
    assert!(leaf_columns.len() > 1, "{tree:?} does not cross a border");

    let size = 3 * SECTION_SIZE;
    let min = (center.0 - IVec2::ONE) * SECTION_SIZE;
    let index = |p: IVec3| ((p.x - min.x) + size * (p.z - min.y) + size * size * p.y) as usize;
    let inside = |p: IVec3| {
        (min.x..min.x + size).contains(&p.x)
            && (min.y..min.y + size).contains(&p.z)
            && (0..WORLD_HEIGHT).contains(&p.y)
    };
    let mut window = vec![Block::Air; (size * size * WORLD_HEIGHT) as usize];
    for cz in -1..=1 {
        for cx in -1..=1 {
            let key = center + IVec2::new(cx, cz);
            let ground = terrain.generate_carved(key);
            for y in 0..WORLD_HEIGHT {
                for z in 0..SECTION_SIZE {
                    for x in 0..SECTION_SIZE {
                        let local = IVec3::new(x, y, z);
                        let world = local + key.origin();
                        window[index(world)] = ground.get(local);
                    }
                }
            }
        }
    }
    let trees: Vec<Tree> = (min.y..min.y + size)
        .flat_map(|z| (min.x..min.x + size).map(move |x| (x, z)))
        .filter_map(|(x, z)| tree_at(&terrain, x, z))
        .collect();
    for tree in &trees {
        for (log, _) in tree.logs() {
            if inside(log) {
                window[index(log)] = tree.kind.log();
            }
        }
    }
    for tree in &trees {
        for leaf in tree.leaves() {
            if inside(leaf) && window[index(leaf)] == Block::Air {
                window[index(leaf)] = tree.kind.leaves();
            }
        }
    }

    let mut compared = 0;
    for cz in -1..=1 {
        for cx in -1..=1 {
            let key = center + IVec2::new(cx, cz);
            let column = terrain.generate_trees(key);
            for y in 0..WORLD_HEIGHT {
                for z in 0..SECTION_SIZE {
                    for x in 0..SECTION_SIZE {
                        let local = IVec3::new(x, y, z);
                        let world = local + key.origin();
                        let from_edge = [world.x - min.x, world.z - min.y]
                            .map(|c| c.min(size - 1 - c))
                            .into_iter()
                            .min()
                            .unwrap();
                        if from_edge >= MAX_REACH {
                            let without_snow = |block| match block {
                                Block::Snow => Block::Air,
                                block => block,
                            };
                            assert_eq!(
                                without_snow(column.get(local)),
                                without_snow(window[index(world)]),
                                "{world}"
                            );
                            compared += 1;
                        }
                    }
                }
            }
        }
    }
    assert!(compared > (SECTION_SIZE * SECTION_SIZE * WORLD_HEIGHT * 4) as usize);
}

#[test]
fn leaves_do_not_replace_logs_or_ground() {
    let terrain = Terrain::new(1337);
    let tree = border_tree(&terrain, TreeKind::BigOak);
    let key = ColumnPos::of_block(tree.base);
    let ground = terrain.generate_carved(key);
    let column = terrain.generate_trees(key);
    let origin = key.origin();
    for (log, axis) in tree.logs() {
        let local = log - origin;
        if Column::contains(local) {
            assert_eq!(
                (column.get(local), column.get_data(local)),
                (Block::Log, axis)
            );
        }
    }
    assert_eq!(column.get(tree.base - origin - IVec3::Y), Block::Grass);
    for y in 0..WORLD_HEIGHT {
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                let p = IVec3::new(x, y, z);
                if ground.get(p) != Block::Air {
                    assert_eq!(column.get(p), ground.get(p), "{p}");
                }
            }
        }
    }
}

fn snowy_spruce(terrain: &Terrain) -> Tree {
    (-150..150)
        .flat_map(|z| (-150..150).map(move |x| ColumnPos::new(x, z)))
        .filter(|key| {
            let o = key.origin();
            matches!(
                terrain.sample_biome(o.x + 8, o.z + 8),
                Biome::ColdTaiga | Biome::IcePlains
            )
        })
        .flat_map(|key| {
            let o = key.origin();
            (0..16).flat_map(move |z| (0..16).map(move |x| (o.x + x, o.z + z)))
        })
        .filter_map(|(x, z)| tree_at(terrain, x, z))
        .find(|t| t.kind == TreeKind::Spruce && terrain.biome_at(t.base.x, t.base.z).is_snowy())
        .expect("no snowy spruce")
}

#[test]
fn taiga_crowns_carry_snow_on_top_only() {
    let terrain = Terrain::new(7);
    let tree = snowy_spruce(&terrain);
    let key = ColumnPos::of_block(tree.base);
    let column = terrain.generate_trees(key);
    let origin = key.origin();
    let mut snowy = 0;
    for leaf in tree.leaves() {
        let local = leaf - origin;
        if !Column::contains(local) || !Column::contains(local + IVec3::Y) {
            continue;
        }
        if column.get(local) != Block::SpruceLeaves {
            continue;
        }
        let above = column.get(local + IVec3::Y);
        if above == Block::Snow {
            snowy += 1;
        } else {
            assert!(above.is_opaque() || above == Block::SpruceLeaves || above.is_log());
        }
    }
    assert!(snowy > 0, "no snow on the spruce at {}", tree.base);
}

#[test]
fn oak_crowns_outside_taiga_stay_bare() {
    let terrain = Terrain::new(1337);
    let tree = of_kind(TreeKind::Oak)
        .find(|t| terrain.biome_at(t.base.x, t.base.z) == Biome::Forest)
        .copied()
        .unwrap();
    let key = ColumnPos::of_block(tree.base);
    let column = terrain.generate_trees(key);
    for y in 0..WORLD_HEIGHT {
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                assert_ne!(column.get(IVec3::new(x, y, z)), Block::Snow);
            }
        }
    }
}
