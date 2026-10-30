use super::*;
use crate::column::Column;

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
        ticks.step(world, light, ColumnPos::ZERO, 8);
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
        ticks.step(&mut world, &light, ColumnPos::ZERO, 4);
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
            .map(|_| ticks.step(&mut world, &light, ColumnPos::ZERO, 8).changed)
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
