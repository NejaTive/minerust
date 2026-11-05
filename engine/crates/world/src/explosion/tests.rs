use super::*;
use crate::column::Column;
use crate::pos::ColumnPos;

const CENTER: IVec3 = IVec3::new(24, 30, 24);

fn solid_world(block: Block) -> World {
    let mut world = World::new();
    for z in 0..3 {
        for x in 0..3 {
            let mut column = Column::new();
            for y in 0..60 {
                for lz in 0..16 {
                    for lx in 0..16 {
                        column.set(IVec3::new(lx, y, lz), block);
                    }
                }
            }
            world.insert(ColumnPos::new(x, z), column);
        }
    }
    world.set_block(CENTER, Block::Air);
    world
}

fn middle() -> Vec3 {
    CENTER.as_vec3() + Vec3::splat(0.5)
}

fn average_random() -> impl FnMut() -> f32 {
    || 0.5
}

fn seeded(mut state: u32) -> impl FnMut() -> f32 {
    move || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        (state >> 8) as f32 / (1u32 << 24) as f32
    }
}

#[test]
fn stone_breaks_a_layer_or_two_around_the_center() {
    let world = solid_world(Block::Stone);
    let cells = destroyed_cells(&world, middle(), TNT_POWER, average_random());
    for dir in [
        IVec3::X,
        IVec3::NEG_X,
        IVec3::Y,
        IVec3::NEG_Y,
        IVec3::Z,
        IVec3::NEG_Z,
    ] {
        assert!(cells.contains(&(CENTER + dir)), "{dir}");
    }
    assert!(cells.iter().all(|c| (*c - CENTER).abs().max_element() <= 3));
    assert!(!cells.contains(&(CENTER + IVec3::new(4, 0, 0))));
}

#[test]
fn explosion_is_deterministic_for_a_seed() {
    let world = solid_world(Block::Stone);
    let first = destroyed_cells(&world, middle(), TNT_POWER, seeded(7));
    let second = destroyed_cells(&world, middle(), TNT_POWER, seeded(7));
    assert_eq!(first, second);
    assert!(!first.is_empty());
}

#[test]
fn softer_blocks_break_further_than_stone() {
    let dirt = destroyed_cells(
        &solid_world(Block::Dirt),
        middle(),
        TNT_POWER,
        average_random(),
    );
    let stone = destroyed_cells(
        &solid_world(Block::Stone),
        middle(),
        TNT_POWER,
        average_random(),
    );
    assert!(dirt.len() > stone.len());
}

#[test]
fn obsidian_and_bedrock_are_untouched() {
    for block in [Block::Obsidian, Block::Bedrock, Block::Water, Block::Lava] {
        let world = solid_world(block);
        let cells = destroyed_cells(&world, middle(), TNT_POWER, seeded(3));
        assert!(cells.is_empty(), "{block:?}");
    }
}

#[test]
fn obsidian_shell_protects_the_stone_behind_it() {
    let mut world = solid_world(Block::Stone);
    for x in -1..=1 {
        for y in -1..=1 {
            for z in -1..=1 {
                if (x, y, z) != (0, 0, 0) {
                    world.set_block(CENTER + IVec3::new(x, y, z), Block::Obsidian);
                }
            }
        }
    }
    assert!(destroyed_cells(&world, middle(), TNT_POWER, seeded(5)).is_empty());
}

#[test]
fn destroy_clears_cells_and_reports_tnt() {
    let mut world = solid_world(Block::Stone);
    let tnt = CENTER + IVec3::X;
    world.set_block(tnt, Block::Tnt);
    let cells = destroyed_cells(&world, middle(), TNT_POWER, average_random());
    let destruction = destroy(&mut world, &cells);
    assert_eq!(destruction.tnt, vec![tnt]);
    assert_eq!(world.block(tnt), Block::Air);
    assert_eq!(world.block(CENTER + IVec3::Y), Block::Air);
    assert_eq!(world.block(CENTER + IVec3::new(0, 6, 0)), Block::Stone);
}

#[test]
fn knockback_points_away_and_fades_with_distance() {
    let world = solid_world(Block::Air);
    let body = |x: f32| {
        let feet = Vec3::new(x, 30.0, 24.5);
        (
            feet - Vec3::new(0.3, 0.0, 0.3),
            feet + Vec3::new(0.3, 1.8, 0.3),
        )
    };
    let near = knockback(&world, middle(), TNT_POWER, body(26.5), 1.62);
    let far = knockback(&world, middle(), TNT_POWER, body(30.5), 1.62);
    assert!(near.x > 0.0 && far.x > 0.0);
    assert!(near.length() > far.length());
    assert_eq!(
        knockback(&world, middle(), TNT_POWER, body(40.5), 1.62),
        Vec3::ZERO
    );
}

#[test]
fn walls_shield_from_knockback() {
    let mut world = solid_world(Block::Air);
    for y in 25..40 {
        for z in 20..30 {
            world.set_block(IVec3::new(26, y, z), Block::Stone);
        }
    }
    let feet = Vec3::new(28.5, 30.0, 24.5);
    let body = (
        feet - Vec3::new(0.3, 0.0, 0.3),
        feet + Vec3::new(0.3, 1.8, 0.3),
    );
    assert_eq!(
        knockback(&world, middle(), TNT_POWER, body, 1.62),
        Vec3::ZERO
    );
}
