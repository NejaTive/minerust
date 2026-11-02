use super::*;

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
fn surface_stays_in_range() {
    let terrain = Terrain::new(1337);
    let (mut min, mut max) = (i32::MAX, i32::MIN);
    for z in (-2048..2048).step_by(8) {
        for x in (-2048..2048).step_by(8) {
            let h = terrain.surface_height(x, z);
            assert!((1..WORLD_HEIGHT - 1).contains(&h), "{h} at {x}, {z}");
            min = min.min(h);
            max = max.max(h);
        }
    }
    assert!(max - min > 30, "{min}..{max}");
    assert!(min > 20 && max < 110, "{min}..{max}");
}

fn find_biome(terrain: &Terrain, biome: Biome) -> (i32, i32) {
    find_column(terrain, |x, z, w| {
        w.dominant() == biome && terrain.surface(x, z, w) > *BEACH.end()
    })
    .unwrap_or_else(|| panic!("no {biome:?}"))
}

fn find_column(
    terrain: &Terrain,
    pred: impl Fn(i32, i32, BiomeWeights) -> bool,
) -> Option<(i32, i32)> {
    (0..8192)
        .map(|i| (i * 8, i * 3))
        .find(|&(x, z)| pred(x, z, terrain.biome_weights(x, z)))
}

#[test]
fn low_ground_is_under_water() {
    let terrain = Terrain::new(1337);
    let (x, z) = find_column(&terrain, |x, z, w| {
        w.dominant() != Biome::Desert && terrain.surface(x, z, w) < *BEACH.start()
    })
    .expect("no deep water");
    let (top, layers) = ground_layers(&terrain, x, z);
    assert!(top < WATER_LEVEL);
    assert_eq!(layers[0], Block::Gravel, "deep water floor is gravel");
    assert!(layers.iter().all(|&b| b != Block::Water));
}

#[test]
fn water_fills_column_up_to_level() {
    let terrain = Terrain::new(1);
    let (x, z) = find_column(&terrain, |x, z, w| {
        terrain.surface(x, z, w) == 55 && w.dominant() != Biome::Taiga
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
    for surface in [WATER_LEVEL - 1, WATER_LEVEL + 1] {
        let (x, z) = find_column(&terrain, |x, z, w| {
            w.dominant() != Biome::Desert && terrain.surface(x, z, w) == surface
        })
        .unwrap_or_else(|| panic!("no beach at {surface}"));
        let (_, layers) = ground_layers(&terrain, x, z);
        let sand = layers.iter().take_while(|&&b| b == Block::Sand).count();
        assert_eq!(sand, BEACH_SAND_DEPTH as usize, "surface {surface}");
        assert_eq!(layers[sand], Block::Dirt);
    }
}

fn ground_layers(terrain: &Terrain, x: i32, z: i32) -> (i32, Vec<Block>) {
    let key = ColumnPos::new(x.div_euclid(16), z.div_euclid(16));
    let column = terrain.generate_ground(key);
    let local = |y| IVec3::new(x.rem_euclid(16), y, z.rem_euclid(16));
    let top = terrain.surface_height(x, z);
    for y in top + 1..WORLD_HEIGHT {
        let expected = if y <= WATER_LEVEL {
            Block::Water
        } else {
            Block::Air
        };
        assert_eq!(column.get(local(y)), expected, "y {y}");
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
        assert!(
            layers[1..=DIRT_DEPTH as usize]
                .iter()
                .all(|&b| b == Block::Dirt)
        );
        assert!(
            layers[DIRT_DEPTH as usize + 1..]
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
    assert!((3..=4).contains(&sand), "{sand} sand blocks");
    assert!(
        layers[sand..]
            .iter()
            .all(|&b| matches!(b, Block::Stone | Block::Bedrock))
    );
}

#[test]
fn biomes_are_deterministic() {
    let (a, b) = (Terrain::new(42), Terrain::new(42));
    for (x, z) in [(0, 0), (-1000, 37), (123_456, -98_765)] {
        assert_eq!(a.biome_weights(x, z), b.biome_weights(x, z));
    }
}

#[test]
fn weights_sum_to_one() {
    let terrain = Terrain::new(7);
    for i in 0..1000 {
        let w = terrain.biome_weights(i * 13, i * -7);
        assert!(
            [w.plains, w.desert, w.forest, w.taiga]
                .iter()
                .all(|&v| (0.0..=1.0).contains(&v))
        );
        assert!(
            (w.plains + w.desert + w.forest + w.taiga - 1.0).abs() < 1e-5,
            "{w:?}"
        );
    }
}

#[test]
fn all_biomes_within_2048_blocks() {
    for seed in [1337, 42, 7] {
        let terrain = Terrain::new(seed);
        let mut counts = [0; 4];
        for z in (0..2048).step_by(16) {
            for x in (0..2048).step_by(16) {
                counts[terrain.biome_at(x, z) as usize] += 1;
            }
        }
        let total: i32 = counts.iter().sum();
        assert!(
            counts.iter().all(|&c| c * 20 > total),
            "seed {seed}: {counts:?}"
        );
    }
}

#[test]
fn biome_borders_have_no_cliffs() {
    let terrain = Terrain::new(1337);
    let pure = |w: BiomeWeights| [w.plains, w.desert, w.forest, w.taiga].contains(&1.0);
    let (mut border, mut forest, mut border_pairs) = (0, 0, 0);
    for row in (-2048..2048).step_by(32) {
        for along_x in [true, false] {
            let mut prev: Option<(i32, BiomeWeights)> = None;
            for i in -2048..2048 {
                let (x, z) = if along_x { (i, row) } else { (row, i) };
                let w = terrain.biome_weights(x, z);
                let h = terrain.surface_height(x, z);
                if let Some((ph, pw)) = prev {
                    let step = (h - ph).abs();
                    if !pure(w) || !pure(pw) {
                        border = border.max(step);
                        border_pairs += 1;
                    } else if w.forest == 1.0 && pw.forest == 1.0 {
                        forest = forest.max(step);
                    }
                }
                prev = Some((h, w));
            }
        }
    }
    assert!(border_pairs > 1000, "{border_pairs} border pairs");
    assert!(
        border <= forest,
        "border step {border}, forest step {forest}"
    );
}

#[test]
fn deep_water_floor_is_gravel() {
    let terrain = Terrain::new(42);
    let key = lake_key(&terrain);
    let column = terrain.generate(key);
    let origin = key.origin();
    let mut deep = 0;
    for z in 0..SECTION_SIZE {
        for x in 0..SECTION_SIZE {
            let surface = terrain.surface_height(origin.x + x, origin.z + z);
            let top = column.get(IVec3::new(x, surface, z));
            if WATER_LEVEL - surface > 2 {
                deep += 1;
                assert_eq!(top, Block::Gravel, "({x}, {z})");
            } else {
                assert_ne!(top, Block::Gravel, "({x}, {z})");
            }
        }
    }
    assert!(deep > 0);
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
    for (y, expected) in [(1, 0.75), (2, 0.5), (3, 0.25)] {
        assert!((share(y) - expected).abs() < 0.07, "y={y}: {}", share(y));
    }
    assert_eq!(share(4), 0.0);
}

#[test]
fn caves_and_lava_leave_bedrock_alone() {
    let terrain = Terrain::new(7);
    let ground = terrain.generate_ground(ColumnPos::new(2, 2));
    let carved = terrain.generate(ColumnPos::new(2, 2));
    for y in 0..=3 {
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

fn biome_share(terrain: &Terrain, biome: Biome) -> f32 {
    let mut hits = 0;
    let mut total = 0;
    for z in (-8000..8000).step_by(40) {
        for x in (-8000..8000).step_by(40) {
            total += 1;
            hits += i32::from(terrain.biome_at(x, z) == biome);
        }
    }
    hits as f32 / total as f32
}

#[test]
fn taiga_is_about_as_common_as_desert() {
    for seed in [1337, 42, 7] {
        let terrain = Terrain::new(seed);
        let (taiga, desert) = (
            biome_share(&terrain, Biome::Taiga),
            biome_share(&terrain, Biome::Desert),
        );
        assert!((0.12..0.25).contains(&taiga), "seed {seed}: taiga {taiga}");
        assert!(
            (taiga - desert).abs() < 0.04,
            "seed {seed}: taiga {taiga}, desert {desert}"
        );
    }
}

#[test]
fn taiga_never_touches_desert() {
    for seed in [1337, 42] {
        let terrain = Terrain::new(seed);
        let mut taiga = 0;
        for z in (-4000..4000).step_by(53) {
            let mut previous = terrain.biome_at(-4000, z);
            for x in -3999..4000 {
                let biome = terrain.biome_at(x, z);
                taiga += i32::from(biome == Biome::Taiga);
                assert!(
                    !matches!(
                        (previous, biome),
                        (Biome::Taiga, Biome::Desert) | (Biome::Desert, Biome::Taiga)
                    ),
                    "seed {seed}: {x}, {z}"
                );
                previous = biome;
            }
        }
        assert!(taiga > 10_000, "seed {seed}: {taiga} taiga cells");
    }
}

#[test]
fn flat_world_has_no_taiga() {
    let terrain = Terrain::of(1337, WorldType::Flat);
    for i in 0..500 {
        let weights = terrain.biome_weights(i * 97, i * -61);
        assert_eq!(weights.taiga, 0.0);
        assert_eq!(weights.dominant(), Biome::Plains);
    }
}

fn taiga_columns(terrain: &Terrain) -> Vec<ColumnPos> {
    (-48..48)
        .flat_map(|z| (-48..48).map(move |x| ColumnPos::new(x, z)))
        .filter(|key| {
            let origin = key.origin();
            (0..SECTION_SIZE)
                .step_by(5)
                .flat_map(|d| [(d, d), (d, SECTION_SIZE - 1 - d)])
                .all(|(dx, dz)| terrain.biome_at(origin.x + dx, origin.z + dz) == Biome::Taiga)
        })
        .collect()
}

#[test]
fn taiga_surface_is_snowy_grass_under_a_snow_layer() {
    let terrain = Terrain::new(1337);
    let mut checked = 0;
    for key in taiga_columns(&terrain).into_iter().take(40) {
        let column = terrain.generate_ground(key);
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                let (wx, wz) = (key.origin().x + x, key.origin().z + z);
                let surface = terrain.surface_height(wx, wz);
                let at = |y| column.get(IVec3::new(x, y, z));
                if surface < WATER_LEVEL || terrain.biome_at(wx, wz) != Biome::Taiga {
                    continue;
                }
                if matches!(at(surface), Block::Sand | Block::Bedrock) {
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
    assert!(checked > 2000, "{checked} cells");
}

#[test]
fn other_biomes_stay_green_and_snowless() {
    let terrain = Terrain::new(1337);
    for biome in [Biome::Plains, Biome::Forest, Biome::Desert] {
        let (x, z) = find_biome(&terrain, biome);
        let column = terrain.generate_ground(ColumnPos::new(x.div_euclid(16), z.div_euclid(16)));
        for y in 0..WORLD_HEIGHT {
            for lz in 0..SECTION_SIZE {
                for lx in 0..SECTION_SIZE {
                    let block = column.get(IVec3::new(lx, y, lz));
                    assert!(block != Block::Ice && block != Block::Snow, "{biome:?}");
                }
            }
        }
    }
}

#[test]
fn taiga_water_is_capped_with_ice() {
    let terrain = Terrain::new(1337);
    let mut lakes = 0;
    let mut deep = 0;
    for z in -48..48 {
        for x in -48..48 {
            let key = ColumnPos::new(x, z);
            let column = terrain.generate_ground(key);
            for lz in 0..SECTION_SIZE {
                for lx in 0..SECTION_SIZE {
                    let (wx, wz) = (key.origin().x + lx, key.origin().z + lz);
                    let surface = terrain.surface_height(wx, wz);
                    if surface >= WATER_LEVEL || terrain.biome_at(wx, wz) != Biome::Taiga {
                        continue;
                    }
                    let at = |y| column.get(IVec3::new(lx, y, lz));
                    assert_eq!(at(WATER_LEVEL), Block::Ice, "{wx}, {wz}");
                    assert_eq!(at(WATER_LEVEL + 1), Block::Air, "{wx}, {wz}");
                    assert!(
                        (surface + 1..WATER_LEVEL).all(|y| at(y) == Block::Water),
                        "{wx}, {wz}"
                    );
                    lakes += 1;
                    deep += i32::from(surface + 1 < WATER_LEVEL);
                }
            }
        }
    }
    assert!(lakes > 200, "{lakes} frozen cells");
    assert!(deep > 100, "{deep} cells with water under the ice");
}

#[test]
fn snow_layer_follows_the_ground_through_caves() {
    let terrain = Terrain::new(1337);
    for key in taiga_columns(&terrain).into_iter().take(40) {
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
