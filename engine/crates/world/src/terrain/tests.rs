use super::*;
use crate::biome_color::BiomePalette;

const RUN_LENGTH: (f32, f32) = (20.0, 400.0);
const MAX_BORDER_STEP: i32 = 12;
const SNOW_SEED: i32 = 7;

fn same_columns(a: &Column, b: &Column) -> bool {
    (0..WORLD_HEIGHT).all(|y| {
        (0..SECTION_SIZE).all(|z| {
            (0..SECTION_SIZE).all(|x| {
                let p = IVec3::new(x, y, z);
                a.get(p) == b.get(p)
            })
        })
    })
}

#[test]
fn same_seed_same_world() {
    let (a, b) = (Terrain::new(42), Terrain::new(42));
    for (x, z) in [(0, 0), (-1000, 37), (123_456, -98_765)] {
        assert_eq!(a.surface_height(x, z), b.surface_height(x, z));
        assert_eq!(a.biome_at(x, z), b.biome_at(x, z));
    }
    for key in [ColumnPos::new(-3, 7), lake_key(&a)] {
        assert!(same_columns(&a.generate(key), &b.generate(key)));
    }
}

#[test]
fn other_seed_other_world() {
    let (a, b) = (Terrain::new(42), Terrain::new(43));
    let differs =
        (0..64).filter(|&i| a.surface_height(i * 7, i * 3) != b.surface_height(i * 7, i * 3));
    assert!(differs.count() > 32);
}

#[test]
fn biome_at_matches_the_uncached_map() {
    let terrain = Terrain::new(77);
    for (x, z) in [(0, 0), (-1, -1), (15, 16), (-1234, 987), (4000, -4000)] {
        assert_eq!(
            terrain.biome_at(x, z),
            terrain.sample_biome(x, z),
            "{x}, {z}"
        );
    }
}

#[test]
fn surface_stays_in_range() {
    let terrain = Terrain::new(1337);
    let (mut min, mut max) = (i32::MAX, i32::MIN);
    for z in (-4096..4096).step_by(16) {
        for x in (-4096..4096).step_by(16) {
            let h = terrain.surface_height(x, z);
            assert!((1..WORLD_HEIGHT - 1).contains(&h), "{h} at {x}, {z}");
            min = min.min(h);
            max = max.max(h);
        }
    }
    assert!(max - min > 40, "{min}..{max}");
    assert!(min > 15, "{min}..{max}");
}

fn find_column(terrain: &Terrain, pred: impl Fn(i32, i32, Biome) -> bool) -> Option<(i32, i32)> {
    (0..16_384)
        .map(|i| (i * 8, i * 3))
        .find(|&(x, z)| pred(x, z, terrain.sample_biome(x, z)))
}

fn find_biome(terrain: &Terrain, biome: Biome) -> (i32, i32) {
    find_column(terrain, |x, z, b| {
        b == biome && terrain.biome_at(x, z) == biome && terrain.surface_height(x, z) > WATER_LEVEL
    })
    .unwrap_or_else(|| panic!("no {biome:?}"))
}

#[test]
fn oceans_lie_under_water_and_their_floor_is_gravel() {
    let terrain = Terrain::new(1337);
    let (x, z) = find_column(&terrain, |x, z, b| {
        b == Biome::DeepOcean && terrain.surface_height(x, z) < WATER_LEVEL - 14
    })
    .expect("no deep ocean");
    let (top, layers) = ground_layers(&terrain, x, z);
    assert!(top < WATER_LEVEL - 14);
    assert_eq!(layers[0], Block::Gravel, "deep water floor is gravel");
    assert!(layers.iter().all(|&b| b != Block::Water));
}

#[test]
fn rivers_are_below_the_water_level_and_hills_above() {
    let terrain = Terrain::new(1337);
    let mean = |biome: Biome| {
        let heights: Vec<i32> = (0..40_000)
            .map(|i| (i * 13 - 200_000, i * 7 - 100_000))
            .filter(|&(x, z)| terrain.sample_biome(x, z) == biome)
            .take(200)
            .map(|(x, z)| terrain.surface_height(x, z))
            .collect();
        assert!(heights.len() > 20, "{biome:?}: {} samples", heights.len());
        heights.iter().sum::<i32>() as f32 / heights.len() as f32
    };
    assert!(mean(Biome::River) < WATER_LEVEL as f32 - 2.0);
    assert!(mean(Biome::Ocean) < mean(Biome::River) - 2.0);
    assert!(mean(Biome::DeepOcean) < mean(Biome::Ocean) - 3.0);
    assert!(mean(Biome::ExtremeHills) > mean(Biome::Plains) + 10.0);
}

#[test]
fn water_fills_column_up_to_level() {
    let terrain = Terrain::new(1);
    let (x, z) = find_column(&terrain, |x, z, b| {
        !b.is_snowy() && terrain.surface_height(x, z) == 55
    })
    .expect("no column with surface 55");
    let column = terrain.generate(ColumnPos::new(x.div_euclid(16), z.div_euclid(16)));
    let at = |y| column.get(IVec3::new(x.rem_euclid(16), y, z.rem_euclid(16)));
    assert!(at(55).is_solid());
    assert!((56..=WATER_LEVEL).all(|y| at(y) == Block::Water));
    assert!((WATER_LEVEL + 1..WORLD_HEIGHT).all(|y| at(y) == Block::Air));
}

#[test]
fn beaches_are_sand() {
    let terrain = Terrain::new(1337);
    let (x, z) = find_biome(&terrain, Biome::Beach);
    let (_, layers) = ground_layers(&terrain, x, z);
    let sand = layers.iter().take_while(|&&b| b == Block::Sand).count();
    assert!(sand >= 2, "{sand}");
    let rest = layers[sand..]
        .iter()
        .skip_while(|&&b| b == Block::Sandstone);
    assert!(
        rest.clone()
            .all(|&b| matches!(b, Block::Stone | Block::Bedrock))
    );
}

#[test]
fn stone_beach_is_stone() {
    let terrain = Terrain::new(1337);
    let (x, z) = find_biome(&terrain, Biome::StoneBeach);
    let (_, layers) = ground_layers(&terrain, x, z);
    assert!(
        layers
            .iter()
            .all(|&b| matches!(b, Block::Stone | Block::Bedrock))
    );
}

fn ground_layers(terrain: &Terrain, x: i32, z: i32) -> (i32, Vec<Block>) {
    let key = ColumnPos::new(x.div_euclid(16), z.div_euclid(16));
    let column = terrain.generate_ground(key);
    let local = |y| IVec3::new(x.rem_euclid(16), y, z.rem_euclid(16));
    let top = terrain.surface_height(x, z);
    for y in top + 1..WORLD_HEIGHT {
        let found = column.get(local(y));
        let allowed = match found {
            Block::Water => y <= WATER_LEVEL,
            Block::Ice => y == WATER_LEVEL,
            Block::Snow => y == top + 1,
            Block::Air => y > WATER_LEVEL,
            _ => false,
        };
        assert!(allowed, "y {y}: {found:?}");
    }
    (top, (0..=top).rev().map(|y| column.get(local(y))).collect())
}

#[test]
fn grass_dirt_stone_outside_desert() {
    let terrain = Terrain::new(1337);
    for biome in [Biome::Plains, Biome::Forest] {
        let (x, z) = find_biome(&terrain, biome);
        let (_, layers) = ground_layers(&terrain, x, z);
        assert_eq!(layers[0], Block::Grass);
        let dirt = layers[1..]
            .iter()
            .take_while(|&&b| b == Block::Dirt)
            .count();
        assert!((1..=8).contains(&dirt), "{dirt}");
        assert!(
            layers[1 + dirt..]
                .iter()
                .all(|&b| matches!(b, Block::Stone | Block::Bedrock))
        );
    }
}

#[test]
fn sand_over_stone_in_desert() {
    let terrain = Terrain::new(1337);
    let (x, z) = find_biome(&terrain, Biome::Desert);
    let (_, layers) = ground_layers(&terrain, x, z);
    let sand = layers.iter().take_while(|&&b| b == Block::Sand).count();
    assert!(sand >= 2, "{sand}");
    assert!(
        layers[sand..]
            .iter()
            .all(|&b| matches!(b, Block::Sandstone | Block::Stone | Block::Bedrock))
    );
}

#[test]
fn extreme_hills_mix_grass_and_bare_stone() {
    let terrain = Terrain::new(1337);
    let (mut stone, mut grass) = (0, 0);
    for key in columns_of(&terrain, |b| b == Biome::ExtremeHills, 40) {
        let column = terrain.generate_ground(key);
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                let top = terrain.surface_height(key.origin().x + x, key.origin().z + z);
                if top <= WATER_LEVEL {
                    continue;
                }
                match column.get(IVec3::new(x, top, z)) {
                    Block::Stone => stone += 1,
                    Block::Grass | Block::SnowyGrass => grass += 1,
                    _ => {}
                }
            }
        }
    }
    assert!(stone > 100 && grass > 100, "{stone} stone, {grass} grass");
}

pub(super) fn mean_run_length<K: PartialEq>(terrain: &Terrain, class: impl Fn(Biome) -> K) -> f32 {
    let (mut cells, mut runs) = (0, 0);
    for row in (-8000..8000).step_by(97) {
        let line = terrain.map.blocks(-8000, row, 16_000, 1);
        let mut previous = None;
        for &biome in line.iter().step_by(4) {
            let class = class(biome);
            runs += i32::from(previous.as_ref() != Some(&class));
            cells += 4;
            previous = Some(class);
        }
    }
    cells as f32 / runs as f32
}

#[test]
fn biomes_are_deterministic() {
    let (a, b) = (Terrain::new(42), Terrain::new(42));
    for (x, z) in [(0, 0), (-1000, 37), (123_456, -98_765)] {
        assert_eq!(a.biome_at(x, z), b.biome_at(x, z));
    }
}

#[test]
fn biomes_have_a_sensible_size() {
    for seed in [1337, 42, 7, 99] {
        let length = mean_run_length(&Terrain::new(seed), |biome| biome);
        assert!(
            (RUN_LENGTH.0..RUN_LENGTH.1).contains(&length),
            "seed {seed}: mean run {length}"
        );
    }
}

#[test]
fn all_climate_groups_cover_a_real_share_of_the_world() {
    for seed in [1337, 42, 7] {
        let terrain = Terrain::new(seed);
        let (mut water, mut snowy, mut desert, mut green) = (0, 0, 0, 0);
        let mut total = 0;
        for z in (-8000..8000).step_by(160) {
            for &biome in &terrain
                .map
                .blocks(-8000, z, 16_000, 1)
                .into_iter()
                .step_by(160)
                .collect::<Vec<_>>()
            {
                total += 1;
                if biome.is_ocean() {
                    water += 1;
                } else if biome.is_snowy() {
                    snowy += 1;
                } else if biome.is_desert() || biome.is_mesa() {
                    desert += 1;
                } else {
                    green += 1;
                }
            }
        }
        let share = |n: i32| n as f32 / total as f32;
        assert!(
            (0.2..0.55).contains(&share(water)),
            "seed {seed}: water {}",
            share(water)
        );
        assert!(
            (0.005..0.2).contains(&share(snowy)),
            "seed {seed}: snowy {}",
            share(snowy)
        );
        assert!(
            (0.03..0.35).contains(&share(desert)),
            "seed {seed}: desert {}",
            share(desert)
        );
        assert!(share(green) > 0.25, "seed {seed}: green {}", share(green));
    }
}

#[test]
fn surface_is_deterministic() {
    let (a, b) = (Terrain::new(42), Terrain::new(42));
    for (x, z) in [(0, 0), (-1000, 37), (123_456, -98_765)] {
        assert_eq!(a.surface_height(x, z), b.surface_height(x, z));
    }
}

fn row_steps(terrain: &Terrain, row: i32, from: i32, to: i32) -> Vec<(i32, Biome, Biome)> {
    let mut previous: Option<(i32, Biome)> = None;
    let mut steps = Vec::new();
    for x in from..to {
        let height = terrain.surface_height(x, row);
        let biome = terrain.biome_at(x, row);
        if let Some((ph, pb)) = previous {
            steps.push(((height - ph).abs(), pb, biome));
        }
        previous = Some((height, biome));
    }
    steps
}

#[test]
fn terrain_is_smooth_across_biome_borders() {
    let (mut border, mut border_count, mut inside, mut inside_count) = (0, 0, 0, 0);
    let mut worst = 0;
    for seed in [1337, 42] {
        let terrain = Terrain::new(seed);
        for row in (-1500..1500).step_by(97) {
            for (step, a, b) in row_steps(&terrain, row, -1500, 1500) {
                if a != b {
                    border += step;
                    border_count += 1;
                    worst = worst.max(step);
                } else {
                    inside += step;
                    inside_count += 1;
                }
            }
        }
    }
    let (border, inside) = (
        border as f32 / border_count as f32,
        inside as f32 / inside_count as f32,
    );
    assert!(border_count > 100, "{border_count} border steps");
    assert!(worst <= MAX_BORDER_STEP, "worst border step {worst}");
    assert!(
        border <= inside * 2.0 + 0.5,
        "border {border}, inside {inside}"
    );
}

fn flat() -> Terrain {
    Terrain::of(42, WorldType::Flat)
}

#[test]
fn flat_column_has_fixed_layers() {
    let column = flat().generate(ColumnPos::new(-5, 9));
    for (x, z) in [(0, 0), (7, 3), (15, 15)] {
        let at = |y| column.get(IVec3::new(x, y, z));
        assert_eq!(at(0), Block::Bedrock);
        assert_eq!(at(1), Block::Stone);
        assert_eq!((at(2), at(3)), (Block::Dirt, Block::Dirt));
        assert_eq!(at(4), Block::Grass);
        assert!((5..WORLD_HEIGHT).all(|y| at(y) == Block::Air));
    }
}

#[test]
fn flat_world_has_no_features() {
    let terrain = flat();
    for key in [
        ColumnPos::ZERO,
        ColumnPos::new(-30, 17),
        ColumnPos::new(8, 8),
    ] {
        let column = terrain.generate(key);
        assert!(same_columns(&column, &terrain.generate_ground(key)));
        let counts = |block| {
            (0..WORLD_HEIGHT)
                .flat_map(|y| (0..16).flat_map(move |z| (0..16).map(move |x| (x, y, z))))
                .filter(|&(x, y, z)| column.get(IVec3::new(x, y, z)) == block)
                .count()
        };
        for block in [
            Block::Water,
            Block::Lava,
            Block::Gravel,
            Block::Sand,
            Block::Log,
            Block::Leaves,
            Block::TallGrass,
            Block::Poppy,
            Block::Dandelion,
        ] {
            assert_eq!(counts(block), 0, "{block:?}");
        }
        assert_eq!(counts(Block::Air), (WORLD_HEIGHT as usize - 5) * 256);
    }
}

#[test]
fn flat_terrain_answers_with_a_plain() {
    let terrain = flat();
    for (x, z) in [(0, 0), (-1000, 37), (123_456, -98_765)] {
        assert_eq!(terrain.surface_height(x, z), FLAT_SURFACE);
        assert_eq!(terrain.biome_at(x, z), Biome::Plains);
        assert!(crate::trees::tree_at(&terrain, x, z).is_none());
    }
}

#[test]
fn standard_type_is_the_default() {
    let (a, b) = (Terrain::new(42), Terrain::of(42, WorldType::Standard));
    assert_eq!(a.world_type(), WorldType::Standard);
    assert!(same_columns(
        &a.generate(ColumnPos::new(3, 4)),
        &b.generate(ColumnPos::new(3, 4))
    ));
    assert_eq!(WorldType::from_name("flat"), Some(WorldType::Flat));
    assert_eq!(WorldType::from_name("standard"), Some(WorldType::Standard));
    assert_eq!(WorldType::from_name("x"), None);
}

fn bedrock_columns() -> Vec<Column> {
    let terrain = Terrain::new(7);
    [(0, 0), (-9, 4), (30, -22), (5, 61)]
        .into_iter()
        .map(|(x, z)| terrain.generate(ColumnPos::new(x, z)))
        .collect()
}

#[test]
fn standard_floor_is_solid_bedrock() {
    for column in bedrock_columns() {
        for z in 0..16 {
            for x in 0..16 {
                assert_eq!(column.get(IVec3::new(x, 0, z)), Block::Bedrock);
            }
        }
    }
}

#[test]
fn bedrock_scatter_thins_out_upwards() {
    let columns = bedrock_columns();
    let share = |y| {
        let hits = columns
            .iter()
            .flat_map(|c| (0..256).map(move |i| c.get(IVec3::new(i % 16, y, i / 16))))
            .filter(|&b| b == Block::Bedrock)
            .count();
        hits as f32 / (columns.len() * 256) as f32
    };
    for (y, expected) in [(1, 0.8), (2, 0.6), (3, 0.4), (4, 0.2)] {
        assert!((share(y) - expected).abs() < 0.07, "y={y}: {}", share(y));
    }
    assert_eq!(share(5), 0.0);
}

#[test]
fn caves_and_lava_leave_bedrock_alone() {
    let terrain = Terrain::new(7);
    let ground = terrain.generate_ground(ColumnPos::new(2, 2));
    let carved = terrain.generate(ColumnPos::new(2, 2));
    for y in 0..=4 {
        for z in 0..16 {
            for x in 0..16 {
                let at = IVec3::new(x, y, z);
                if ground.get(at) == Block::Bedrock {
                    assert_eq!(carved.get(at), Block::Bedrock);
                }
            }
        }
    }
}

#[test]
fn bedrock_is_deterministic() {
    let key = ColumnPos::new(-4, 9);
    assert!(same_columns(
        &Terrain::new(3).generate(key),
        &Terrain::new(3).generate(key)
    ));
}

fn columns_of(terrain: &Terrain, wanted: impl Fn(Biome) -> bool, limit: usize) -> Vec<ColumnPos> {
    (-300..300)
        .step_by(2)
        .flat_map(|z| (-300..300).step_by(2).map(move |x| ColumnPos::new(x, z)))
        .filter(|key| {
            let origin = key.origin();
            wanted(terrain.sample_biome(origin.x + 8, origin.z + 8))
                && (0..SECTION_SIZE)
                    .step_by(5)
                    .flat_map(|d| [(d, d), (d, SECTION_SIZE - 1 - d)])
                    .all(|(dx, dz)| wanted(terrain.biome_at(origin.x + dx, origin.z + dz)))
        })
        .take(limit)
        .collect()
}

#[test]
fn snowy_surface_is_snowy_grass_under_a_snow_layer() {
    let terrain = Terrain::new(SNOW_SEED);
    for biome in [Biome::IcePlains, Biome::ColdTaiga] {
        let keys = columns_of(&terrain, |b| b == biome, 12);
        assert!(!keys.is_empty(), "no {biome:?} columns");
        let mut checked = 0;
        for key in keys {
            let column = terrain.generate_ground(key);
            for z in 0..SECTION_SIZE {
                for x in 0..SECTION_SIZE {
                    let (wx, wz) = (key.origin().x + x, key.origin().z + z);
                    let surface = terrain.surface_height(wx, wz);
                    let at = |y| column.get(IVec3::new(x, y, z));
                    if surface < WATER_LEVEL || terrain.biome_at(wx, wz) != biome {
                        continue;
                    }
                    if matches!(at(surface), Block::Sand | Block::Bedrock | Block::Gravel) {
                        continue;
                    }
                    assert_eq!(at(surface), Block::SnowyGrass, "{wx}, {wz}");
                    assert_eq!(at(surface + 1), Block::Snow, "{wx}, {wz}");
                    assert_eq!(at(surface + 2), Block::Air, "{wx}, {wz}");
                    assert_eq!(at(surface - 1), Block::Dirt, "{wx}, {wz}");
                    checked += 1;
                }
            }
        }
        assert!(checked > 500, "{biome:?}: {checked} cells");
    }
}

#[test]
fn other_biomes_stay_green_and_snowless() {
    let terrain = Terrain::new(1337);
    for biome in [Biome::Plains, Biome::Forest, Biome::Desert, Biome::Taiga] {
        let (x, z) = find_biome(&terrain, biome);
        let column = terrain.generate_ground(ColumnPos::new(x.div_euclid(16), z.div_euclid(16)));
        for y in 0..WORLD_HEIGHT {
            for lz in 0..SECTION_SIZE {
                for lx in 0..SECTION_SIZE {
                    let block = column.get(IVec3::new(lx, y, lz));
                    let snowy_neighbour = terrain
                        .biome_at(key_origin(x, z).0 + lx, key_origin(x, z).1 + lz)
                        .is_snowy();
                    assert!(
                        snowy_neighbour || (block != Block::Ice && block != Block::Snow),
                        "{biome:?}"
                    );
                }
            }
        }
    }
}

fn key_origin(x: i32, z: i32) -> (i32, i32) {
    (x.div_euclid(16) * 16, z.div_euclid(16) * 16)
}

#[test]
fn snowy_water_is_capped_with_ice() {
    let terrain = Terrain::new(SNOW_SEED);
    let mut frozen = 0;
    let mut deep = 0;
    for key in columns_of(&terrain, Biome::is_snowy, 400) {
        let column = terrain.generate_ground(key);
        for lz in 0..SECTION_SIZE {
            for lx in 0..SECTION_SIZE {
                let (wx, wz) = (key.origin().x + lx, key.origin().z + lz);
                let surface = terrain.surface_height(wx, wz);
                if surface >= WATER_LEVEL || !terrain.biome_at(wx, wz).is_snowy() {
                    continue;
                }
                let at = |y| column.get(IVec3::new(lx, y, lz));
                assert_eq!(at(WATER_LEVEL), Block::Ice, "{wx}, {wz}");
                assert_eq!(at(WATER_LEVEL + 1), Block::Air, "{wx}, {wz}");
                assert!((surface + 1..WATER_LEVEL).all(|y| at(y) == Block::Water));
                frozen += 1;
                deep += i32::from(surface + 1 < WATER_LEVEL);
            }
        }
    }
    assert!(frozen > 200, "{frozen} frozen cells");
    assert!(deep > 100, "{deep} cells with water under the ice");
}

#[test]
fn warm_water_is_never_frozen() {
    let terrain = Terrain::new(1337);
    for key in columns_of(&terrain, |b| b == Biome::Ocean, 6) {
        let column = terrain.generate_ground(key);
        for lz in 0..SECTION_SIZE {
            for lx in 0..SECTION_SIZE {
                assert_ne!(column.get(IVec3::new(lx, WATER_LEVEL, lz)), Block::Ice);
            }
        }
    }
}

#[test]
fn snow_layer_follows_the_ground_through_caves() {
    let terrain = Terrain::new(SNOW_SEED);
    for key in columns_of(&terrain, |b| b == Biome::IcePlains, 12) {
        let column = terrain.generate_carved(key);
        for y in 1..WORLD_HEIGHT {
            for z in 0..SECTION_SIZE {
                for x in 0..SECTION_SIZE {
                    if column.get(IVec3::new(x, y, z)) == Block::Snow {
                        let below = column.get(IVec3::new(x, y - 1, z));
                        assert!(below.is_solid(), "snow on {below:?} at {x}, {y}, {z}");
                    }
                }
            }
        }
    }
}

#[test]
fn flat_and_debug_worlds_are_all_plains() {
    for world_type in [WorldType::Flat, WorldType::Debug] {
        let terrain = Terrain::of(1337, world_type);
        for i in 0..500 {
            assert_eq!(terrain.biome_at(i * 97, i * -61), Biome::Plains);
            assert_eq!(terrain.sample_biome(i * 97, i * -61), Biome::Plains);
        }
    }
}

#[test]
fn tint_bytes_hold_grass_foliage_then_water_for_every_block() {
    let terrain = Terrain::new(7);
    let palette = BiomePalette::constants();
    let bytes = terrain.column_tint_bytes(ColumnPos::new(3, -5), &palette);
    let plane = 16 * 16 * 4;
    assert_eq!(bytes.len(), 3 * plane);
    assert!(bytes.chunks(4).all(|p| p[3] == 255));
    assert_ne!(bytes[..3], bytes[plane..plane + 3]);
    assert!(bytes[2 * plane..].chunks(4).all(|p| p[..3] == [255; 3]));
}

#[test]
fn flat_world_tint_is_plains_everywhere() {
    let terrain = Terrain::of(1, WorldType::Flat);
    let palette = BiomePalette::constants();
    let bytes = terrain.column_tint_bytes(ColumnPos::new(0, 0), &palette);
    let [r, g, b] = palette
        .grass(Biome::Plains)
        .map(|c| (c * 255.0).round() as u8);
    assert!(bytes[..16 * 16 * 4].chunks(4).all(|p| p == [r, g, b, 255]));
}

#[test]
fn tints_blend_smoothly_between_neighbours() {
    let terrain = Terrain::new(11);
    let palette = BiomePalette::constants();
    let mut row = Vec::new();
    for x in -60..60 {
        let bytes = terrain.column_tint_bytes(ColumnPos::new(x, 4), &palette);
        row.extend((0..16).map(|dx| [bytes[dx * 4], bytes[dx * 4 + 1], bytes[dx * 4 + 2]]));
    }
    let step = row
        .windows(2)
        .map(|w| (0..3).map(|c| w[0][c].abs_diff(w[1][c])).max().unwrap())
        .max()
        .unwrap();
    assert!(
        step < 24,
        "grass color jumps by {step}/255 between adjacent blocks"
    );
}

fn debug_world() -> (Terrain, Vec<Column>) {
    let terrain = Terrain::of(7, WorldType::Debug);
    let span = (debug_grid_side(debug_states().len()) * 2 + SECTION_SIZE - 1) / SECTION_SIZE + 1;
    let columns = (0..span * span)
        .map(|i| terrain.generate(ColumnPos::new(i % span, i / span)))
        .collect();
    (terrain, columns)
}

#[test]
fn debug_world_holds_every_state_once_on_the_grid() {
    let (_, columns) = debug_world();
    let mut seen = Vec::new();
    let mut other = 0;
    for column in &columns {
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                for y in 0..WORLD_HEIGHT {
                    let p = IVec3::new(x, y, z);
                    match (y, column.get(p)) {
                        (_, Block::Air) => {}
                        (DEBUG_GRID_Y, block) => seen.push((block, column.get_data(p))),
                        (DEBUG_FLOOR_Y, Block::Barrier) => {}
                        _ => other += 1,
                    }
                }
            }
        }
    }
    assert_eq!(other, 0);
    seen.sort_by_key(|&(block, data)| (block as u8, data));
    let mut expected = debug_states();
    expected.sort_by_key(|&(block, data)| (block as u8, data));
    assert_eq!(seen, expected);
}

#[test]
fn debug_blocks_do_not_touch() {
    let (_, columns) = debug_world();
    let mut cells = Vec::new();
    for (i, column) in columns.iter().enumerate() {
        let span = (columns.len() as f64).sqrt() as i32;
        let origin = ColumnPos::new(i as i32 % span, i as i32 / span).origin();
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                if column.get(IVec3::new(x, DEBUG_GRID_Y, z)) != Block::Air {
                    cells.push((origin.x + x, origin.z + z));
                }
            }
        }
    }
    assert!(cells.iter().all(|&(x, z)| x % 2 == 0 && z % 2 == 0));
}

#[test]
fn debug_floor_covers_columns_outside_the_grid() {
    let terrain = Terrain::of(7, WorldType::Debug);
    let column = terrain.generate(ColumnPos::new(-9, 30));
    for (x, z) in [(0, 0), (15, 15)] {
        assert_eq!(column.get(IVec3::new(x, DEBUG_FLOOR_Y, z)), Block::Barrier);
        assert_eq!(column.get(IVec3::new(x, DEBUG_GRID_Y, z)), Block::Air);
    }
}

#[test]
fn debug_world_has_no_features_and_plains_biome() {
    let terrain = Terrain::of(7, WorldType::Debug);
    assert_eq!(WorldType::from_name("debug"), Some(WorldType::Debug));
    assert_eq!(terrain.biome_at(5, 5), Biome::Plains);
    let column = terrain.generate_trees(ColumnPos::new(3, 3));
    assert_eq!(column.get(IVec3::new(0, 1, 0)), Block::Air);
}

#[test]
fn cold_beach_is_snow_covered_sand() {
    const SPAN: i32 = 256;
    let terrain = Terrain::new(1337);
    let (mut snowy_sand, mut bare_sand) = (0, 0);
    'scan: for rz in -40..40 {
        for rx in -40..40 {
            let (x0, z0) = (rx * SPAN, rz * SPAN);
            let blocks = terrain.map.blocks(x0, z0, SPAN as usize, SPAN as usize);
            for (i, biome) in blocks.iter().enumerate() {
                if *biome != Biome::ColdBeach {
                    continue;
                }
                let (x, z) = (x0 + i as i32 % SPAN, z0 + i as i32 / SPAN);
                let surface = terrain.surface_height(x, z);
                let key = ColumnPos::of_block(IVec3::new(x, 0, z));
                let (lx, lz) = (x - key.origin().x, z - key.origin().z);
                let column = terrain.generate_ground(key);
                if column.get([lx, surface, lz].into()) != Block::Sand || surface < WATER_LEVEL {
                    continue;
                }
                match column.get([lx, surface + 1, lz].into()) {
                    Block::Snow => snowy_sand += 1,
                    _ => bare_sand += 1,
                }
                if snowy_sand + bare_sand >= 40 {
                    break 'scan;
                }
            }
        }
    }
    assert!(
        snowy_sand >= 10 && bare_sand == 0,
        "{snowy_sand} {bare_sand}"
    );
}

fn hill_columns(terrain: &Terrain) -> Vec<(ColumnPos, Column)> {
    columns_of(terrain, |b| b.is_extreme_hills(), 60)
        .into_iter()
        .map(|key| (key, terrain.generate_ground(key)))
        .collect()
}

#[test]
fn extreme_hills_have_overhangs_and_tall_peaks() {
    let terrain = Terrain::new(1337);
    let (mut overhangs, mut peak) = (0, 0);
    for (key, column) in hill_columns(&terrain) {
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                peak = peak.max(terrain.surface_height(key.origin().x + x, key.origin().z + z));
                let top = terrain.surface_height(key.origin().x + x, key.origin().z + z);
                overhangs += (WATER_LEVEL + 1..top)
                    .filter(|&y| {
                        column.get(IVec3::new(x, y, z)) == Block::Air
                            && column.get(IVec3::new(x, y + 1, z)).is_solid()
                    })
                    .count();
            }
        }
    }
    assert!(peak > 100, "{peak}");
    assert!(overhangs > 20, "{overhangs}");
}

#[test]
fn deep_oceans_are_deep_and_plains_are_gentle() {
    let terrain = Terrain::new(1337);
    let deepest = (-200..200)
        .flat_map(|z| (-200..200).map(move |x| (x * 16, z * 16)))
        .filter(|&(x, z)| terrain.sample_biome(x, z) == Biome::DeepOcean)
        .map(|(x, z)| terrain.surface_height(x, z))
        .min()
        .expect("no deep ocean");
    assert!(deepest < 50, "{deepest}");
    let steps: Vec<i32> = (-200..200)
        .flat_map(|z| (-200..200).map(move |x| (x * 16, z * 16)))
        .filter(|&(x, z)| terrain.sample_biome(x, z) == Biome::Plains)
        .take(400)
        .map(|(x, z)| (terrain.surface_height(x + 1, z) - terrain.surface_height(x, z)).abs())
        .collect();
    assert!(steps.len() > 100);
    assert!(steps.iter().sum::<i32>() < steps.len() as i32 * 2 / 3 + 1);
}

#[test]
fn warm_mountain_peaks_are_snow_capped_only_high_up() {
    let terrain = Terrain::new(1337);
    let (mut capped, mut low_snow) = (0, 0);
    for (_, column) in hill_columns(&terrain) {
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                for y in 1..WORLD_HEIGHT {
                    if column.get(IVec3::new(x, y, z)) == Block::Snow {
                        if y > 90 {
                            capped += 1;
                        } else if y < 75 {
                            low_snow += 1;
                        }
                    }
                }
            }
        }
    }
    assert!(capped > 0, "no snow on the peaks");
    assert_eq!(low_snow, 0);
}
