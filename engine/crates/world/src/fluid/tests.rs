use super::*;
use crate::block::fluid_height_units;
use crate::column::Column;
use crate::pos::ColumnPos;

const FLOOR: i32 = 10;
const WATER_Y: i32 = FLOOR + 1;

fn flat_world(columns: &[(i32, i32)]) -> World {
    let mut world = World::new();
    for &(x, z) in columns {
        let mut column = Column::new();
        for lz in 0..16 {
            for lx in 0..16 {
                column.set(IVec3::new(lx, FLOOR, lz), Block::Stone);
            }
        }
        world.insert(ColumnPos::new(x, z), column);
    }
    world
}

fn at(x: i32, z: i32) -> IVec3 {
    IVec3::new(x, WATER_Y, z)
}

fn put(world: &mut World, fluid: &mut Fluid, pos: IVec3, block: Block, data: u8) {
    world.set_block_supported_with_data(pos, block, data);
    fluid.block_changed(world, pos);
}

fn settle(world: &mut World, fluid: &mut Fluid) -> Vec<IVec3> {
    let mut all = Vec::new();
    for _ in 0..200 {
        if fluid.is_empty() {
            return all;
        }
        all.extend(fluid.step(world));
    }
    panic!("fluid does not settle");
}

fn data_at(world: &World, pos: IVec3) -> Option<u8> {
    world.block(pos).is_water().then(|| world.block_data(pos))
}

fn water_cells(world: &World, y: i32) -> usize {
    (0..32)
        .flat_map(|z| (0..32).map(move |x| IVec3::new(x, y, z)))
        .filter(|&p| world.block(p).is_water())
        .count()
}

#[test]
fn spreads_seven_blocks_over_flat_floor() {
    let mut world = flat_world(&[(0, 0), (1, 0), (0, 1), (1, 1)]);
    let mut fluid = Fluid::new();
    put(
        &mut world,
        &mut fluid,
        at(16, 16),
        Block::Water,
        FLUID_SOURCE,
    );
    settle(&mut world, &mut fluid);
    for d in 0..=7 {
        assert_eq!(data_at(&world, at(16 + d, 16)), Some(d as u8), "+x {d}");
        assert_eq!(data_at(&world, at(16, 16 - d)), Some(d as u8), "-z {d}");
    }
    assert_eq!(data_at(&world, at(16 + 8, 16)), None);
    assert_eq!(data_at(&world, at(19, 18)), Some(5));
    assert_eq!(data_at(&world, at(20, 20)), None);
    assert_eq!(water_cells(&world, WATER_Y), 113);
}

#[test]
fn advances_one_cell_per_step() {
    let mut world = flat_world(&[(0, 0)]);
    let mut fluid = Fluid::new();
    put(&mut world, &mut fluid, at(8, 8), Block::Water, FLUID_SOURCE);
    fluid.step(&mut world);
    assert_eq!(data_at(&world, at(9, 8)), Some(1));
    assert_eq!(data_at(&world, at(10, 8)), None);
    fluid.step(&mut world);
    assert_eq!(data_at(&world, at(10, 8)), Some(2));
    assert_eq!(data_at(&world, at(11, 8)), None);
}

#[test]
fn falls_off_ledge_and_spreads_below() {
    let mut world = flat_world(&[(0, 0)]);
    let mut fluid = Fluid::new();
    for z in 0..16 {
        for x in 0..8 {
            world.set_block(IVec3::new(x, WATER_Y, z), Block::Stone);
        }
    }
    let top = WATER_Y + 1;
    put(
        &mut world,
        &mut fluid,
        IVec3::new(4, top, 8),
        Block::Water,
        FLUID_SOURCE,
    );
    settle(&mut world, &mut fluid);
    let edge = IVec3::new(8, top, 8);
    assert!(world.block(edge).is_water());
    assert_eq!(
        data_at(&world, IVec3::new(8, WATER_Y, 8)),
        Some(FLUID_FALLING)
    );
    assert_eq!(data_at(&world, IVec3::new(9, WATER_Y, 8)), Some(1));
    assert_eq!(data_at(&world, IVec3::new(10, WATER_Y, 8)), Some(2));
    assert_eq!(
        data_at(&world, IVec3::new(8, WATER_Y, 9)),
        Some(FLUID_FALLING)
    );
}

#[test]
fn source_over_air_falls_in_a_column() {
    let mut world = flat_world(&[(0, 0)]);
    let mut fluid = Fluid::new();
    put(
        &mut world,
        &mut fluid,
        IVec3::new(8, 20, 8),
        Block::Water,
        FLUID_SOURCE,
    );
    settle(&mut world, &mut fluid);
    for y in WATER_Y..20 {
        assert_eq!(
            data_at(&world, IVec3::new(8, y, 8)),
            Some(FLUID_FALLING),
            "y {y}"
        );
    }
    assert!(world.block(at(11, 8)).is_water());
}

#[test]
fn dries_up_after_source_removed() {
    let mut world = flat_world(&[(0, 0), (1, 0)]);
    let mut fluid = Fluid::new();
    put(
        &mut world,
        &mut fluid,
        at(10, 8),
        Block::Water,
        FLUID_SOURCE,
    );
    settle(&mut world, &mut fluid);
    assert!(water_cells(&world, WATER_Y) > 50);
    put(&mut world, &mut fluid, at(10, 8), Block::Air, 0);
    settle(&mut world, &mut fluid);
    assert_eq!(water_cells(&world, WATER_Y), 0);
}

#[test]
fn flows_toward_nearest_cliff_only() {
    let mut world = flat_world(&[(0, 0)]);
    let mut fluid = Fluid::new();
    world.set_block(IVec3::new(10, FLOOR, 8), Block::Air);
    world.set_block(IVec3::new(4, FLOOR, 8), Block::Air);
    put(&mut world, &mut fluid, at(8, 8), Block::Water, FLUID_SOURCE);
    fluid.step(&mut world);
    assert_eq!(data_at(&world, at(9, 8)), Some(1));
    assert_eq!(data_at(&world, at(7, 8)), None);
    assert_eq!(data_at(&world, at(8, 9)), None);
    assert_eq!(data_at(&world, at(8, 7)), None);
}

#[test]
fn flows_everywhere_without_cliffs_in_range() {
    let mut world = flat_world(&[(0, 0)]);
    let mut fluid = Fluid::new();
    world.set_block(IVec3::new(14, FLOOR, 8), Block::Air);
    put(&mut world, &mut fluid, at(8, 8), Block::Water, FLUID_SOURCE);
    fluid.step(&mut world);
    for p in [at(9, 8), at(7, 8), at(8, 9), at(8, 7)] {
        assert_eq!(data_at(&world, p), Some(1), "{p}");
    }
}

#[test]
fn two_sources_over_floor_make_a_new_source() {
    let mut world = flat_world(&[(0, 0)]);
    let mut fluid = Fluid::new();
    put(&mut world, &mut fluid, at(6, 8), Block::Water, FLUID_SOURCE);
    put(&mut world, &mut fluid, at(8, 8), Block::Water, FLUID_SOURCE);
    settle(&mut world, &mut fluid);
    assert_eq!(data_at(&world, at(7, 8)), Some(FLUID_SOURCE));
}

#[test]
fn no_infinite_source_without_support() {
    let mut world = flat_world(&[(0, 0)]);
    let mut fluid = Fluid::new();
    world.set_block(IVec3::new(7, FLOOR, 8), Block::Air);
    put(&mut world, &mut fluid, at(6, 8), Block::Water, FLUID_SOURCE);
    put(&mut world, &mut fluid, at(8, 8), Block::Water, FLUID_SOURCE);
    settle(&mut world, &mut fluid);
    assert_ne!(data_at(&world, at(7, 8)), Some(FLUID_SOURCE));
}

#[test]
fn water_washes_away_plants_and_torch() {
    let mut world = flat_world(&[(0, 0)]);
    let mut fluid = Fluid::new();
    world.set_block(at(9, 8), Block::Torch);
    world.set_block(at(8, 9), Block::Poppy);
    world.set_block(at(7, 8), Block::TallGrass);
    put(&mut world, &mut fluid, at(8, 8), Block::Water, FLUID_SOURCE);
    let changed = settle(&mut world, &mut fluid);
    for p in [at(9, 8), at(8, 9), at(7, 8)] {
        assert_eq!(data_at(&world, p), Some(1), "{p}");
        assert!(changed.contains(&p), "{p}");
    }
}

#[test]
fn stops_at_unloaded_column_edge() {
    let mut world = flat_world(&[(0, 0)]);
    let mut fluid = Fluid::new();
    put(
        &mut world,
        &mut fluid,
        at(14, 8),
        Block::Water,
        FLUID_SOURCE,
    );
    settle(&mut world, &mut fluid);
    assert_eq!(data_at(&world, at(15, 8)), Some(1));
    assert_eq!(world.block(at(16, 8)), Block::Air);
    assert!(fluid.is_empty());
    assert!(!world.is_loaded(at(16, 8)));
}

#[test]
fn queue_has_no_duplicates_and_skips_unloaded() {
    let world = flat_world(&[(0, 0)]);
    let mut fluid = Fluid::new();
    fluid.block_changed(&world, at(4, 4));
    fluid.block_changed(&world, at(4, 4));
    assert_eq!(fluid.len(), 7);
    let mut edge = Fluid::new();
    edge.block_changed(&world, at(15, 4));
    assert_eq!(edge.len(), 6);
}

#[test]
fn step_handles_at_most_the_limit() {
    let world = flat_world(&[(0, 0)]);
    let mut fluid = Fluid::new();
    for y in 20..60 {
        for z in 0..16 {
            fluid.block_changed(&world, IVec3::new(8, y, z));
        }
    }
    let queued = fluid.len();
    assert!(queued > STEP_LIMIT);
    let mut scratch = flat_world(&[(0, 0)]);
    fluid.step(&mut scratch);
    assert_eq!(fluid.len(), queued - STEP_LIMIT);
}

#[test]
fn changed_cells_are_queued_for_neighbors() {
    let mut world = flat_world(&[(0, 0)]);
    let mut fluid = Fluid::new();
    put(&mut world, &mut fluid, at(8, 8), Block::Water, FLUID_SOURCE);
    let changed = fluid.step(&mut world);
    assert_eq!(changed, [at(9, 8), at(7, 8), at(8, 9), at(8, 7)]);
    assert!(!fluid.is_empty());
}

#[test]
fn water_over_flowing_water_becomes_falling() {
    let mut world = flat_world(&[(0, 0)]);
    let mut fluid = Fluid::new();
    put(&mut world, &mut fluid, at(8, 8), Block::Water, FLUID_SOURCE);
    settle(&mut world, &mut fluid);
    let over = at(9, 8) + IVec3::Y;
    put(&mut world, &mut fluid, over, Block::Water, FLUID_SOURCE);
    settle(&mut world, &mut fluid);
    assert_eq!(data_at(&world, at(9, 8)), Some(FLUID_FALLING));
    assert_eq!(fluid_height_units(FLUID_FALLING), 16);
}
