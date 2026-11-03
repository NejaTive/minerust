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
        if tree.kind.wide_trunk() || matches!(tree.kind, TreeKind::MegaJungle | TreeKind::Acacia) {
            continue;
        }
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
                    assert!(
                        near >= tree.kind.footprint() + other.footprint() - 1,
                        "{x}, {z} +{dx}, {dz}"
                    );
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

const KINDS: [TreeKind; 11] = [
    TreeKind::Oak,
    TreeKind::Birch,
    TreeKind::Spruce,
    TreeKind::BigOak,
    TreeKind::JungleTree,
    TreeKind::MegaJungle,
    TreeKind::Shrub,
    TreeKind::Acacia,
    TreeKind::DarkOak,
    TreeKind::BrownMushroom,
    TreeKind::RedMushroom,
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
    for tree in &trees {
        for (cell, _, drop) in tree.vines() {
            for step in 0..=drop {
                let at = cell - IVec3::Y * step;
                if inside(at) && window[index(at)] == Block::Air {
                    window[index(at)] = Block::Vine;
                } else {
                    break;
                }
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
    assert!(compared > (SECTION_SIZE * SECTION_SIZE * WORLD_HEIGHT * 3) as usize);
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

const JUNGLE_SEED: i32 = 4;

fn jungle_trees() -> &'static [Tree] {
    static TREES: OnceLock<Vec<Tree>> = OnceLock::new();
    TREES.get_or_init(|| {
        let terrain = Terrain::new(JUNGLE_SEED);
        (-256..512)
            .flat_map(|z| (-1280..-768).map(move |x| (x, z)))
            .filter_map(|(x, z)| tree_at(&terrain, x, z))
            .collect()
    })
}

fn all_trees() -> impl Iterator<Item = &'static Tree> {
    forest_trees().iter().chain(jungle_trees())
}

fn jungle_biome_of(tree: &Tree) -> Biome {
    Terrain::new(JUNGLE_SEED).biome_at(tree.base.x, tree.base.z)
}

fn extent(tree: &Tree) -> i32 {
    let logs = tree.logs().into_iter().map(|(pos, _)| pos);
    let vines = tree.vines().into_iter().map(|(pos, _, _)| pos);
    tree.leaves()
        .into_iter()
        .chain(logs)
        .chain(vines)
        .map(|p| (p.x - tree.base.x).abs().max((p.z - tree.base.z).abs()))
        .max()
        .unwrap()
}

#[test]
fn every_new_kind_grows() {
    for kind in [
        TreeKind::JungleTree,
        TreeKind::MegaJungle,
        TreeKind::Shrub,
        TreeKind::Acacia,
        TreeKind::DarkOak,
        TreeKind::BrownMushroom,
        TreeKind::RedMushroom,
    ] {
        assert!(
            all_trees().filter(|t| t.kind == kind).count() > 5,
            "{kind:?}"
        );
    }
}

#[test]
fn no_tree_reaches_further_than_the_planting_margin() {
    for tree in all_trees() {
        assert!(
            extent(tree) <= MAX_REACH,
            "{:?}: {}",
            tree.kind,
            extent(tree)
        );
    }
}

#[test]
fn new_trees_are_deterministic() {
    for tree in all_trees().filter(|t| t.kind.log() != Block::Log).take(400) {
        let copy = *tree;
        assert_eq!(tree.logs(), copy.logs());
        assert_eq!(tree.leaves(), copy.leaves());
        assert_eq!(tree.vines(), copy.vines());
    }
}

#[test]
fn jungle_grows_its_own_mix_and_the_edge_has_no_mega_trees() {
    let mut kinds = HashSet::new();
    for tree in jungle_trees() {
        let biome = jungle_biome_of(tree);
        if biome.is_jungle() {
            kinds.insert(tree.kind);
            assert!(
                matches!(
                    tree.kind,
                    TreeKind::BigOak
                        | TreeKind::Shrub
                        | TreeKind::MegaJungle
                        | TreeKind::JungleTree
                ),
                "{biome:?} {:?}",
                tree.kind
            );
        }
        if matches!(biome, Biome::JungleEdge | Biome::JungleEdgeM) {
            assert_ne!(tree.kind, TreeKind::MegaJungle);
        }
        if tree.kind == TreeKind::MegaJungle {
            assert!(matches!(
                biome,
                Biome::Jungle | Biome::JungleHills | Biome::JungleM
            ));
        }
    }
    assert_eq!(kinds.len(), 4);
}

#[test]
fn jungle_is_much_denser_than_its_edge() {
    let terrain = Terrain::new(JUNGLE_SEED);
    let density = |biome: Biome| {
        let (mut cells, mut trees) = (0, 0);
        for z in -256..512 {
            for x in -1280..-768 {
                if terrain.biome_at(x, z) == biome && terrain.surface_height(x, z) > WATER_LEVEL + 1
                {
                    cells += 1;
                    trees += i32::from(tree_at(&terrain, x, z).is_some());
                }
            }
        }
        trees as f32 / cells as f32
    };
    let (jungle, edge) = (density(Biome::Jungle), density(Biome::JungleEdge));
    assert!(jungle > 0.03, "{jungle}");
    assert!(jungle > edge * 5.0, "{jungle} {edge}");
}

#[test]
fn mega_jungle_has_a_whole_two_by_two_trunk() {
    let trees: Vec<_> = jungle_trees()
        .iter()
        .filter(|t| t.kind == TreeKind::MegaJungle)
        .collect();
    assert!(trees.len() > 10);
    for tree in trees {
        assert!((10..=31).contains(&tree.height));
        let logs: HashSet<IVec3> = tree.logs().into_iter().map(|(pos, _)| pos).collect();
        for dy in 0..tree.height - 1 {
            for (dx, dz) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                assert!(
                    logs.contains(&(tree.base + IVec3::new(dx, dy, dz))),
                    "{tree:?} {dx} {dy} {dz}"
                );
            }
        }
        assert!(logs.contains(&(tree.base + IVec3::Y * (tree.height - 1))));
        let terrain = Terrain::new(JUNGLE_SEED);
        for (dx, dz) in [(1, 0), (0, 1), (1, 1)] {
            assert_eq!(
                terrain.surface_height(tree.base.x + dx, tree.base.z + dz) + 1,
                tree.base.y
            );
        }
        assert!(extent(tree) >= 4);
    }
}

#[test]
fn shrub_is_one_jungle_log_in_oak_leaves() {
    let shrubs: Vec<_> = jungle_trees()
        .iter()
        .filter(|t| t.kind == TreeKind::Shrub)
        .collect();
    assert!(shrubs.len() > 10);
    for tree in shrubs {
        assert_eq!(tree.kind.log(), Block::JungleLog);
        assert_eq!(tree.kind.leaves(), Block::Leaves);
        assert_eq!(tree.logs(), vec![(tree.base, AXIS_Y)]);
        let leaves = tree.leaves();
        assert!(
            leaves
                .iter()
                .all(|l| (0..=2).contains(&(l.y - tree.base.y)))
        );
        assert_eq!(extent(tree), 2);
    }
}

#[test]
fn jungle_trees_are_draped_in_vines_that_touch_their_wood() {
    let mut draped = 0;
    for tree in jungle_trees()
        .iter()
        .filter(|t| matches!(t.kind, TreeKind::JungleTree | TreeKind::MegaJungle))
    {
        let vines = tree.vines();
        draped += usize::from(!vines.is_empty());
        let wood: HashSet<IVec3> = tree
            .logs()
            .into_iter()
            .map(|(pos, _)| pos)
            .chain(tree.leaves())
            .collect();
        for (cell, side, _) in vines {
            let (_, dir) = crate::block::VINE_SIDES
                .iter()
                .copied()
                .find(|&(bit, _)| bit == side)
                .expect("a single side bit");
            assert!(
                wood.contains(&(cell + dir)),
                "{:?} {cell} {side}",
                tree.kind
            );
        }
    }
    assert!(draped > 20);
}

#[test]
fn other_trees_grow_no_vines() {
    for tree in
        all_trees().filter(|t| !matches!(t.kind, TreeKind::JungleTree | TreeKind::MegaJungle))
    {
        assert!(tree.vines().is_empty(), "{:?}", tree.kind);
    }
}

#[test]
fn planted_jungle_column_has_vines_on_its_trunks() {
    let terrain = Terrain::new(JUNGLE_SEED);
    let tree = jungle_trees()
        .iter()
        .find(|t| t.kind == TreeKind::MegaJungle)
        .unwrap();
    let key = ColumnPos::of_block(tree.base);
    let column = terrain.generate_trees(key);
    let origin = key.origin();
    let vines = (0..WORLD_HEIGHT)
        .flat_map(|y| (0..16).flat_map(move |z| (0..16).map(move |x| IVec3::new(x, y, z))))
        .filter(|&p| column.get(p) == Block::Vine)
        .count();
    assert!(vines > 0);
    assert_eq!(
        column.get(tree.base - origin + IVec3::new(1, 1, 1)),
        Block::JungleLog
    );
}

#[test]
fn acacia_grows_in_savanna_with_a_bent_trunk_and_a_flat_crown() {
    let terrain = Terrain::new(1337);
    let acacias: Vec<_> = of_kind(TreeKind::Acacia).collect();
    assert!(acacias.len() > 100);
    for tree in &acacias {
        assert!(matches!(
            terrain.biome_at(tree.base.x, tree.base.z),
            Biome::Savanna | Biome::SavannaM | Biome::SavannaPlateau | Biome::SavannaPlateauM
        ));
        assert!((5..=9).contains(&tree.height));
        let logs = tree.logs();
        assert!(logs.iter().all(|&(_, axis)| axis == AXIS_Y));
        let mut ys: Vec<i32> = logs.iter().map(|(p, _)| p.y).collect();
        ys.sort_unstable();
        ys.dedup();
        assert_eq!(ys.first(), Some(&tree.base.y));
        assert!(ys.windows(2).all(|w| w[1] - w[0] == 1), "{ys:?}");
        let top = logs.iter().map(|(p, _)| p.y).max().unwrap();
        let widest = tree
            .leaves()
            .iter()
            .filter(|l| l.y == top)
            .map(|l| (l.x - logs.iter().find(|(p, _)| p.y == top).unwrap().0.x).abs())
            .max()
            .unwrap();
        assert!(widest >= 3);
    }
    let bent = acacias
        .iter()
        .filter(|t| {
            t.logs()
                .iter()
                .any(|(p, _)| (p.x, p.z) != (t.base.x, t.base.z))
        })
        .count();
    assert!(bent * 2 > acacias.len(), "{bent} of {}", acacias.len());
}

#[test]
fn savanna_is_mostly_acacia_with_some_oak() {
    let terrain = Terrain::new(1337);
    let savanna: Vec<_> = forest_trees()
        .iter()
        .filter(|t| terrain.biome_at(t.base.x, t.base.z) == Biome::Savanna)
        .collect();
    assert!(savanna.len() > 100);
    let acacia = savanna
        .iter()
        .filter(|t| t.kind == TreeKind::Acacia)
        .count() as f32
        / savanna.len() as f32;
    assert!((0.7..0.9).contains(&acacia), "{acacia}");
    assert!(
        savanna
            .iter()
            .all(|t| matches!(t.kind, TreeKind::Acacia | TreeKind::Oak))
    );
}

#[test]
fn roofed_forest_is_a_grid_of_dark_oaks_and_huge_mushrooms() {
    let terrain = Terrain::new(1337);
    let roofed: Vec<_> = forest_trees()
        .iter()
        .filter(|t| terrain.biome_at(t.base.x, t.base.z) == Biome::RoofedForest)
        .collect();
    assert!(roofed.len() > 200);
    let share = |f: fn(TreeKind) -> bool| {
        roofed.iter().filter(|t| f(t.kind)).count() as f32 / roofed.len() as f32
    };
    assert!(share(|k| k == TreeKind::DarkOak) > 0.6);
    let mushrooms = share(|k| matches!(k, TreeKind::BrownMushroom | TreeKind::RedMushroom));
    assert!((0.01..0.1).contains(&mushrooms), "{mushrooms}");
    assert!(roofed.iter().all(|t| matches!(
        t.kind,
        TreeKind::DarkOak
            | TreeKind::Oak
            | TreeKind::Birch
            | TreeKind::BrownMushroom
            | TreeKind::RedMushroom
    )));
    for tree in &roofed {
        let cell = |v: i32| (v - GRID_OFFSET).div_euclid(GRID_CELL);
        let (cx, cz) = (cell(tree.base.x), cell(tree.base.z));
        let (x, z, _) = grid_spot(1337, cx, cz);
        assert_eq!((x, z), (tree.base.x, tree.base.z));
    }
}

#[test]
fn dark_oak_has_a_two_by_two_trunk_and_a_wide_canopy() {
    for tree in of_kind(TreeKind::DarkOak) {
        assert!((6..=9).contains(&tree.height));
        let logs: HashSet<IVec3> = tree.logs().into_iter().map(|(pos, _)| pos).collect();
        for dy in 0..tree.height {
            let y = tree.base.y + dy;
            let whole = logs.iter().any(|p| {
                p.y == y
                    && [(1, 0), (0, 1), (1, 1)]
                        .into_iter()
                        .all(|(dx, dz)| logs.contains(&(*p + IVec3::new(dx, 0, dz))))
            });
            assert!(whole, "{tree:?} {dy}");
        }
        assert!(extent(tree) >= 3);
    }
}

#[test]
fn huge_mushrooms_have_a_stem_and_a_cap_of_their_own_blocks() {
    for (kind, cap) in [
        (TreeKind::BrownMushroom, Block::BrownMushroomBlock),
        (TreeKind::RedMushroom, Block::RedMushroomBlock),
    ] {
        let mushrooms: Vec<_> = of_kind(kind).collect();
        assert!(!mushrooms.is_empty(), "{kind:?}");
        assert_eq!(kind.log(), Block::MushroomStem);
        assert_eq!(kind.leaves(), cap);
        for tree in mushrooms {
            assert!([4, 5, 6, 8, 10, 12].contains(&tree.height));
            assert_eq!(tree.logs().len(), tree.height as usize);
            let top = tree.base.y + tree.height;
            let leaves = tree.leaves();
            assert!(leaves.contains(&IVec3::new(tree.base.x, top, tree.base.z)));
            assert!(leaves.iter().all(|l| (top - 3..=top).contains(&l.y)));
            let reach = if kind == TreeKind::BrownMushroom {
                3
            } else {
                2
            };
            assert_eq!(extent(tree), reach);
        }
    }
}
