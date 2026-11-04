use super::*;
use crate::block::FluidKind;

const SEED: i32 = 1337;
const MESA_SEED: i32 = 99;
const ISLAND_SEED: i32 = 7;

fn columns_of(terrain: &Terrain, biome: Biome, count: usize) -> Vec<ColumnPos> {
    (-200..200)
        .flat_map(|z| (-200..200).map(move |x| ColumnPos::new(x, z)))
        .filter(|k| terrain.sample_biome(k.origin().x + 8, k.origin().z + 8) == biome)
        .take(count)
        .collect()
}

fn mesa_columns(terrain: &Terrain, biome: Biome, count: usize) -> Vec<ColumnPos> {
    let at = |k: ColumnPos| terrain.sample_biome(k.origin().x + 8, k.origin().z + 8);
    let mut found: Vec<ColumnPos> = Vec::new();
    for z in (-600..600).step_by(4) {
        for x in (-600..600).step_by(4) {
            let hit = ColumnPos::new(x, z);
            if found.len() >= count {
                return found;
            }
            if found.contains(&hit) || at(hit) != biome {
                continue;
            }
            for dz in -4..4 {
                for dx in -4..4 {
                    let k = ColumnPos::new(x + dx, z + dz);
                    if at(k) == biome && !found.contains(&k) {
                        found.push(k);
                    }
                }
            }
        }
    }
    assert!(!found.is_empty(), "no {biome:?} near the origin");
    found
}

fn cells() -> impl Iterator<Item = IVec3> {
    (0..WORLD_HEIGHT)
        .flat_map(|y| (0..16).flat_map(move |z| (0..16).map(move |x| [x, y, z].into())))
}

fn find(column: &Column, block: Block) -> Vec<IVec3> {
    cells().filter(|&p| column.get(p) == block).collect()
}

fn same_columns(a: &Column, b: &Column) -> bool {
    cells().all(|p| a.get(p) == b.get(p))
}

#[test]
fn decoration_is_deterministic() {
    let (a, b) = (Terrain::new(SEED), Terrain::new(SEED));
    for key in columns_of(&a, Biome::Desert, 4) {
        assert!(same_columns(&a.generate(key), &b.generate(key)));
    }
}

#[test]
fn desert_has_cacti_on_sand_with_free_sides() {
    let terrain = Terrain::new(SEED);
    let mut total = 0;
    for key in columns_of(&terrain, Biome::Desert, 10) {
        let column = terrain.generate(key);
        for pos in find(&column, Block::Cactus) {
            total += 1;
            let mut base = pos;
            while column.get(base - IVec3::Y) == Block::Cactus {
                base -= IVec3::Y;
            }
            assert!(column.get(base - IVec3::Y).is_sand(), "{key} {pos}");
            assert!(
                pos.y - base.y < 3,
                "stack of {} at {key} {pos}",
                pos.y - base.y + 1
            );
            for dir in HORIZONTAL {
                let side = pos + dir;
                assert!(
                    !Column::contains(side) || !column.get(side).is_solid(),
                    "{key} {pos}"
                );
            }
        }
    }
    assert!(total > 20, "{total} cacti");
}

#[test]
fn desert_has_dead_bushes_on_allowed_ground() {
    let terrain = Terrain::new(SEED);
    let mut total = 0;
    for key in columns_of(&terrain, Biome::Desert, 20) {
        let column = terrain.generate(key);
        for pos in find(&column, Block::DeadBush) {
            total += 1;
            assert!(
                Block::DeadBush.stands_on(column.get(pos - IVec3::Y)),
                "{key} {pos}"
            );
        }
    }
    assert!(total > 5, "{total} dead bushes");
}

#[test]
fn mesa_has_many_dead_bushes_and_few_cacti_on_red_sand() {
    let terrain = Terrain::new(MESA_SEED);
    let (mut bushes, mut cacti) = (0, 0);
    for key in mesa_columns(&terrain, Biome::Mesa, 12) {
        let column = terrain.generate(key);
        bushes += find(&column, Block::DeadBush).len();
        for pos in find(&column, Block::Cactus) {
            if !terrain
                .biome_at(key.origin().x + pos.x, key.origin().z + pos.z)
                .is_mesa()
            {
                continue;
            }
            cacti += 1;
            let mut base = pos;
            while column.get(base - IVec3::Y) == Block::Cactus {
                base -= IVec3::Y;
            }
            assert_eq!(column.get(base - IVec3::Y), Block::RedSand);
        }
    }
    assert!(bushes > 20, "{bushes} dead bushes");
    assert!(cacti > 0, "{cacti} cacti");
}

#[test]
fn grassland_has_no_cacti_or_dead_bushes() {
    let terrain = Terrain::new(SEED);
    for key in columns_of(&terrain, Biome::Plains, 8) {
        let column = terrain.generate(key);
        assert!(find(&column, Block::Cactus).is_empty());
        assert!(find(&column, Block::DeadBush).is_empty());
    }
}

#[test]
fn reeds_stand_at_the_waterline_two_to_four_high() {
    let terrain = Terrain::new(SEED);
    let mut total = 0;
    for z in -40..40 {
        for x in -40..40 {
            let key = ColumnPos::new(x, z);
            let column = terrain.generate(key);
            for pos in find(&column, Block::Reeds) {
                total += 1;
                let mut base = pos;
                while column.get(base - IVec3::Y) == Block::Reeds {
                    base -= IVec3::Y;
                }
                assert_eq!(base.y, WATER_LEVEL + 1, "{key} {pos}");
                let ground = base - IVec3::Y;
                assert!(Block::Reeds.stands_on(column.get(ground)), "{key} {pos}");
                assert!(pos.y - base.y < 4, "{key} {pos}");
                let wet = HORIZONTAL.iter().any(|&d| {
                    let side = ground + d;
                    if Column::contains(side) {
                        column.get(side) == Block::Water
                    } else {
                        true
                    }
                });
                assert!(wet, "{key} {pos}");
            }
        }
    }
    assert!(total > 0, "no reeds in 6400 columns");
}

#[test]
fn mesa_has_colored_bands_under_red_sand() {
    let terrain = Terrain::new(MESA_SEED);
    let mut kinds = std::collections::BTreeSet::new();
    for key in mesa_columns(&terrain, Biome::Mesa, 12) {
        let column = terrain.generate_ground(key);
        for p in cells().filter(|p| column.get(*p).is_terracotta()) {
            kinds.insert(column.get(p) as u8);
        }
        let o = key.origin();
        let surface = terrain.surface_height(o.x + 8, o.z + 8);
        if surface <= WATER_LEVEL {
            continue;
        }
        let top = column.get([8, surface, 8].into());
        assert!(
            top == Block::RedSand || top.is_terracotta() || top == Block::Grass,
            "{top:?}"
        );
    }
    assert!(kinds.len() >= 4, "{kinds:?}");
    assert!(
        kinds.contains(&(Block::Terracotta as u8))
            && kinds.contains(&(Block::OrangeTerracotta as u8))
    );
}

#[test]
fn bryce_pillars_rise_above_the_plain() {
    let terrain = Terrain::new(MESA_SEED);
    let keys = mesa_columns(&terrain, Biome::MesaBryce, 30);
    let tops: Vec<i32> = keys
        .iter()
        .flat_map(|k| {
            let o = k.origin();
            (0..16).flat_map(move |z| (0..16).map(move |x| (o.x + x, o.z + z)))
        })
        .map(|(x, z)| terrain.surface_height(x, z))
        .collect();
    assert!(
        tops.iter().max().copied().unwrap_or(0) > 80,
        "{:?}",
        tops.iter().max()
    );
}

#[test]
fn forest_plateau_has_oaks() {
    let terrain = Terrain::new(MESA_SEED);
    let logs: usize = mesa_columns(&terrain, Biome::MesaPlateauF, 30)
        .into_iter()
        .map(|key| find(&terrain.generate(key), Block::Log).len())
        .sum();
    assert!(logs > 0);
}

const JUNGLE_SEED: i32 = 4;

#[test]
fn jungle_wall_vines_hang_on_solid_blocks_or_wood() {
    let terrain = Terrain::new(JUNGLE_SEED);
    let mut total = 0;
    for key in columns_of(&terrain, Biome::Jungle, 40) {
        let column = terrain.generate(key);
        for pos in find(&column, Block::Vine) {
            total += 1;
            let side = column.get_data(pos);
            let held = crate::block::VINE_SIDES.iter().any(|&(bit, dir)| {
                side & bit != 0
                    && (!Column::contains(pos + dir) || column.get(pos + dir).holds_vine())
            }) || column.get(pos + IVec3::Y) == Block::Vine;
            assert!(held, "{key} {pos} {side}");
        }
    }
    assert!(total > 200, "{total} vines");
}

#[test]
fn jungle_wall_vines_grow_on_bare_stone_walls() {
    let terrain = Terrain::new(JUNGLE_SEED);
    let mut on_ground_walls = 0;
    for key in columns_of(&terrain, Biome::Jungle, 80) {
        let column = terrain.generate(key);
        for pos in find(&column, Block::Vine) {
            on_ground_walls += crate::block::VINE_SIDES[..4]
                .iter()
                .filter(|&&(bit, dir)| {
                    column.get_data(pos) & bit != 0
                        && Column::contains(pos + dir)
                        && matches!(
                            column.get(pos + dir),
                            Block::Stone | Block::Dirt | Block::Grass
                        )
                })
                .count();
        }
    }
    assert!(on_ground_walls > 0);
}

#[test]
fn melons_lie_on_grass_in_the_jungle_only() {
    let jungle = Terrain::new(JUNGLE_SEED);
    let mut total = 0;
    for key in columns_of(&jungle, Biome::Jungle, 120) {
        let column = jungle.generate(key);
        for pos in find(&column, Block::Melon) {
            total += 1;
            assert_eq!(column.get(pos - IVec3::Y), Block::Grass, "{key} {pos}");
        }
    }
    assert!(total > 0, "no melons in 120 jungle columns");

    let plains = Terrain::new(SEED);
    for key in columns_of(&plains, Biome::Forest, 20) {
        let column = plains.generate(key);
        assert!(find(&column, Block::Melon).is_empty());
        assert!(find(&column, Block::Vine).is_empty());
    }
}

fn swamp_columns(terrain: &Terrain) -> Vec<(ColumnPos, Column)> {
    columns_of(terrain, Biome::Swampland, 24)
        .into_iter()
        .map(|key| (key, terrain.generate(key)))
        .collect()
}

#[test]
fn lilies_float_only_on_water_and_swamps_have_them() {
    let terrain = Terrain::new(SEED);
    let mut total = 0;
    for (key, column) in swamp_columns(&terrain) {
        for pos in find(&column, Block::Waterlily) {
            total += 1;
            assert_eq!(column.get(pos - IVec3::Y), Block::Water, "{pos}");
            let world = key.origin() + pos;
            assert!(pos.y == WATER_LEVEL + 1 || terrain.lake_covers(world.x, world.z));
        }
    }
    assert!(total > 40, "only {total} lilies");
    for key in columns_of(&terrain, Biome::Plains, 12) {
        assert!(find(&terrain.generate(key), Block::Waterlily).is_empty());
    }
}

#[test]
fn swamp_puddles_replace_the_sea_level_top_with_water() {
    let terrain = Terrain::new(SEED);
    let mut puddles = 0;
    for (key, column) in swamp_columns(&terrain) {
        let origin = key.origin();
        for z in 0..16 {
            for x in 0..16 {
                let (wx, wz) = (origin.x + x, origin.z + z);
                let at = IVec3::new(x, WATER_LEVEL, z);
                let dry = terrain.surface_height(wx, wz) == WATER_LEVEL;
                if dry && terrain.is_puddle(wx, wz) && !terrain.lake_covers(wx, wz) {
                    puddles += 1;
                    assert_eq!(column.get(at), Block::Water);
                    assert!(matches!(
                        column.get(at - IVec3::Y),
                        Block::Dirt | Block::Clay | Block::Sand | Block::Gravel
                    ));
                } else if dry && !terrain.lake_covers(wx, wz) && column.get(at) != Block::Air {
                    assert!(
                        matches!(column.get(at), Block::Grass | Block::Sand | Block::Gravel),
                        "{wx}, {wz}"
                    );
                }
            }
        }
    }
    assert!(puddles > 20, "only {puddles} puddles");
}

#[test]
fn swamp_oaks_stand_in_shallow_water_and_hang_vines() {
    let terrain = Terrain::new(SEED);
    let mut oaks = 0;
    let mut vines = 0;
    for (key, column) in swamp_columns(&terrain) {
        vines += find(&column, Block::Vine).len();
        let origin = key.origin();
        for z in 0..16 {
            for x in 0..16 {
                let Some(tree) = trees::tree_at(&terrain, origin.x + x, origin.z + z) else {
                    continue;
                };
                if tree.kind != trees::TreeKind::SwampOak {
                    continue;
                }
                oaks += 1;
                let ground = tree.base - IVec3::Y;
                assert!(matches!(
                    terrain.surface_height(tree.base.x, tree.base.z) - ground.y,
                    0 | 1
                ));
                let below = column.get(IVec3::new(x, ground.y, z));
                assert!(matches!(below, Block::Grass | Block::Dirt), "{below:?}");
                assert_eq!(column.get(IVec3::new(x, tree.base.y, z)), Block::Log);
            }
        }
    }
    assert!(oaks > 3, "only {oaks} swamp oaks");
    assert!(vines > 50, "only {vines} vines");
}

#[test]
fn swamp_mushrooms_and_reeds_grow() {
    let terrain = Terrain::new(SEED);
    let columns = swamp_columns(&terrain);
    let mushrooms: usize = columns
        .iter()
        .map(|(_, c)| find(c, Block::BrownMushroom).len() + find(c, Block::RedMushroom).len())
        .sum();
    assert!(mushrooms > 8, "only {mushrooms} mushrooms");
    let reeds: usize = columns_of(&terrain, Biome::Swampland, 150)
        .into_iter()
        .map(|key| find(&terrain.generate(key), Block::Reeds).len())
        .sum();
    assert!(reeds > 0, "only {reeds} reeds");
}

#[test]
fn mushrooms_have_support_and_stand_in_the_dark_or_on_mycelium() {
    let terrain = Terrain::new(SEED);
    let mut seen = 0;
    for key in columns_of(&terrain, Biome::Forest, 200) {
        let column = terrain.generate(key);
        for block in [Block::BrownMushroom, Block::RedMushroom] {
            for pos in find(&column, block) {
                seen += 1;
                let below = column.get(pos - IVec3::Y);
                assert!(below.is_solid(), "{pos} on {below:?}");
                let origin = key.origin();
                let surface = terrain.surface_height(origin.x + pos.x, origin.z + pos.z);
                assert!(pos.y <= surface || shaded(&column, pos), "{pos}");
            }
        }
    }
    assert!(seen > 0);
}

#[test]
fn mushroom_island_is_mycelium_with_giant_and_small_mushrooms() {
    let terrain = Terrain::new(ISLAND_SEED);
    let (mut giant, mut small) = (0, 0);
    for key in columns_of(&terrain, Biome::MushroomIsland, 40) {
        let column = terrain.generate(key);
        let origin = key.origin();
        for z in 0..16 {
            for x in 0..16 {
                let top = terrain.surface_height(origin.x + x, origin.z + z);
                if top > WATER_LEVEL
                    && terrain
                        .biome_at(origin.x + x, origin.z + z)
                        .is_mushroom_island()
                {
                    assert!(matches!(
                        column.get(IVec3::new(x, top, z)),
                        Block::Mycelium | Block::Air
                    ));
                }
            }
        }
        giant += find(&column, Block::MushroomStem).len();
        for block in [Block::BrownMushroom, Block::RedMushroom] {
            small += find(&column, block).len();
        }
        assert!(find(&column, Block::Log).is_empty());
        assert!(find(&column, Block::TallGrass).is_empty());
    }
    assert!(giant > 0, "no giant mushrooms");
    assert!(small > 0, "no small mushrooms");
}

#[test]
fn clay_lies_on_dirt_floors_under_water() {
    let terrain = Terrain::new(SEED);
    let mut clay = 0;
    for key in columns_of(&terrain, Biome::Swampland, 60)
        .into_iter()
        .chain(columns_of(&terrain, Biome::Forest, 60))
    {
        let column = terrain.generate(key);
        for pos in find(&column, Block::Clay) {
            clay += 1;
            let origin = key.origin();
            assert!(terrain.surface_height(origin.x + pos.x, origin.z + pos.z) < WATER_LEVEL + 4);
        }
    }
    assert!(clay > 0, "no clay");
}

#[test]
fn swamp_and_island_decoration_is_deterministic() {
    let (a, b) = (Terrain::new(SEED), Terrain::new(SEED));
    for key in columns_of(&a, Biome::Swampland, 4) {
        assert!(same_columns(&a.generate(key), &b.generate(key)));
    }
    let (a, b) = (Terrain::new(ISLAND_SEED), Terrain::new(ISLAND_SEED));
    for key in columns_of(&a, Biome::MushroomIsland, 4) {
        assert!(same_columns(&a.generate(key), &b.generate(key)));
    }
}

fn surface_of(column: &Column, x: i32, z: i32) -> i32 {
    (0..WORLD_HEIGHT)
        .rev()
        .find(|&y| column.get([x, y, z].into()).is_solid())
        .unwrap_or(0)
}

#[test]
fn ice_spikes_grow_packed_ice_spires_and_paths() {
    let terrain = Terrain::new(SEED);
    let (mut spires, mut paths, mut tallest) = (0, 0, 0);
    for key in mesa_columns(&terrain, Biome::IceSpikes, 24) {
        let column = terrain.generate(key);
        for pos in find(&column, Block::IcePacked) {
            let ground = terrain.surface_height(key.origin().x + pos.x, key.origin().z + pos.z);
            if pos.y > ground + 1 {
                spires += 1;
                tallest = tallest.max(pos.y - ground);
            } else if pos.y >= ground - 1 {
                paths += 1;
            }
        }
    }
    assert!(spires > 100, "{spires} spire blocks");
    assert!(paths > 20, "{paths} path blocks");
    assert!(tallest >= 7, "tallest spire {tallest}");
}

#[test]
fn ice_spikes_are_deterministic_and_stay_in_their_column() {
    let (a, b) = (Terrain::new(SEED), Terrain::new(SEED));
    for key in mesa_columns(&a, Biome::IceSpikes, 6) {
        let first = a.generate(key);
        assert!(same_columns(&first, &b.generate(key)));
        let ground = a.generate_trees(key);
        let packed = find(&first, Block::IcePacked)
            .into_iter()
            .filter(|&p| ground.get(p) != Block::IcePacked)
            .count();
        assert!(packed < 16 * 16 * WORLD_HEIGHT as usize);
    }
}

#[test]
fn spikes_only_stand_in_the_ice_spikes_biome() {
    let terrain = Terrain::new(SEED);
    for biome in [Biome::IcePlains, Biome::ColdTaiga] {
        for key in columns_of(&terrain, biome, 6) {
            assert!(
                find(&terrain.generate(key), Block::IcePacked).is_empty(),
                "{biome:?} {key}"
            );
        }
    }
}

#[test]
fn mega_taiga_has_podzol_coarse_dirt_and_mossy_boulders() {
    let terrain = Terrain::new(SEED);
    let (mut podzol, mut coarse, mut grass, mut boulders) = (0, 0, 0, 0);
    for key in mesa_columns(&terrain, Biome::MegaTaiga, 20) {
        let column = terrain.generate(key);
        for z in 0..16 {
            for x in 0..16 {
                match column.get([x, surface_of(&column, x, z), z].into()) {
                    Block::Podzol => podzol += 1,
                    Block::CoarseDirt => coarse += 1,
                    Block::Grass => grass += 1,
                    _ => {}
                }
            }
        }
        boulders += find(&column, Block::MossyCobblestone).len();
    }
    assert!(
        podzol > coarse && coarse > 0 && grass > 0,
        "{podzol} {coarse} {grass}"
    );
    assert!(boulders > 10, "{boulders} boulder blocks");
}

#[test]
fn boulders_are_small_mossy_blobs_sunk_into_the_ground() {
    let terrain = Terrain::new(SEED);
    for key in mesa_columns(&terrain, Biome::MegaTaiga, 12) {
        let column = terrain.generate(key);
        for pos in find(&column, Block::MossyCobblestone) {
            let (x, z) = (key.origin().x + pos.x, key.origin().z + pos.z);
            let ground = (-3..=3)
                .flat_map(|dz| (-3..=3).map(move |dx| (dx, dz)))
                .map(|(dx, dz)| terrain.surface_height(x + dx, z + dz))
                .max()
                .unwrap();
            assert!(pos.y <= ground + 3, "{key} {pos}");
        }
    }
    assert!(Block::MossyCobblestone.is_solid());
}

fn double_pairs(column: &Column) -> Vec<(IVec3, Block)> {
    cells()
        .filter_map(|p| {
            let block = column.get(p);
            block.double_upper().map(|upper| {
                assert_eq!(column.get(p + IVec3::Y), upper, "{p}");
                (p, block)
            })
        })
        .collect()
}

fn assert_doubles_whole(column: &Column) {
    for p in cells() {
        let block = column.get(p);
        if let Some(lower) = block.double_lower() {
            assert_eq!(column.get(p - IVec3::Y), lower, "{p}");
        }
    }
    for (p, _) in double_pairs(column) {
        assert!(
            matches!(
                column.get(p - IVec3::Y),
                Block::Grass | Block::Dirt | Block::Podzol | Block::CoarseDirt
            ),
            "{p}"
        );
    }
}

#[test]
fn double_plants_are_always_whole_pairs_on_soil() {
    let terrain = Terrain::new(SEED);
    let mut total = 0;
    for biome in [
        Biome::Plains,
        Biome::SunflowerPlains,
        Biome::FlowerForest,
        Biome::Forest,
        Biome::Taiga,
        Biome::MegaTaiga,
        Biome::Savanna,
    ] {
        for key in mesa_columns(&terrain, biome, 12) {
            let column = terrain.generate(key);
            assert_doubles_whole(&column);
            total += double_pairs(&column).len();
        }
    }
    assert!(total > 100, "{total} double plants");
}

fn count_of(terrain: &Terrain, biome: Biome, columns: usize, blocks: &[Block]) -> usize {
    mesa_columns(terrain, biome, columns)
        .into_iter()
        .map(|key| {
            let column = terrain.generate(key);
            cells().filter(|&p| blocks.contains(&column.get(p))).count()
        })
        .sum()
}

#[test]
fn sunflowers_grow_in_sunflower_plains_only() {
    let terrain = Terrain::new(SEED);
    assert!(
        count_of(
            &terrain,
            Biome::SunflowerPlains,
            12,
            &[Block::SunflowerLower]
        ) > 10
    );
    assert_eq!(
        count_of(&terrain, Biome::Plains, 12, &[Block::SunflowerLower]),
        0
    );
}

#[test]
fn tall_grass_doubles_grow_on_plains_and_ferns_in_taiga() {
    let terrain = Terrain::new(SEED);
    assert!(count_of(&terrain, Biome::Plains, 12, &[Block::DoubleGrassLower]) > 5);
    assert!(count_of(&terrain, Biome::Taiga, 12, &[Block::DoubleFernLower]) > 5);
    assert!(count_of(&terrain, Biome::MegaTaiga, 12, &[Block::DoubleFernLower]) > 5);
    assert_eq!(
        count_of(&terrain, Biome::Taiga, 12, &[Block::DoubleGrassLower]),
        0
    );
}

#[test]
fn flower_forest_has_many_tall_flowers_and_ordinary_forest_few() {
    let terrain = Terrain::new(SEED);
    let kinds = [Block::LilacLower, Block::RoseBushLower, Block::PeonyLower];
    let flower_forest = count_of(&terrain, Biome::FlowerForest, 16, &kinds);
    let forest = count_of(&terrain, Biome::Forest, 16, &kinds);
    assert!(flower_forest > 20, "{flower_forest}");
    assert!(flower_forest > 3 * forest, "{flower_forest} vs {forest}");
}

#[test]
fn flower_forest_is_a_patchwork_of_many_flower_kinds() {
    let terrain = Terrain::new(SEED);
    let kinds = [
        Block::Dandelion,
        Block::Poppy,
        Block::Allium,
        Block::Houstonia,
        Block::RedTulip,
        Block::OrangeTulip,
        Block::WhiteTulip,
        Block::PinkTulip,
        Block::OxeyeDaisy,
    ];
    let present = kinds
        .into_iter()
        .filter(|&k| count_of(&terrain, Biome::FlowerForest, 40, &[k]) > 0)
        .count();
    assert!(present >= 4, "{present} flower kinds");
    assert_eq!(
        count_of(&terrain, Biome::FlowerForest, 40, &[Block::BlueOrchid]),
        0
    );
}

#[test]
fn swamps_grow_blue_orchids_and_plains_grow_the_new_flowers() {
    let terrain = Terrain::new(SEED);
    assert!(count_of(&terrain, Biome::Swampland, 60, &[Block::BlueOrchid]) > 0);
    let plains = [Block::Houstonia, Block::OxeyeDaisy];
    assert!(count_of(&terrain, Biome::Plains, 24, &plains) > 0);
    assert_eq!(count_of(&terrain, Biome::Plains, 24, &[Block::Allium]), 0);
}

#[test]
fn ferns_replace_some_grass_in_taiga_and_jungle() {
    let terrain = Terrain::new(SEED);
    for biome in [Biome::Taiga, Biome::Jungle, Biome::MegaTaiga] {
        let ferns = count_of(&terrain, biome, 12, &[Block::Fern]);
        let grass = count_of(&terrain, biome, 12, &[Block::TallGrass]);
        assert!(ferns > 0, "{biome:?}");
        assert!(
            biome != Biome::Jungle || grass > ferns,
            "{biome:?} {grass} {ferns}"
        );
    }
    assert_eq!(count_of(&terrain, Biome::Plains, 12, &[Block::Fern]), 0);
}

#[test]
fn shallow_seabeds_are_dirt_with_sand_gravel_and_clay_disks() {
    let terrain = Terrain::new(SEED);
    let (mut dirt, mut sand, mut gravel, mut clay) = (0, 0, 0, 0);
    let keys = columns_of(&terrain, Biome::Ocean, 60)
        .into_iter()
        .chain(columns_of(&terrain, Biome::River, 60));
    for key in keys {
        let column = terrain.generate(key);
        let ground = terrain.generate_ground(key);
        for z in 0..16 {
            for x in 0..16 {
                let floor = surface_of(&column, x, z);
                if !(WATER_LEVEL - 3..WATER_LEVEL).contains(&floor)
                    || surface_of(&ground, x, z) != floor
                {
                    continue;
                }
                match column.get([x, floor, z].into()) {
                    Block::Dirt => dirt += 1,
                    Block::Sand => sand += 1,
                    Block::Gravel => gravel += 1,
                    Block::Clay => clay += 1,
                    other if other.is_stone() => {}
                    other => panic!("{other:?} on a shallow seabed"),
                }
            }
        }
    }
    assert!(dirt > sand / 2 && dirt > gravel, "{dirt} {sand} {gravel}");
    assert!(sand > 0 && gravel > 0 && clay > 0, "{sand} {gravel} {clay}");
}

fn open_water_lake_cell(column: &Column, at: IVec3) -> bool {
    at.y > WATER_LEVEL && column.get(at) == Block::Water && column.get_data(at) == 0
}

fn lake_cells(column: &Column) -> Vec<IVec3> {
    cells()
        .filter(|&p| open_water_lake_cell(column, p))
        .filter(|&p| {
            [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z]
                .into_iter()
                .filter(|&d| Column::contains(p + d) && column.get(p + d) == Block::Water)
                .count()
                >= 2
        })
        .collect()
}

#[test]
fn lakes_and_springs_are_deterministic() {
    let (a, b) = (Terrain::new(SEED), Terrain::new(SEED));
    for z in -3..3 {
        for x in -3..3 {
            let key = ColumnPos::new(x, z);
            assert!(same_columns(&a.generate(key), &b.generate(key)));
        }
    }
}

#[test]
fn water_and_lava_lakes_appear_but_not_in_deserts() {
    let terrain = Terrain::new(SEED);
    let (mut water, mut lava) = (0, 0);
    for z in -14..14 {
        for x in -14..14 {
            let column = terrain.generate(ColumnPos::new(x, z));
            water += lake_cells(&column).len();
            lava += cells()
                .filter(|&p| p.y >= 11 && column.get(p) == Block::Lava && column.get_data(p) == 0)
                .count();
        }
    }
    assert!(water > 50, "water lake cells: {water}");
    assert!(lava > 5, "lava cells: {lava}");

    let corner_dry = |key: ColumnPos| {
        [(0, 0), (1, 0), (0, 1), (1, 1)]
            .into_iter()
            .all(|(dx, dz)| {
                let at = ColumnPos::new(key.x() + dx, key.z() + dz).origin();
                matches!(
                    terrain.sample_biome(at.x, at.z),
                    Biome::Desert | Biome::DesertHills
                )
            })
    };
    let mut checked = 0;
    for key in columns_of(&terrain, Biome::Desert, 400)
        .into_iter()
        .filter(|&k| corner_dry(k))
    {
        checked += 1;
        assert!(lake_cells(&terrain.generate(key)).is_empty(), "{key:?}");
    }
    assert!(checked > 0);
}

#[test]
fn lakes_are_whole_across_column_borders() {
    let terrain = Terrain::new(SEED);
    let mut straddling = 0;
    let mut checked = 0;
    for sz in -40..40 {
        for sx in -40..40 {
            for lake in terrain.lakes_of(sx, sz).iter() {
                if lake.kind != FluidKind::Water
                    || lake.y0 + 4 <= WATER_LEVEL + 1
                    || straddling >= 12
                {
                    continue;
                }
                straddling += 1;
                let columns: Vec<(ColumnPos, Column)> = [(0, 0), (1, 0), (0, 1), (1, 1)]
                    .into_iter()
                    .map(|(dx, dz)| {
                        let key = ColumnPos::new(sx + dx, sz + dz);
                        (key, terrain.generate(key))
                    })
                    .collect();
                for cell in lake.liquid_cells() {
                    let key = ColumnPos::of_block(cell);
                    let (_, column) = columns.iter().find(|(k, _)| *k == key).unwrap();
                    let found = column.get(cell - key.origin());
                    assert!(
                        matches!(found, Block::Water | Block::Ice),
                        "{cell} {found:?}"
                    );
                    checked += 1;
                }
            }
        }
    }
    assert!(
        straddling >= 5 && checked > 500,
        "{straddling} lakes, {checked} cells"
    );
}

fn stone_box() -> Column {
    let mut column = Column::new();
    for y in 0..80 {
        for z in 0..16 {
            for x in 0..16 {
                column.set(IVec3::new(x, y, z), Block::Stone);
            }
        }
    }
    column
}

#[test]
fn spring_needs_three_stone_sides_and_one_open() {
    let terrain = Terrain::new(SEED);
    let key = ColumnPos::new(0, 0);
    let at = IVec3::new(8, 40, 8);
    let mut column = stone_box();
    assert!(!terrain.try_spring(key, &mut column, at, FluidKind::Water));
    column.set(at + IVec3::Z, Block::Air);
    assert!(terrain.try_spring(key, &mut column, at, FluidKind::Water));
    assert_eq!(column.get(at), Block::Water);
    column.set(at, Block::Stone);
    column.set(at - IVec3::Z, Block::Air);
    assert!(!terrain.try_spring(key, &mut column, at, FluidKind::Lava));
    column.set(at - IVec3::Z, Block::Stone);
    column.set(at + IVec3::Y, Block::Air);
    assert!(!terrain.try_spring(key, &mut column, at, FluidKind::Lava));
}

#[test]
fn spring_flows_down_the_open_shaft_once_the_world_wakes_it() {
    use crate::fluid::Fluid;
    use crate::world::World;
    let terrain = Terrain::new(SEED);
    let key = ColumnPos::new(0, 0);
    let at = IVec3::new(8, 50, 8);
    let mut column = stone_box();
    for y in 30..=50 {
        column.set(IVec3::new(8, y, 9), Block::Air);
    }
    assert!(terrain.try_spring(key, &mut column, at, FluidKind::Water));
    column.springs.push(at);
    let mut world = World::new();
    world.insert(key, column);
    let wakes = world.drain_fluid_wakes();
    assert_eq!(wakes, vec![at]);
    assert!(world.drain_fluid_wakes().is_empty());
    let mut water = Fluid::new();
    for pos in wakes {
        water.block_changed(&world, pos);
    }
    for _ in 0..64 {
        water.step(&mut world);
    }
    assert_eq!(world.block(at), Block::Water);
    for y in 31..=49 {
        assert_eq!(world.block(IVec3::new(8, y, 9)), Block::Water, "{y}");
    }
}

#[test]
fn generated_springs_sit_in_stone_with_one_open_side() {
    let terrain = Terrain::new(SEED);
    let (mut water, mut lava) = (0, 0);
    for z in -12..12 {
        for x in -12..12 {
            let column = terrain.generate(ColumnPos::new(x, z));
            for &p in &column.springs {
                let block = column.get(p);
                assert!(matches!(block, Block::Water | Block::Lava));
                assert_eq!(column.get_data(p), 0);
                assert!(column.get(p + IVec3::Y).is_stone());
                assert!(column.get(p - IVec3::Y).is_stone());
                let sides = [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z]
                    .into_iter()
                    .filter(|&d| Column::contains(p + d))
                    .map(|d| column.get(p + d));
                let (inner, stone, air) = sides.fold((0, 0, 0), |(n, s, a), b| {
                    (
                        n + 1,
                        s + usize::from(b.is_stone()),
                        a + usize::from(b == Block::Air),
                    )
                });
                assert!(stone <= 3 && (inner < 4 || (stone == 3 && air == 1)), "{p}");
                if block == Block::Water {
                    water += 1;
                } else {
                    lava += 1;
                }
            }
        }
    }
    assert!(water > 5 && lava > 1, "water {water}, lava {lava}");
}

#[test]
fn pumpkin_patches_stand_on_grass_facing_a_side() {
    let terrain = Terrain::new(SEED);
    let sources: Vec<(i32, i32)> = (-300..300)
        .flat_map(|z| (-300..300).map(move |x| (x, z)))
        .filter(|&(x, z)| terrain.pumpkin_patch(x, z).is_some())
        .take(80)
        .collect();
    let mut found = 0;
    for (sx, sz) in sources {
        for (dx, dz) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let column = terrain.generate(ColumnPos::new(sx + dx, sz + dz));
            for p in find(&column, Block::Pumpkin) {
                found += 1;
                assert_eq!(column.get(p - IVec3::Y), Block::Grass);
                assert!(column.get_data(p) < 4);
            }
        }
    }
    assert!(found > 0);
}

fn highest(column: &Column, block: Block) -> Option<i32> {
    find(column, block).iter().map(|p| p.y).max()
}

fn ore_blocks() -> [Block; 7] {
    [
        Block::CoalOre,
        Block::IronOre,
        Block::GoldOre,
        Block::RedstoneOre,
        Block::DiamondOre,
        Block::LapisOre,
        Block::EmeraldOre,
    ]
}

#[test]
fn ores_are_deterministic() {
    let (a, b) = (Terrain::new(SEED), Terrain::new(SEED));
    for key in columns_of(&a, Biome::Plains, 4) {
        assert!(same_columns(&a.generate(key), &b.generate(key)));
    }
}

#[test]
fn ores_stay_under_their_height_limits() {
    let terrain = Terrain::new(SEED);
    let limits = [
        (Block::DiamondOre, 20),
        (Block::RedstoneOre, 20),
        (Block::GoldOre, 36),
        (Block::IronOre, 68),
        (Block::LapisOre, 36),
        (Block::CoalOre, 132),
    ];
    let mut seen = [0usize; 6];
    for key in columns_of(&terrain, Biome::Plains, 60) {
        let column = terrain.generate(key);
        for (i, (block, limit)) in limits.into_iter().enumerate() {
            let cells = find(&column, block);
            seen[i] += cells.len();
            assert!(cells.iter().all(|p| p.y < limit), "{block:?} {key}");
        }
    }
    assert!(seen.iter().all(|&n| n > 0), "{seen:?}");
}

#[test]
fn emeralds_only_grow_in_extreme_hills() {
    let terrain = Terrain::new(SEED);
    for key in columns_of(&terrain, Biome::Plains, 30) {
        assert!(find(&terrain.generate(key), Block::EmeraldOre).is_empty());
    }
    let mut total = 0;
    for key in columns_of(&terrain, Biome::ExtremeHills, 20) {
        for p in find(&terrain.generate(key), Block::EmeraldOre) {
            total += 1;
            assert!((4..32).contains(&p.y), "{key} {p}");
        }
    }
    assert!(total > 0);
}

#[test]
fn ores_only_replace_stone() {
    let terrain = Terrain::new(SEED);
    for key in columns_of(&terrain, Biome::Plains, 10) {
        let mut before = terrain.generate_carved(key);
        terrain.grow_lakes(key, &mut before);
        let mut after = before.clone();
        terrain.grow_ores(key, &mut after);
        let mut changed = 0;
        for p in cells() {
            if before.get(p) != after.get(p) {
                changed += 1;
                assert!(before.get(p).is_stone(), "{key} {p} {:?}", before.get(p));
            }
        }
        assert!(changed > 0);
    }
}

#[test]
fn veins_continue_across_column_borders() {
    let terrain = Terrain::new(SEED);
    let mut joined = 0;
    for z in 0..6 {
        for x in 0..6 {
            let (a, b) = (
                terrain.generate(ColumnPos::new(x, z)),
                terrain.generate(ColumnPos::new(x + 1, z)),
            );
            for y in 0..WORLD_HEIGHT {
                for k in 0..16 {
                    let (left, right) = (a.get([15, y, k].into()), b.get([0, y, k].into()));
                    joined += usize::from(left == right && ore_blocks().contains(&left));
                }
            }
        }
    }
    assert!(joined > 0);
}

#[test]
fn mesa_has_extra_gold_high_up() {
    let terrain = Terrain::new(MESA_SEED);
    let high = mesa_columns(&terrain, Biome::Mesa, 40)
        .into_iter()
        .filter_map(|key| highest(&terrain.generate(key), Block::GoldOre))
        .max();
    assert!(high.is_some_and(|y| y >= 40), "{high:?}");
}

#[test]
fn generated_cocoa_hangs_on_jungle_logs() {
    let terrain = Terrain::new(SEED);
    let (mut stages, mut checked) = ([0; 3], 0);
    for key in mesa_columns(&terrain, Biome::Jungle, 60) {
        let column = terrain.generate(key);
        for at in find(&column, Block::Cocoa) {
            let data = column.get_data(at);
            stages[usize::from(data >> crate::block::COCOA_STAGE_SHIFT)] += 1;
            let behind = at + Block::Cocoa.support_dir(data).unwrap();
            if Column::contains(behind) {
                assert_eq!(column.get(behind), Block::JungleLog, "{key} {at}");
                checked += 1;
            }
        }
    }
    assert!(checked > 0, "{stages:?}");
}

#[test]
fn ice_spikes_ground_is_snow_block_over_dirt() {
    let terrain = Terrain::new(SEED);
    let mut checked = 0;
    for key in mesa_columns(&terrain, Biome::IceSpikes, 6) {
        let ground = terrain.generate_ground(key);
        for (x, z) in [(2, 3), (9, 9), (14, 5)] {
            let (wx, wz) = (key.origin().x + x, key.origin().z + z);
            let surface = terrain.surface_height(wx, wz);
            if surface < WATER_LEVEL || terrain.biome_at(wx, wz) != Biome::IceSpikes {
                continue;
            }
            checked += 1;
            assert_eq!(
                ground.get(IVec3::new(x, surface, z)),
                Block::SnowBlock,
                "{key}"
            );
            assert_eq!(
                ground.get(IVec3::new(x, surface - 1, z)),
                Block::Dirt,
                "{key}"
            );
        }
    }
    assert!(checked > 10, "{checked}");
}

#[test]
fn monster_eggs_hide_in_extreme_hills_stone_below_the_sky() {
    let terrain = Terrain::new(SEED);
    for key in columns_of(&terrain, Biome::Plains, 30) {
        assert!(find(&terrain.generate(key), Block::MonsterEgg).is_empty());
    }
    let mut total = 0;
    for biome in [Biome::ExtremeHills, Biome::ExtremeHillsPlus] {
        for key in columns_of(&terrain, biome, 20) {
            for p in find(&terrain.generate(key), Block::MonsterEgg) {
                total += 1;
                assert!(p.y <= 64, "{key} {p}");
            }
        }
    }
    assert!(total > 200, "{total} egg blocks");
}

fn fossil_sources(terrain: &Terrain) -> Vec<(i32, i32, fossils::FossilBlocks)> {
    (-150..150)
        .flat_map(|z| (-150..150).map(move |x| (x, z)))
        .filter_map(|(x, z)| {
            let cells = terrain.fossil(x, z);
            (!cells.is_empty()).then_some((x, z, cells))
        })
        .collect()
}

#[test]
fn fossils_appear_only_in_deserts_and_swamps_deep_underground() {
    let terrain = Terrain::new(SEED);
    let fossils = fossil_sources(&terrain);
    assert!(fossils.len() > 3, "{} fossils", fossils.len());
    let mut desert = 0;
    for (sx, sz, cells) in &fossils {
        let biome = terrain.sample_biome(sx * 16 + 8, sz * 16 + 8);
        desert += usize::from(biome.is_desert());
        assert!(biome.is_desert() || biome.is_swamp(), "{biome:?}");
        let top = cells.iter().map(|(at, ..)| at.y).max().unwrap();
        let lowest_ground = cells
            .iter()
            .map(|(at, ..)| terrain.surface_height(at.x, at.z))
            .min()
            .unwrap();
        assert!(
            top + 3 <= lowest_ground,
            "top {top}, ground {lowest_ground}"
        );
        assert!(cells.iter().all(|(at, ..)| at.y >= 10 - 1));
        assert!(
            cells
                .iter()
                .all(|(_, block, _)| matches!(block, Block::BoneBlock | Block::CoalOre))
        );
    }
    assert!(desert > 0, "no desert fossil");
}

#[test]
fn fossils_are_mostly_bone_with_some_coal_and_gaps() {
    let terrain = Terrain::new(SEED);
    let (mut bone, mut coal) = (0, 0);
    for (_, _, cells) in fossil_sources(&terrain) {
        for (_, block, _) in cells {
            bone += usize::from(block == Block::BoneBlock);
            coal += usize::from(block == Block::CoalOre);
        }
    }
    assert!(coal > 0 && bone > coal * 4, "{bone} bone, {coal} coal");
}

#[test]
fn fossils_cross_column_borders_without_seams() {
    let terrain = Terrain::new(SEED);
    let (mut crossing, mut matched, mut total) = (0, 0, 0);
    for (.., cells) in fossil_sources(&terrain).into_iter().take(12) {
        let owners: std::collections::HashSet<(i32, i32)> = cells
            .iter()
            .map(|(at, ..)| (at.x >> 4, at.z >> 4))
            .collect();
        crossing += usize::from(owners.len() > 1);
        for (cx, cz) in owners {
            let key = ColumnPos::new(cx, cz);
            let column = terrain.generate(key);
            for (at, block, _) in cells
                .iter()
                .filter(|(at, ..)| (at.x >> 4, at.z >> 4) == (cx, cz))
            {
                let local = *at - IVec3::new(key.origin().x, 0, key.origin().z);
                total += 1;
                matched += usize::from(column.get(local) == *block);
            }
        }
    }
    assert!(crossing > 0, "no fossil crosses a border");
    assert!(matched * 100 >= total * 98, "{matched} of {total}");
}
