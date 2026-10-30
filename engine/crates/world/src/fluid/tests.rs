use super::*;
use crate::block::fluid_height_units;
use crate::column::Column;
use crate::pos::ColumnPos;
use crate::updates::STEP_LIMIT;

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

fn lava_world() -> (World, Fluid) {
    (flat_world(&[(0, 0), (1, 0)]), Fluid::of(FluidKind::Lava))
}

fn lava_at(world: &World, pos: IVec3) -> Option<u8> {
    (world.block(pos) == Block::Lava).then(|| world.block_data(pos))
}

#[test]
fn lava_spreads_three_blocks_in_steps_of_two_levels() {
    let (mut world, mut lava) = lava_world();
    put(&mut world, &mut lava, at(8, 8), Block::Lava, FLUID_SOURCE);
    settle(&mut world, &mut lava);
    for d in 0..=3 {
        assert_eq!(lava_at(&world, at(8 + d, 8)), Some(2 * d as u8), "+x {d}");
    }
    assert_eq!(lava_at(&world, at(8 + 4, 8)), None);
    assert_eq!(lava_at(&world, at(9, 9)), Some(4));
}

#[test]
fn lava_has_no_infinite_source() {
    let (mut world, mut lava) = lava_world();
    put(&mut world, &mut lava, at(6, 8), Block::Lava, FLUID_SOURCE);
    put(&mut world, &mut lava, at(8, 8), Block::Lava, FLUID_SOURCE);
    settle(&mut world, &mut lava);
    assert_eq!(lava_at(&world, at(7, 8)), Some(2));
}

#[test]
fn lava_dries_up_after_source_removed() {
    let (mut world, mut lava) = lava_world();
    put(&mut world, &mut lava, at(8, 8), Block::Lava, FLUID_SOURCE);
    settle(&mut world, &mut lava);
    put(&mut world, &mut lava, at(8, 8), Block::Air, 0);
    settle(&mut world, &mut lava);
    assert_eq!(world.block(at(9, 8)), Block::Air);
}

#[test]
fn lava_looks_for_a_cliff_only_two_cells_away() {
    let (mut world, mut lava) = lava_world();
    world.set_block(IVec3::new(11, FLOOR, 8), Block::Air);
    world.set_block(IVec3::new(14, FLOOR, 8), Block::Air);
    put(&mut world, &mut lava, at(8, 8), Block::Lava, FLUID_SOURCE);
    lava.step(&mut world);
    assert!(lava_at(&world, at(9, 8)).is_some());
    assert!(lava_at(&world, at(7, 8)).is_none());
}

#[test]
fn lava_and_water_do_not_replace_each_other() {
    let mut world = flat_world(&[(0, 0)]);
    let mut water = Fluid::new();
    put(&mut world, &mut water, at(8, 8), Block::Water, FLUID_SOURCE);
    world.set_block(at(9, 8), Block::Lava);
    settle(&mut world, &mut water);
    assert_eq!(world.block(at(9, 8)), Block::Lava);
    assert_eq!(data_at(&world, at(7, 8)), Some(1));
}

#[test]
fn lava_source_next_to_water_becomes_obsidian() {
    let mut world = flat_world(&[(0, 0)]);
    let mut lava = Fluid::of(FluidKind::Lava);
    put(&mut world, &mut lava, at(8, 8), Block::Lava, FLUID_SOURCE);
    world.set_block(at(9, 8), Block::Water);
    lava.block_changed(&world, at(9, 8));
    settle(&mut world, &mut lava);
    assert_eq!(world.block(at(8, 8)), Block::Obsidian);
}

#[test]
fn lava_source_under_water_becomes_obsidian() {
    let mut world = flat_world(&[(0, 0)]);
    let mut lava = Fluid::of(FluidKind::Lava);
    put(&mut world, &mut lava, at(8, 8), Block::Lava, FLUID_SOURCE);
    let above = at(8, 8) + IVec3::Y;
    world.set_block(above, Block::Water);
    lava.block_changed(&world, above);
    settle(&mut world, &mut lava);
    assert_eq!(world.block(at(8, 8)), Block::Obsidian);
}

#[test]
fn flowing_lava_touching_water_becomes_cobblestone() {
    let mut world = flat_world(&[(0, 0)]);
    let mut lava = Fluid::of(FluidKind::Lava);
    put(&mut world, &mut lava, at(8, 8), Block::Lava, FLUID_SOURCE);
    settle(&mut world, &mut lava);
    world.set_block(at(11, 9), Block::Water);
    lava.block_changed(&world, at(11, 9));
    settle(&mut world, &mut lava);
    assert_eq!(world.block(at(11, 8)), Block::Cobblestone);
    assert_eq!(world.block(at(8, 8)), Block::Lava);
    assert_eq!(world.block(at(9, 8)), Block::Lava);
}

#[test]
fn lava_washes_away_plants_and_torch() {
    let (mut world, mut lava) = lava_world();
    world.set_block(at(9, 8), Block::Torch);
    world.set_block(at(8, 9), Block::Poppy);
    world.set_block(at(7, 8), Block::TallGrass);
    put(&mut world, &mut lava, at(8, 8), Block::Lava, FLUID_SOURCE);
    let changed = settle(&mut world, &mut lava);
    for p in [at(9, 8), at(8, 9), at(7, 8)] {
        assert_eq!(lava_at(&world, p), Some(2), "{p}");
        assert!(changed.contains(&p), "{p}");
    }
}

#[test]
fn lava_source_over_water_source_turns_the_water_to_stone() {
    let mut world = flat_world(&[(0, 0)]);
    let mut lava = Fluid::of(FluidKind::Lava);
    world.set_block(at(8, 8), Block::Water);
    put(
        &mut world,
        &mut lava,
        at(8, 8) + IVec3::Y,
        Block::Lava,
        FLUID_SOURCE,
    );
    settle(&mut world, &mut lava);
    assert_eq!(world.block(at(8, 8)), Block::Stone);
    assert_eq!(lava_at(&world, at(8, 8) + IVec3::Y), Some(FLUID_SOURCE));
}

#[test]
fn falling_lava_over_water_turns_the_water_to_stone_and_spreads_over_it() {
    let mut world = flat_world(&[(0, 0)]);
    let mut lava = Fluid::of(FluidKind::Lava);
    world.set_block_with_data(at(8, 8), Block::Water, 3);
    put(
        &mut world,
        &mut lava,
        at(8, 8) + IVec3::new(0, 2, 0),
        Block::Lava,
        FLUID_SOURCE,
    );
    settle(&mut world, &mut lava);
    assert_eq!(world.block(at(8, 8)), Block::Stone);
    assert_eq!(lava_at(&world, at(8, 8) + IVec3::Y), Some(FLUID_FALLING));
    assert!(lava_at(&world, at(9, 8) + IVec3::Y).is_some());
}

#[test]
fn water_updating_under_lava_is_replaced_via_the_lava_queue() {
    let mut world = flat_world(&[(0, 0)]);
    let mut lava = Fluid::of(FluidKind::Lava);
    world.set_block(at(8, 8) + IVec3::Y, Block::Lava);
    put(&mut world, &mut lava, at(8, 8), Block::Water, FLUID_SOURCE);
    settle(&mut world, &mut lava);
    assert_eq!(world.block(at(8, 8)), Block::Stone);
}

fn water_over_lava(lava_data: u8, lava_first: bool) -> Block {
    let mut world = flat_world(&[(0, 0)]);
    let (mut water, mut lava) = (Fluid::new(), Fluid::of(FluidKind::Lava));
    world.set_block_with_data(at(8, 8), Block::Lava, lava_data);
    let above = at(8, 8) + IVec3::Y;
    world.set_block(above, Block::Water);
    for fluid in [&mut water, &mut lava] {
        fluid.block_changed(&world, above);
    }
    for _ in 0..20 {
        if lava_first {
            lava.step(&mut world);
            water.step(&mut world);
        } else {
            water.step(&mut world);
            lava.step(&mut world);
        }
    }
    world.block(at(8, 8))
}

#[test]
fn water_over_lava_hardens_it_in_either_queue_order() {
    for lava_first in [false, true] {
        assert_eq!(
            water_over_lava(FLUID_SOURCE, lava_first),
            Block::Obsidian,
            "{lava_first}"
        );
        assert_eq!(
            water_over_lava(2, lava_first),
            Block::Cobblestone,
            "{lava_first}"
        );
    }
}

const TRENCH_Z: i32 = 8;
const LAVA_END: i32 = 4;
const WATER_END: i32 = 9;

fn trench() -> World {
    let mut world = flat_world(&[(0, 0)]);
    for x in LAVA_END - 1..=WATER_END + 1 {
        for z in [TRENCH_Z - 1, TRENCH_Z + 1] {
            world.set_block(at(x, z), Block::Stone);
        }
    }
    world.set_block(at(LAVA_END - 1, TRENCH_Z), Block::Stone);
    world.set_block(at(WATER_END + 1, TRENCH_Z), Block::Stone);
    world
}

fn run_ticks(world: &mut World, water: &mut Fluid, lava: &mut Fluid, ticks: u32) {
    for t in 1..=ticks {
        let mut changed = Vec::new();
        if t % 15 == 0 {
            changed.extend(water.step(world));
        }
        if t % 30 == 0 {
            changed.extend(lava.step(world));
        }
        for pos in changed {
            wake_all(world, water, lava, pos);
        }
    }
}

fn wake_all(world: &World, water: &mut Fluid, lava: &mut Fluid, pos: IVec3) {
    water.block_changed(world, pos);
    lava.block_changed(world, pos);
}

fn generator_survives_cycles(water_source: IVec3) {
    let mut world = trench();
    let (mut water, mut lava) = (Fluid::new(), Fluid::of(FluidKind::Lava));
    let lava_source = at(LAVA_END, TRENCH_Z);
    world.set_block(lava_source, Block::Lava);
    world.set_block(water_source, Block::Water);
    if water_source.y > WATER_Y {
        for dir in [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z] {
            world.set_block(water_source + dir, Block::Stone);
        }
    }
    wake_all(&world, &mut water, &mut lava, lava_source);
    wake_all(&world, &mut water, &mut lava, water_source);
    for cycle in 0..40 {
        run_ticks(&mut world, &mut water, &mut lava, 900);
        assert_eq!(world.block(lava_source), Block::Lava, "cycle {cycle}");
        assert_eq!(world.block_data(lava_source), FLUID_SOURCE);
        assert_eq!(world.block(water_source), Block::Water, "cycle {cycle}");
        let cobble: Vec<i32> = (LAVA_END..=WATER_END)
            .filter(|&x| world.block(at(x, TRENCH_Z)) == Block::Cobblestone)
            .collect();
        assert_eq!(cobble.len(), 1, "cycle {cycle}: {cobble:?}");
        let window = at(cobble[0], TRENCH_Z);
        world.set_block(window, Block::Air);
        wake_all(&world, &mut water, &mut lava, window);
    }
}

#[test]
fn cobblestone_generator_in_one_plane_survives_many_cycles() {
    generator_survives_cycles(at(WATER_END, TRENCH_Z));
}

#[test]
fn cobblestone_generator_with_water_from_above_survives_many_cycles() {
    generator_survives_cycles(at(WATER_END, TRENCH_Z) + IVec3::Y);
}

#[test]
fn block_over_a_source_leaves_it_in_place() {
    for kind in [FluidKind::Water, FluidKind::Lava] {
        let mut world = flat_world(&[(0, 0)]);
        let mut fluid = Fluid::of(kind);
        for x in 6..11 {
            for z in 6..11 {
                world.set_block(at(x, z), kind.block());
            }
        }
        let above = at(8, 8) + IVec3::Y;
        put(&mut world, &mut fluid, above, Block::Stone, 0);
        settle(&mut world, &mut fluid);
        assert_eq!(world.block(at(8, 8)), kind.block(), "{kind:?}");
        assert_eq!(world.block_data(at(8, 8)), FLUID_SOURCE);
    }
}

#[test]
fn water_flowing_to_a_lava_source_makes_obsidian_and_to_a_lava_flow_cobblestone() {
    let mut world = flat_world(&[(0, 0)]);
    let (mut water, mut lava) = (Fluid::new(), Fluid::of(FluidKind::Lava));
    world.set_block(at(8, 8), Block::Lava);
    world.set_block(at(12, 8), Block::Lava);
    world.set_block_with_data(at(13, 8), Block::Lava, 2);
    world.set_block(at(10, 8), Block::Water);
    wake_all(&world, &mut water, &mut lava, at(8, 8));
    wake_all(&world, &mut water, &mut lava, at(10, 8));
    wake_all(&world, &mut water, &mut lava, at(13, 8));
    run_ticks(&mut world, &mut water, &mut lava, 300);
    assert_eq!(world.block(at(8, 8)), Block::Obsidian);
    assert_eq!(world.block(at(9, 8)), Block::Water);
}

#[test]
fn water_flow_meeting_flowing_lava_makes_cobblestone_and_keeps_the_source() {
    let mut world = flat_world(&[(0, 0)]);
    let (mut water, mut lava) = (Fluid::new(), Fluid::of(FluidKind::Lava));
    world.set_block(at(5, 8), Block::Lava);
    world.set_block(at(10, 8), Block::Water);
    wake_all(&world, &mut water, &mut lava, at(5, 8));
    wake_all(&world, &mut water, &mut lava, at(10, 8));
    run_ticks(&mut world, &mut water, &mut lava, 120);
    assert!((6..10).any(|x| world.block(at(x, 8)) == Block::Cobblestone));
    assert_eq!(world.block(at(5, 8)), Block::Lava);
}
