use glam::IVec2;

use std::collections::HashSet;

use super::*;
use crate::column::WORLD_HEIGHT;

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
fn trees_keep_their_distance_and_shape() {
    let terrain = Terrain::new(1337);
    let mut trunks = HashSet::new();
    for z in -512..512 {
        for x in -512..512 {
            if let Some(tree) = tree_at(&terrain, x, z) {
                assert!((MIN_TRUNK..=MAX_TRUNK).contains(&tree.height));
                assert_eq!(tree.base.y, terrain.surface_height(x, z) + 1);
                let leaves: Vec<IVec3> = tree.leaves().collect();
                let spread = leaves
                    .iter()
                    .map(|l| (l.x - x).abs().max((l.z - z).abs()))
                    .max();
                assert_eq!(spread, Some(CROWN_RADIUS));
                // Над стволом лист, крона выше третьего бревна сверху не опускается.
                let top = tree.base.y + tree.height - 1;
                assert!(leaves.contains(&IVec3::new(x, top + 1, z)));
                assert!(leaves.iter().all(|l| (top - 2..=top + 1).contains(&l.y)));
                trunks.insert((x, z));
            }
        }
    }
    assert!(trunks.len() > 500, "only {} trees", trunks.len());
    for &(x, z) in &trunks {
        for dz in -(MIN_SPACING - 1)..MIN_SPACING {
            for dx in -(MIN_SPACING - 1)..MIN_SPACING {
                if (dx, dz) != (0, 0) {
                    assert!(!trunks.contains(&(x + dx, z + dz)), "{x}, {z} +{dx}, {dz}");
                }
            }
        }
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
    // Площадь — только суша: в воде и на пляжах деревьев нет в любом биоме.
    let terrain = Terrain::new(1337);
    let mut area = [0; 3];
    let mut trees = [0; 3];
    for z in -1024..1024 {
        for x in -1024..1024 {
            if terrain.surface_height(x, z) <= WATER_LEVEL + 1 {
                continue;
            }
            let biome = terrain.biome_at(x, z) as usize;
            area[biome] += 1;
            if tree_at(&terrain, x, z).is_some() {
                trees[biome] += 1;
            }
        }
    }
    let [plains, desert, forest] = [0, 1, 2].map(|b| trees[b] as f32 / area[b] as f32);
    assert!(area.iter().all(|&a| a > 10_000), "{area:?}");
    assert_eq!(trees[Biome::Desert as usize], 0, "{desert}");
    assert!(forest > 0.02 && plains > 0.0005, "{forest} {plains}");
    assert!(forest > plains * 5.0, "{forest} {plains}");
}

#[test]
fn crowns_cross_columns_seamlessly() {
    let terrain = Terrain::new(1337);
    let tree = border_tree(&terrain);
    let center = ColumnPos::of_block(tree.base);
    let leaf_columns: HashSet<_> = tree
        .leaves()
        .map(|l| (l.x.div_euclid(SECTION_SIZE), l.z.div_euclid(SECTION_SIZE)))
        .collect();
    assert!(leaf_columns.len() > 1, "{tree:?} does not cross a border");

    // «Окно» 3×3 колонок одним массивом: земля, затем деревья со стволами в окне
    // целиком, с записью за границы колонок.
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
    for log in trees.iter().flat_map(Tree::logs) {
        if inside(log) {
            window[index(log)] = Block::Log;
        }
    }
    for leaf in trees.iter().flat_map(Tree::leaves) {
        if inside(leaf) && window[index(leaf)] == Block::Air {
            window[index(leaf)] = Block::Leaves;
        }
    }

    // Колонки по отдельности совпадают с окном везде, куда не дотягиваются деревья
    // из-за его края.
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
                        if from_edge >= CROWN_RADIUS {
                            assert_eq!(column.get(local), window[index(world)], "{world}");
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
    let tree = border_tree(&terrain);
    let key = ColumnPos::of_block(tree.base);
    let ground = terrain.generate_carved(key);
    let column = terrain.generate_trees(key);
    let origin = key.origin();
    for log in tree.logs() {
        assert_eq!(column.get(log - origin), Block::Log);
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
