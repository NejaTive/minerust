use super::*;

fn columns_of(terrain: &Terrain, biome: Biome, count: usize) -> Vec<ColumnPos> {
    (-300..300)
        .flat_map(|z| (-300..300).map(move |x| ColumnPos::new(x, z)))
        .filter(|k| terrain.sample_biome(k.origin().x + 8, k.origin().z + 8) == biome)
        .take(count)
        .collect()
}

fn count(column: &Column, pred: impl Fn(Block) -> bool) -> usize {
    (0..16)
        .flat_map(|z| (0..WORLD_HEIGHT).flat_map(move |y| (0..16).map(move |x| [x, y, z])))
        .filter(|&p| pred(column.get(p.into())))
        .count()
}

#[test]
fn same_seed_same_plants() {
    let key = ColumnPos::new(3, -2);
    let a = Terrain::new(42).generate(key);
    let b = Terrain::new(42).generate(key);
    for y in 0..WORLD_HEIGHT {
        for z in 0..16 {
            for x in 0..16 {
                assert_eq!(a.get([x, y, z].into()), b.get([x, y, z].into()));
            }
        }
    }
    assert!(count(&a, |b| b == Block::TallGrass) > 0 || count(&a, Block::needs_support) == 0);
}

#[test]
fn plains_have_reasonable_grass_share() {
    let terrain = Terrain::new(7);
    let (mut grass, mut flowers, mut surface) = (0, 0, 0);
    for key in columns_of(&terrain, Biome::Plains, 12) {
        let column = terrain.generate(key);
        grass += count(&column, |b| b == Block::TallGrass);
        flowers += count(&column, |b| matches!(b, Block::Poppy | Block::Dandelion));
        for z in 0..16 {
            for x in 0..16 {
                let top = (0..WORLD_HEIGHT)
                    .rev()
                    .find(|&y| column.get([x, y, z].into()).is_solid());
                if top.is_some_and(|y| column.get([x, y, z].into()) == Block::Grass) {
                    surface += 1;
                }
            }
        }
    }
    let share = grass as f32 / surface as f32;
    assert!((0.06..0.3).contains(&share), "grass share {share}");
    assert!(
        flowers > 0 && flowers < grass / 3,
        "{flowers} flowers, {grass} grass"
    );
}

#[test]
fn desert_has_no_meadow_plants() {
    let terrain = Terrain::new(7);
    for key in columns_of(&terrain, Biome::Desert, 6) {
        let column = terrain.generate(key);
        for z in 0..16 {
            for x in 0..16 {
                let has_plant = (0..WORLD_HEIGHT).any(|y| {
                    matches!(
                        column.get([x, y, z].into()),
                        Block::TallGrass | Block::Poppy | Block::Dandelion
                    )
                });
                let biome = terrain.biome_at(key.origin().x + x, key.origin().z + z);
                assert!(!has_plant || biome != Biome::Desert, "{key} {x},{z}");
            }
        }
    }
}

#[test]
fn plants_stand_on_dry_grass_only() {
    let terrain = Terrain::new(7);
    for key in columns_of(&terrain, Biome::Forest, 6)
        .into_iter()
        .chain(columns_of(&terrain, Biome::Plains, 6))
    {
        let column = terrain.generate(key);
        for y in 0..WORLD_HEIGHT {
            for z in 0..16 {
                for x in 0..16 {
                    let block = column.get([x, y, z].into());
                    let single = !block.is_mushroom()
                        && block.double_lower().is_none()
                        && block.double_upper().is_none();
                    if block.needs_support()
                        && !matches!(block, Block::Snow | Block::Reeds)
                        && single
                    {
                        assert_eq!(column.get([x, y - 1, z].into()), Block::Grass);
                        assert!(y - 1 > WATER_LEVEL, "plant {block:?} at y {y} near water");
                    }
                }
            }
        }
    }
}

fn cell_counts(column: &Column, pred: impl Fn(Block) -> bool) -> Vec<usize> {
    let mut cells = vec![0; 16];
    for y in 0..WORLD_HEIGHT {
        for z in 0..16 {
            for x in 0..16 {
                if pred(column.get([x, y, z].into())) {
                    cells[(z / 4 * 4 + x / 4) as usize] += 1;
                }
            }
        }
    }
    cells
}

#[test]
fn attempts_follow_vanilla_per_chunk_counters() {
    let table = [
        (Biome::Plains, 4, 10),
        (Biome::SunflowerPlains, 4, 10),
        (Biome::Forest, 2, 2),
        (Biome::RoofedForest, 2, 2),
        (Biome::FlowerForest, 100, 2),
        (Biome::Taiga, 2, 1),
        (Biome::ColdTaiga, 2, 1),
        (Biome::MegaTaiga, 2, 7),
        (Biome::Swampland, 1, 5),
        (Biome::Jungle, 4, 25),
        (Biome::Savanna, 4, 20),
        (Biome::ExtremeHills, 2, 1),
        (Biome::Desert, 0, 0),
        (Biome::MushroomIsland, 0, 0),
    ];
    for (biome, flowers, grass) in table {
        let got = attempts(biome);
        assert_eq!((got.flowers, got.grass), (flowers, grass), "{biome:?}");
    }
}

#[test]
fn grass_grows_in_patches_with_gaps() {
    let terrain = Terrain::new(7);
    let (mut dense, mut empty) = (0, 0);
    for key in columns_of(&terrain, Biome::Plains, 20) {
        let column = terrain.generate(key);
        for cell in cell_counts(&column, |b| b == Block::TallGrass) {
            dense += usize::from(cell >= 4);
            empty += usize::from(cell == 0);
        }
    }
    assert!(dense > 10, "{dense} dense cells");
    assert!(empty > 20, "{empty} empty cells");
}

#[test]
fn flowers_come_in_clusters_and_many_columns_have_none() {
    let terrain = Terrain::new(7);
    let totals: Vec<usize> = columns_of(&terrain, Biome::Plains, 60)
        .into_iter()
        .map(|key| {
            count(&terrain.generate(key), |b| {
                b == Block::Dandelion || b == Block::Poppy
            })
        })
        .collect();
    assert!(totals.contains(&0), "{totals:?}");
    assert!(totals.iter().any(|&n| n >= 10), "{totals:?}");
}

#[test]
fn taiga_grass_is_mostly_fern_and_swamp_flowers_are_orchids() {
    let terrain = Terrain::new(7);
    let (mut ferns, mut tall) = (0, 0);
    for key in columns_of(&terrain, Biome::MegaTaiga, 30) {
        let column = terrain.generate(key);
        ferns += count(&column, |b| b == Block::Fern);
        tall += count(&column, |b| b == Block::TallGrass);
    }
    let share = ferns as f32 / (ferns + tall).max(1) as f32;
    assert!((0.7..0.9).contains(&share), "fern share {share}");
    for key in columns_of(&terrain, Biome::Swampland, 30) {
        let column = terrain.generate(key);
        assert_eq!(count(&column, |b| b == Block::Dandelion), 0);
        assert_eq!(count(&column, |b| b == Block::Poppy), 0);
    }
}

#[test]
fn flower_forest_flowers_form_stripes_by_kind() {
    let terrain = Terrain::new(7);
    let mut flowers = Vec::new();
    for key in columns_of(&terrain, Biome::FlowerForest, 40) {
        let column = terrain.generate(key);
        let origin = key.origin();
        for y in 0..WORLD_HEIGHT {
            for z in 0..16 {
                for x in 0..16 {
                    let block = column.get([x, y, z].into());
                    if FLOWER_FOREST_PICKS.contains(&block) {
                        flowers.push((origin.x + x, origin.z + z, block));
                    }
                }
            }
        }
    }
    assert!(flowers.len() > 500, "{} flowers", flowers.len());
    let (mut near_same, mut near, mut far_same, mut far) = (0, 0, 0, 0);
    for (i, a) in flowers.iter().enumerate().step_by(7) {
        for b in &flowers[i + 1..] {
            let dist = (a.0 - b.0).abs().max((a.1 - b.1).abs());
            if dist <= 3 {
                near += 1;
                near_same += usize::from(a.2 == b.2);
            } else if dist >= 30 {
                far += 1;
                far_same += usize::from(a.2 == b.2);
            }
        }
    }
    let (near_share, far_share) = (
        near_same as f32 / near.max(1) as f32,
        far_same as f32 / far.max(1) as f32,
    );
    assert!(
        near_share > far_share + 0.3,
        "near {near_share}, far {far_share}"
    );
}

#[test]
fn flower_forest_is_far_denser_in_flowers_than_forest() {
    let terrain = Terrain::new(7);
    let per_column = |biome| {
        let keys = columns_of(&terrain, biome, 30);
        let total: usize = keys
            .iter()
            .map(|&k| count(&terrain.generate(k), |b| FLOWER_FOREST_PICKS.contains(&b)))
            .sum();
        total as f32 / keys.len().max(1) as f32
    };
    let (flower_forest, forest) = (per_column(Biome::FlowerForest), per_column(Biome::Forest));
    assert!(flower_forest > forest * 5.0, "{flower_forest} vs {forest}");
}
