use super::*;
use crate::block::FARMLAND_WET;
use crate::column::Column;
use crate::terrain::Biome;

const GROUND: i32 = 10;
const CAP: usize = 400_000;

fn column(ground: Block) -> Column {
    let mut column = Column::new();
    for z in 0..SECTION_SIZE {
        for x in 0..SECTION_SIZE {
            for y in 0..GROUND {
                column.set(IVec3::new(x, y, z), Block::Stone);
            }
            column.set(IVec3::new(x, GROUND, z), ground);
        }
    }
    column
}

fn world_of(keys: &[ColumnPos], ground: Block) -> (World, LightMap) {
    let mut world = World::new();
    for &key in keys {
        world.insert(key, column(ground));
    }
    let mut light = LightMap::new();
    for &key in keys {
        light.add_column(&world, key);
    }
    (world, light)
}

fn at(x: i32, z: i32) -> IVec3 {
    IVec3::new(x, GROUND, z)
}

fn run(
    ticks: &mut RandomTicks,
    world: &mut World,
    light: &LightMap,
    steps: usize,
    until: impl Fn(&World) -> bool,
) -> usize {
    for step in 0..steps {
        ticks.step(world, light, &terrain(), ColumnPos::ZERO, 8, false);
        if until(world) {
            return step + 1;
        }
    }
    steps
}

fn count(world: &World, block: Block) -> usize {
    (0..SECTION_SIZE)
        .flat_map(|z| (0..SECTION_SIZE).map(move |x| at(x, z)))
        .filter(|&pos| world.block(pos) == block)
        .count()
}

#[test]
fn dirt_next_to_grass_in_light_turns_green() {
    let (mut world, light) = world_of(&[ColumnPos::ZERO], Block::Dirt);
    world.set_block(at(8, 8), Block::Grass);
    let mut ticks = RandomTicks::new(1);
    run(&mut ticks, &mut world, &light, CAP, |w| {
        w.block(at(9, 8)) == Block::Grass
    });
    assert_eq!(world.block(at(9, 8)), Block::Grass);
}

#[test]
fn dirt_in_the_dark_stays_dirt() {
    let (mut world, _) = world_of(&[ColumnPos::ZERO], Block::Dirt);
    world.set_block(at(8, 8), Block::Grass);
    for z in 0..SECTION_SIZE {
        for x in 0..SECTION_SIZE {
            world.set_block(IVec3::new(x, GROUND + 2, z), Block::Stone);
        }
    }
    let mut light = LightMap::new();
    light.add_column(&world, ColumnPos::ZERO);
    assert!(light.sky(at(9, 8) + IVec3::Y) < SPREAD_LIGHT);
    let mut ticks = RandomTicks::new(1);
    run(&mut ticks, &mut world, &light, CAP, |_| false);
    assert_eq!(count(&world, Block::Grass), 1);
}

#[test]
fn dirt_without_grass_stays_dirt() {
    let (mut world, light) = world_of(&[ColumnPos::ZERO], Block::Dirt);
    let mut ticks = RandomTicks::new(1);
    run(&mut ticks, &mut world, &light, CAP, |_| false);
    assert_eq!(count(&world, Block::Grass), 0);
}

#[test]
fn grass_under_stone_or_water_turns_to_dirt() {
    for cover in [Block::Stone, Block::Water] {
        let (mut world, light) = world_of(&[ColumnPos::ZERO], Block::Grass);
        world.set_block(at(8, 8) + IVec3::Y, cover);
        let mut ticks = RandomTicks::new(1);
        run(&mut ticks, &mut world, &light, CAP, |w| {
            w.block(at(8, 8)) == Block::Dirt
        });
        assert_eq!(world.block(at(8, 8)), Block::Dirt, "{cover:?}");
    }
}

#[test]
fn dirt_under_water_does_not_spread() {
    let (mut world, light) = world_of(&[ColumnPos::ZERO], Block::Dirt);
    world.set_block(at(8, 8), Block::Grass);
    world.set_block(at(9, 8) + IVec3::Y, Block::Water);
    let mut ticks = RandomTicks::new(1);
    run(&mut ticks, &mut world, &light, CAP, |_| false);
    assert_eq!(world.block(at(9, 8)), Block::Dirt);
}

#[test]
fn columns_outside_radius_are_not_ticked() {
    let far = ColumnPos::new(SIMULATION_RADIUS + 1, 0);
    let near = ColumnPos::new(SIMULATION_RADIUS, 0);
    let (mut world, light) = world_of(&[ColumnPos::ZERO, near, far], Block::Grass);
    for key in [ColumnPos::ZERO, near, far] {
        world.set_block(key.origin() + at(8, 8) + IVec3::Y, Block::Stone);
    }
    let mut ticks = RandomTicks::new(1);
    run(&mut ticks, &mut world, &light, 20_000, |_| false);
    assert_eq!(world.block(at(8, 8)), Block::Dirt);
    assert_eq!(world.block(near.origin() + at(8, 8)), Block::Dirt);
    assert_eq!(world.block(far.origin() + at(8, 8)), Block::Grass);
}

#[test]
fn view_radius_limits_simulation() {
    let key = ColumnPos::new(6, 0);
    let (mut world, light) = world_of(&[key], Block::Grass);
    world.set_block(key.origin() + at(8, 8) + IVec3::Y, Block::Stone);
    let mut ticks = RandomTicks::new(1);
    for _ in 0..20_000 {
        ticks.step(&mut world, &light, &terrain(), ColumnPos::ZERO, 4, false);
    }
    assert_eq!(world.block(key.origin() + at(8, 8)), Block::Grass);
}

#[test]
fn same_seed_gives_same_result() {
    let play = |seed| {
        let (mut world, light) = world_of(&[ColumnPos::ZERO], Block::Grass);
        for x in 0..SECTION_SIZE {
            world.set_block(at(x, 3) + IVec3::Y, Block::Stone);
        }
        let mut ticks = RandomTicks::new(seed);
        (0..200)
            .map(|_| {
                ticks
                    .step(&mut world, &light, &terrain(), ColumnPos::ZERO, 8, false)
                    .changed
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(play(7), play(7));
    assert_ne!(play(7), play(8));
}

#[test]
fn tick_rate_matches_three_cells_per_section_per_step() {
    let (mut world, light) = world_of(&[ColumnPos::ZERO], Block::Grass);
    for z in 0..SECTION_SIZE {
        for x in 0..SECTION_SIZE {
            world.set_block(at(x, z) + IVec3::Y, Block::Stone);
        }
    }
    let steps = 1000;
    let mut ticks = RandomTicks::new(3);
    run(&mut ticks, &mut world, &light, steps, |_| false);
    let cells = f64::from(SECTION_SIZE * SECTION_SIZE);
    let turned = count(&world, Block::Dirt) as f64 / cells;
    let per_step = RANDOM_TICKS_PER_SECTION as f64 / SECTION_VOLUME as f64;
    let expected = 1.0 - (1.0 - per_step).powi(steps as i32);
    assert!(
        (turned - expected).abs() < 0.1,
        "turned {turned}, expected {expected}"
    );
}

fn snowy_world(light_source: Option<IVec3>) -> (World, LightMap) {
    let (mut world, _) = world_of(&[ColumnPos::ZERO], Block::SnowyGrass);
    world.set_block(at(8, 8) + IVec3::Y, Block::Snow);
    if let Some(source) = light_source {
        world.set_block(source, Block::Glowstone);
    }
    let mut light = LightMap::new();
    light.add_column(&world, ColumnPos::ZERO);
    (world, light)
}

#[test]
fn snow_next_to_a_light_source_melts() {
    let (mut world, light) = snowy_world(Some(at(9, 8) + IVec3::Y));
    assert!(light.block(at(8, 8) + IVec3::Y) >= SNOW_MELT_LIGHT);
    let mut ticks = RandomTicks::new(1);
    run(&mut ticks, &mut world, &light, CAP, |w| {
        w.block(at(8, 8) + IVec3::Y) == Block::Air
    });
    assert_eq!(world.block(at(8, 8) + IVec3::Y), Block::Air);
    assert_eq!(world.block(at(8, 8)), Block::Grass);
}

#[test]
fn snow_far_from_light_stays() {
    let (mut world, light) = snowy_world(Some(at(8, 8) + IVec3::Y * 6));
    assert!(light.block(at(8, 8) + IVec3::Y) < SNOW_MELT_LIGHT);
    let mut ticks = RandomTicks::new(1);
    run(&mut ticks, &mut world, &light, 20_000, |_| false);
    assert_eq!(world.block(at(8, 8) + IVec3::Y), Block::Snow);
}

#[test]
fn snow_does_not_grow() {
    let (mut world, light) = snowy_world(None);
    let mut ticks = RandomTicks::new(1);
    run(&mut ticks, &mut world, &light, 20_000, |_| false);
    let snow = (0..SECTION_SIZE)
        .flat_map(|z| (0..SECTION_SIZE).map(move |x| at(x, z) + IVec3::Y))
        .filter(|&pos| world.block(pos) == Block::Snow)
        .count();
    assert_eq!(snow, 1);
}

#[test]
fn snowy_grass_under_stone_turns_to_dirt() {
    let (mut world, light) = world_of(&[ColumnPos::ZERO], Block::SnowyGrass);
    world.set_block(at(8, 8) + IVec3::Y, Block::Stone);
    let mut ticks = RandomTicks::new(1);
    run(&mut ticks, &mut world, &light, CAP, |w| {
        w.block(at(8, 8)) == Block::Dirt
    });
    assert_eq!(world.block(at(8, 8)), Block::Dirt);
}

#[test]
fn snowy_grass_under_snow_stays() {
    let (mut world, light) = snowy_world(None);
    let mut ticks = RandomTicks::new(1);
    run(&mut ticks, &mut world, &light, 20_000, |_| false);
    assert_eq!(world.block(at(8, 8)), Block::SnowyGrass);
}

#[test]
fn dirt_next_to_snowy_grass_becomes_grass() {
    let (mut world, light) = world_of(&[ColumnPos::ZERO], Block::Dirt);
    world.set_block(at(8, 8), Block::SnowyGrass);
    world.set_block(at(8, 8) + IVec3::Y, Block::Snow);
    let mut ticks = RandomTicks::new(1);
    run(&mut ticks, &mut world, &light, CAP, |w| {
        w.block(at(9, 8)) != Block::Dirt
    });
    assert_eq!(world.block(at(9, 8)), Block::Grass);
}

#[test]
fn cactus_grows_up_to_three_blocks() {
    let (mut world, light) = world_of(&[ColumnPos::ZERO], Block::Sand);
    let base = at(8, 8) + IVec3::Y;
    world.set_block(base, Block::Cactus);
    let mut ticks = RandomTicks::new(5);
    run(&mut ticks, &mut world, &light, CAP, |w| {
        w.block(base + 2 * IVec3::Y) == Block::Cactus
    });
    for d in 0..3 {
        assert_eq!(world.block(base + d * IVec3::Y), Block::Cactus, "{d}");
    }
    run(&mut ticks, &mut world, &light, 60_000, |_| false);
    assert_eq!(world.block(base + 3 * IVec3::Y), Block::Air);
}

#[test]
fn reeds_grow_to_three_blocks() {
    let (mut world, light) = world_of(&[ColumnPos::ZERO], Block::Sand);
    let base = at(8, 8) + IVec3::Y;
    world.set_block(base, Block::Reeds);
    let mut ticks = RandomTicks::new(5);
    run(&mut ticks, &mut world, &light, CAP, |w| {
        w.block(base + 2 * IVec3::Y) == Block::Reeds
    });
    assert_eq!(world.block(base + IVec3::Y), Block::Reeds);
    let top = base + 2 * IVec3::Y;
    assert_eq!(world.block(top), Block::Reeds);
    assert_eq!(world.block(base + 3 * IVec3::Y), Block::Air);
}

#[test]
fn cactus_does_not_grow_into_a_solid_side() {
    let (mut world, light) = world_of(&[ColumnPos::ZERO], Block::Sand);
    let base = at(8, 8) + IVec3::Y;
    world.set_block(base, Block::Cactus);
    world.set_block(base + IVec3::Y + IVec3::X, Block::Stone);
    let mut ticks = RandomTicks::new(5);
    run(&mut ticks, &mut world, &light, 60_000, |_| false);
    assert_eq!(world.block(base + IVec3::Y), Block::Air);
}

#[test]
fn vines_grow_down_while_attached_and_stop_at_the_ground() {
    use crate::block::VINE_POS_X;
    let (mut world, light) = world_of(&[ColumnPos::ZERO], Block::Grass);
    let top = at(4, 4) + IVec3::Y * 6;
    for y in 0..=6 {
        world.set_block(at(4, 4) + IVec3::X + IVec3::Y * y, Block::Stone);
    }
    world.set_block_with_data(top, Block::Vine, VINE_POS_X);
    let mut ticks = RandomTicks::new(5);
    run(&mut ticks, &mut world, &light, CAP, |w| {
        w.block(at(4, 4) + IVec3::Y) == Block::Vine
    });
    for y in 1..=6 {
        let pos = at(4, 4) + IVec3::Y * y;
        assert_eq!(world.block(pos), Block::Vine, "{pos}");
        assert_eq!(world.block_data(pos), VINE_POS_X);
    }
    assert_eq!(world.block(at(4, 4)), Block::Grass);
}

#[test]
fn a_ceiling_vine_does_not_grow_down() {
    use crate::block::VINE_TOP;
    let (mut world, light) = world_of(&[ColumnPos::ZERO], Block::Grass);
    let top = at(4, 4) + IVec3::Y * 4;
    world.set_block(top + IVec3::Y, Block::Stone);
    world.set_block_with_data(top, Block::Vine, VINE_TOP);
    let mut ticks = RandomTicks::new(5);
    run(&mut ticks, &mut world, &light, 60_000, |_| false);
    assert_eq!(world.block(top - IVec3::Y), Block::Air);
}

#[test]
fn mycelium_dies_under_a_block_and_spreads_to_lit_dirt() {
    let (mut world, light) = world_of(&[ColumnPos::ZERO], Block::Dirt);
    world.set_block(at(4, 4), Block::Mycelium);
    world.set_block(at(4, 4) + IVec3::Y, Block::Stone);
    let mut ticks = RandomTicks::new(5);
    run(&mut ticks, &mut world, &light, CAP, |w| {
        w.block(at(4, 4)) == Block::Dirt
    });
    assert_eq!(world.block(at(4, 4)), Block::Dirt);

    world.set_block(at(8, 8), Block::Mycelium);
    run(&mut ticks, &mut world, &light, CAP, |w| {
        w.block(at(9, 8)) == Block::Mycelium
    });
    assert_eq!(world.block(at(9, 8)), Block::Mycelium);
}

const SNOW_SEED: i32 = 7;
const WEATHER_STEPS: usize = 600;

fn terrain() -> Terrain {
    Terrain::new(SNOW_SEED)
}

fn uniform_key(terrain: &Terrain, wanted: impl Fn(Biome) -> bool) -> ColumnPos {
    (-300..300)
        .step_by(2)
        .flat_map(|z| (-300..300).step_by(2).map(move |x| ColumnPos::new(x, z)))
        .find(|key| {
            let origin = key.origin();
            wanted(terrain.sample_biome(origin.x + 8, origin.z + 8))
                && (0..SECTION_SIZE).all(|z| {
                    (0..SECTION_SIZE).all(|x| wanted(terrain.biome_at(origin.x + x, origin.z + z)))
                })
        })
        .expect("no uniform column")
}

fn cold_key(terrain: &Terrain) -> ColumnPos {
    uniform_key(terrain, |biome| biome == Biome::IcePlains)
}

fn warm_key(terrain: &Terrain) -> ColumnPos {
    uniform_key(terrain, |biome| biome == Biome::Plains)
}

fn weather_world(key: ColumnPos, ground: Block, torches: bool) -> (World, LightMap) {
    let mut world = World::new();
    let mut base = column(ground);
    if torches {
        for z in (1..SECTION_SIZE).step_by(3) {
            for x in (1..SECTION_SIZE).step_by(3) {
                base.set(IVec3::new(x, GROUND + 1, z), Block::Torch);
            }
        }
    }
    world.insert(key, base);
    let mut light = LightMap::new();
    light.add_column(&world, key);
    (world, light)
}

fn weather_run(key: ColumnPos, world: &mut World, light: &LightMap, precipitation: bool) {
    let terrain = terrain();
    let mut ticks = RandomTicks::new(5);
    for _ in 0..WEATHER_STEPS {
        ticks.step(world, light, &terrain, key, 1, precipitation);
    }
}

fn count_at(world: &World, key: ColumnPos, y: i32, block: Block) -> usize {
    (0..SECTION_SIZE)
        .flat_map(|z| (0..SECTION_SIZE).map(move |x| IVec3::new(x, y, z)))
        .filter(|&local| world.block(key.origin() + local) == block)
        .count()
}

#[test]
fn open_water_freezes_in_cold_biome_regardless_of_weather() {
    let key = cold_key(&terrain());
    let (mut world, light) = weather_world(key, Block::Water, false);
    weather_run(key, &mut world, &light, false);
    assert!(count_at(&world, key, GROUND, Block::Ice) > 20);
}

#[test]
fn water_does_not_freeze_in_warm_biome_or_near_light() {
    let terrain = terrain();
    let warm = warm_key(&terrain);
    let (mut world, light) = weather_world(warm, Block::Water, false);
    weather_run(warm, &mut world, &light, true);
    assert_eq!(count_at(&world, warm, GROUND, Block::Ice), 0);

    let cold = cold_key(&terrain);
    let (mut world, light) = weather_world(cold, Block::Water, true);
    weather_run(cold, &mut world, &light, true);
    assert_eq!(count_at(&world, cold, GROUND, Block::Ice), 0);
}

#[test]
fn snow_falls_only_during_precipitation_in_cold_biome() {
    let terrain = terrain();
    let cold = cold_key(&terrain);
    let (mut world, light) = weather_world(cold, Block::Grass, false);
    weather_run(cold, &mut world, &light, false);
    assert_eq!(count_at(&world, cold, GROUND + 1, Block::Snow), 0);
    weather_run(cold, &mut world, &light, true);
    let snow = count_at(&world, cold, GROUND + 1, Block::Snow);
    assert!(snow > 20);
    assert_eq!(count_at(&world, cold, GROUND, Block::SnowyGrass), snow);

    let warm = warm_key(&terrain);
    let (mut world, light) = weather_world(warm, Block::Grass, false);
    weather_run(warm, &mut world, &light, true);
    assert_eq!(count_at(&world, warm, GROUND + 1, Block::Snow), 0);
}

#[test]
fn snow_does_not_settle_on_ice() {
    let cold = cold_key(&terrain());
    let (mut world, light) = weather_world(cold, Block::Ice, false);
    weather_run(cold, &mut world, &light, true);
    assert_eq!(count_at(&world, cold, GROUND + 1, Block::Snow), 0);
    assert_eq!(count_at(&world, cold, GROUND, Block::Ice), 256);
}

#[test]
fn ice_melts_near_a_torch_only() {
    let cold = cold_key(&terrain());
    let (mut world, light) = weather_world(cold, Block::Ice, true);
    weather_run(cold, &mut world, &light, false);
    assert!(count_at(&world, cold, GROUND, Block::Water) > 20);
}

#[test]
fn snowy_grass_without_snow_turns_back_to_grass() {
    let (mut world, light) = world_of(&[ColumnPos::ZERO], Block::SnowyGrass);
    let mut ticks = RandomTicks::new(2);
    run(&mut ticks, &mut world, &light, CAP, |w| {
        count(w, Block::SnowyGrass) == 0
    });
    assert_eq!(count(&world, Block::SnowyGrass), 0);
    assert_eq!(count(&world, Block::Grass), 256);
}

#[test]
fn snow_settles_on_leaves_during_precipitation() {
    let cold = cold_key(&terrain());
    let (mut world, light) = weather_world(cold, Block::Leaves, false);
    weather_run(cold, &mut world, &light, true);
    assert!(count_at(&world, cold, GROUND + 1, Block::Snow) > 20);
}

const FARM_TICKS: usize = 4000;

fn farm() -> (World, LightMap, RandomTicks) {
    let (world, light) = world_of(&[ColumnPos::ZERO], Block::Farmland);
    (world, light, RandomTicks::new(3))
}

fn tick_at(ticks: &mut RandomTicks, world: &mut World, light: &LightMap, pos: IVec3, rain: bool) {
    ticks.tick(world, light, &terrain(), pos, rain, &mut Ticked::default());
}

fn plant(world: &mut World, pos: IVec3, crop: Block) {
    world.set_block_supported(pos + IVec3::Y, crop);
}

#[test]
fn farmland_near_water_gets_wet_and_far_one_dries_into_dirt() {
    let (mut world, light, mut ticks) = farm();
    world.set_block(at(8, 8), Block::Water);
    let (near, far) = (at(12, 8), at(0, 0));
    world.set_block_with_data(far, Block::Farmland, 2);
    tick_at(&mut ticks, &mut world, &light, near, false);
    assert_eq!(world.block_data(near), FARMLAND_WET);
    for expected in [1, 0] {
        tick_at(&mut ticks, &mut world, &light, far, false);
        assert_eq!(world.block_data(far), expected);
    }
    tick_at(&mut ticks, &mut world, &light, far, false);
    assert_eq!(world.block(far), Block::Dirt);
}

#[test]
fn water_one_block_above_still_wets_but_five_blocks_away_does_not() {
    let (mut world, light, mut ticks) = farm();
    world.set_block(at(8, 8) + IVec3::Y, Block::Water);
    tick_at(&mut ticks, &mut world, &light, at(4, 4), false);
    assert_eq!(world.block_data(at(4, 4)), FARMLAND_WET);
    world.set_block_with_data(at(3, 8), Block::Farmland, 1);
    tick_at(&mut ticks, &mut world, &light, at(3, 8), false);
    assert_eq!(world.block_data(at(3, 8)), 0);
}

#[test]
fn dry_farmland_with_a_crop_stays_farmland() {
    let (mut world, light, mut ticks) = farm();
    plant(&mut world, at(5, 5), Block::Wheat);
    for _ in 0..3 {
        tick_at(&mut ticks, &mut world, &light, at(5, 5), false);
    }
    assert_eq!(world.block(at(5, 5)), Block::Farmland);
    assert_eq!(world.block(at(5, 5) + IVec3::Y), Block::Wheat);
}

#[test]
fn farmland_turned_to_dirt_takes_its_crop_with_it() {
    let (mut world, light, mut ticks) = farm();
    plant(&mut world, at(5, 5), Block::Carrots);
    world.set_block(at(5, 5) + IVec3::Y, Block::Air);
    tick_at(&mut ticks, &mut world, &light, at(5, 5), false);
    assert_eq!(world.block(at(5, 5)), Block::Dirt);
}

fn lit(world: &World, key: ColumnPos) -> LightMap {
    let mut light = LightMap::new();
    light.add_column(world, key);
    light
}

#[test]
fn rain_wets_open_farmland_but_not_covered() {
    let key = warm_key(&terrain());
    let (mut world, _) = world_of(&[key], Block::Farmland);
    let (open, covered) = (
        key.origin() + IVec3::new(2, GROUND, 2),
        key.origin() + IVec3::new(8, GROUND, 8),
    );
    world.set_block(covered + IVec3::Y * 2, Block::Stone);
    let light = lit(&world, key);
    let mut ticks = RandomTicks::new(1);
    let mut ticked = Ticked::default();
    for pos in [open, covered] {
        ticks.tick(&mut world, &light, &terrain(), pos, true, &mut ticked);
    }
    assert_eq!(world.block_data(open), FARMLAND_WET);
    assert_eq!(world.block_data(covered), 0);
}

#[test]
fn every_crop_ripens_stage_by_stage_and_stops_at_its_last_stage() {
    for crop in [
        Block::Wheat,
        Block::Carrots,
        Block::Potatoes,
        Block::Beetroots,
    ] {
        let (mut world, light, _) = farm();
        world.set_block_with_data(at(5, 5), Block::Farmland, FARMLAND_WET);
        plant(&mut world, at(5, 5), crop);
        let mut ticks = RandomTicks::new(4);
        let mut seen = vec![0];
        for _ in 0..FARM_TICKS * 10 {
            tick_at(&mut ticks, &mut world, &light, at(5, 5) + IVec3::Y, false);
            let age = world.block_data(at(5, 5) + IVec3::Y);
            if age != *seen.last().unwrap() {
                seen.push(age);
            }
        }
        let expected: Vec<u8> = (0..=crop.crop_max_age()).collect();
        assert_eq!(seen, expected, "{crop:?}");
        assert_eq!(world.block(at(5, 5) + IVec3::Y), crop);
    }
}

#[test]
fn crops_grow_faster_on_wet_farmland_and_slower_next_to_a_twin() {
    let chance = |wet: u8, twin: bool| {
        let (mut world, _, _) = farm();
        world.set_block_with_data(at(5, 5), Block::Farmland, wet);
        plant(&mut world, at(5, 5), Block::Wheat);
        if twin {
            plant(&mut world, at(6, 6), Block::Wheat);
        }
        growth_chance(&world, at(5, 5) + IVec3::Y, Block::Wheat)
    };
    assert!(chance(FARMLAND_WET, false) > chance(0, false));
    assert_eq!(chance(0, true) * 2.0, chance(0, false));
    assert_eq!(chance(0, false), 1.0 + 1.0 + 8.0 * 0.25);
}

#[test]
fn crop_without_light_is_removed() {
    let (mut world, _, mut ticks) = farm();
    world.set_block_with_data(at(5, 5), Block::Farmland, FARMLAND_WET);
    plant(&mut world, at(5, 5), Block::Wheat);
    for z in 0..SECTION_SIZE {
        for x in 0..SECTION_SIZE {
            world.set_block(at(x, z) + IVec3::Y * 2, Block::Stone);
        }
    }
    let dark = lit(&world, ColumnPos::ZERO);
    tick_at(&mut ticks, &mut world, &dark, at(5, 5) + IVec3::Y, false);
    assert_eq!(world.block(at(5, 5) + IVec3::Y), Block::Air);
}

#[test]
fn beetroots_ripen_slower_than_wheat_in_the_same_light() {
    let steps = |crop| {
        let (mut world, light, _) = farm();
        world.set_block_with_data(at(5, 5), Block::Farmland, FARMLAND_WET);
        plant(&mut world, at(5, 5), crop);
        let mut total = 0;
        for seed in 0..40 {
            world.set_block_with_data(at(5, 5) + IVec3::Y, crop, 0);
            let mut ticks = RandomTicks::new(seed + 1);
            let mut ticked = Ticked::default();
            for step in 1..=FARM_TICKS * 10 {
                ticks.tick(
                    &mut world,
                    &light,
                    &terrain(),
                    at(5, 5) + IVec3::Y,
                    false,
                    &mut ticked,
                );
                if world.block_data(at(5, 5) + IVec3::Y) >= 1 {
                    total += step;
                    break;
                }
            }
        }
        total
    };
    assert!(steps(Block::Beetroots) > steps(Block::Wheat));
}
