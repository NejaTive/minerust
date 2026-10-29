use std::collections::VecDeque;

use super::*;
use crate::terrain::Terrain;

fn square(center: ColumnPos, radius: i32) -> Vec<ColumnPos> {
    (-radius..=radius)
        .flat_map(|z| (-radius..=radius).map(move |x| center + IVec2::new(x, z)))
        .collect()
}

fn world_of(keys: &[ColumnPos], block_at: impl Fn(IVec3) -> Block) -> World {
    let mut world = World::new();
    for &key in keys {
        let mut column = Column::new();
        for y in 0..WORLD_HEIGHT {
            for z in 0..SECTION_SIZE {
                for x in 0..SECTION_SIZE {
                    let local = IVec3::new(x, y, z);
                    let pos = local + key.origin();
                    column.set(local, block_at(pos));
                }
            }
        }
        world.insert(key, column);
    }
    world
}

fn terrain_world(keys: &[ColumnPos]) -> World {
    let terrain = Terrain::new(1337);
    let mut world = World::new();
    for &key in keys {
        world.insert(key, terrain.generate(key));
    }
    world
}

fn lit(world: &World, order: &[ColumnPos]) -> LightMap {
    let mut light = LightMap::new();
    for &key in order {
        light.add_column(world, key);
    }
    light
}

fn reference(world: &World, keys: &[ColumnPos]) -> HashMap<IVec3, u8> {
    let mut packed = reference_channel(world, keys, Channel::Sky);
    for (pos, level) in reference_channel(world, keys, Channel::Block) {
        let sky = packed.get(&pos).copied().unwrap_or(0);
        packed.insert(pos, sky | level << 4);
    }
    packed
}

fn reference_channel(world: &World, keys: &[ColumnPos], channel: Channel) -> HashMap<IVec3, u8> {
    let loaded: HashSet<ColumnPos> = keys.iter().copied().collect();
    let inside =
        |p: IVec3| (0..WORLD_HEIGHT).contains(&p.y) && loaded.contains(&ColumnPos::of_block(p));
    let mut light: HashMap<IVec3, u8> = HashMap::new();
    let mut queue = VecDeque::new();
    for &key in keys {
        for y in 0..WORLD_HEIGHT {
            for z in 0..SECTION_SIZE {
                for x in 0..SECTION_SIZE {
                    let pos = key.origin() + IVec3::new(x, y, z);
                    let level = match channel {
                        Channel::Sky if y == WORLD_HEIGHT - 1 => {
                            channel.step(MAX_LIGHT, true, world.block(pos))
                        }
                        Channel::Sky => 0,
                        Channel::Block => world.block(pos).emission(),
                    };
                    if level > 0 {
                        light.insert(pos, level);
                        queue.push_back(pos);
                    }
                }
            }
        }
    }
    while let Some(pos) = queue.pop_front() {
        let level = light[&pos];
        for dir in DIRECTIONS {
            let next = pos + dir;
            if !inside(next) {
                continue;
            }
            let value = channel.step(level, dir == IVec3::NEG_Y, world.block(next));
            if value > light.get(&next).copied().unwrap_or(0) {
                light.insert(next, value);
                queue.push_back(next);
            }
        }
    }
    light
}

fn assert_matches_reference(light: &LightMap, world: &World, keys: &[ColumnPos], context: &str) {
    let expected = reference(world, keys);
    for &key in keys {
        for y in 0..WORLD_HEIGHT {
            for z in 0..SECTION_SIZE {
                for x in 0..SECTION_SIZE {
                    let pos = key.origin() + IVec3::new(x, y, z);
                    let want = expected.get(&pos).copied().unwrap_or(0);
                    assert_eq!(light.packed(pos), want, "{context}: {pos}");
                }
            }
        }
    }
}

fn floor(y: i32) -> impl Fn(IVec3) -> Block {
    move |p: IVec3| if p.y <= y { Block::Stone } else { Block::Air }
}

#[test]
fn open_sky_is_full_above_ground() {
    let keys = square(ColumnPos::new(-1, 2), 1);
    let world = world_of(&keys, floor(10));
    let light = lit(&world, &keys);
    for &key in &keys {
        for y in 0..WORLD_HEIGHT + 2 {
            let pos = key.origin() + IVec3::new(7, y, 15);
            assert_eq!(light.sky(pos), if y > 10 { MAX_LIGHT } else { 0 }, "{pos}");
        }
    }
    assert_eq!(light.sky(IVec3::new(0, -1, 0)), 0);
    assert_eq!(light.sky(IVec3::new(1000, 50, 0)), 0);
}

#[test]
fn light_under_canopy_fades_from_edge() {
    let keys = square(ColumnPos::ZERO, 1);
    let world = world_of(&keys, |p| {
        if p.y <= 10 || (p.y == 14 && p.x >= 0) {
            Block::Stone
        } else {
            Block::Air
        }
    });
    let light = lit(&world, &keys);
    for x in 0..20 {
        for y in 11..14 {
            let want = (14 - x).max(0) as u8;
            assert_eq!(light.sky(IVec3::new(x, y, 3)), want, "x {x}, y {y}");
        }
    }
    assert_eq!(light.sky(IVec3::new(-1, 12, 3)), MAX_LIGHT);
    assert_matches_reference(&light, &world, &keys, "canopy");
}

#[test]
fn hole_in_box_lights_and_closing_darkens() {
    let keys = square(ColumnPos::ZERO, 1);
    let shell = |p: IVec3| {
        let inner = p - IVec3::new(4, 40, 4);
        inner.cmpge(IVec3::ZERO).all()
            && inner.cmplt(IVec3::splat(9)).all()
            && !(inner.cmpgt(IVec3::ZERO).all() && inner.cmplt(IVec3::splat(8)).all())
    };
    let mut world = world_of(&keys, |p| {
        if p.y <= 10 || shell(p) {
            Block::Stone
        } else {
            Block::Air
        }
    });
    let mut light = lit(&world, &keys);
    let inside = IVec3::new(8, 44, 8);
    assert_eq!(light.sky(inside), 0);
    let closed: Vec<u8> = light.column(ColumnPos::ZERO).unwrap().0.to_vec();

    let hole = IVec3::new(8, 48, 8);
    assert!(world.set_block(hole, Block::Air));
    let changed = light.block_changed(&world, hole);
    for y in 41..=48 {
        assert_eq!(light.sky(IVec3::new(8, y, 8)), MAX_LIGHT, "y {y}");
    }
    assert_eq!(light.sky(IVec3::new(6, 41, 8)), 13);
    assert_eq!(light.sky(IVec3::new(5, 41, 5)), 9);
    assert!(changed.contains(&SectionPos::new(0, 2, 0)), "{changed:?}");
    assert_matches_reference(&light, &world, &keys, "open");

    assert!(world.set_block(hole, Block::Stone));
    let changed = light.block_changed(&world, hole);
    assert!(changed.contains(&SectionPos::new(0, 2, 0)), "{changed:?}");
    assert_eq!(light.column(ColumnPos::ZERO).unwrap().0.to_vec(), closed);
}

#[test]
fn water_dims_by_two_and_leaves_by_one() {
    let keys = square(ColumnPos::ZERO, 1);
    let world = world_of(&keys, |p| match p.y {
        ..=50 => Block::Stone,
        51..=60 => Block::Water,
        70..=74 if p.x >= 16 => Block::Leaves,
        _ => Block::Air,
    });
    let light = lit(&world, &keys);
    for depth in 1..=10 {
        let want = (15 - 2 * depth).max(0) as u8;
        assert_eq!(
            light.sky(IVec3::new(3, 61 - depth, 3)),
            want,
            "depth {depth}"
        );
    }
    for depth in 1..=5 {
        assert_eq!(light.sky(IVec3::new(24, 75 - depth, 3)), 15 - depth as u8);
    }
    assert_eq!(light.sky(IVec3::new(24, 69, 3)), 9);
    assert_eq!(light.sky(IVec3::new(24, 68, 3)), 8);
    assert_matches_reference(&light, &world, &keys, "water");
}

const CENTERS: [ColumnPos; 2] = [ColumnPos::new(-1, 2), ColumnPos::new(-6, 6)];

#[test]
fn order_of_columns_does_not_matter() {
    for center in CENTERS {
        let keys = square(center, 1);
        let world = terrain_world(&keys);
        let forward = lit(&world, &keys);
        let mut backward_order = keys.clone();
        backward_order.reverse();
        backward_order.retain(|&k| k != center);
        backward_order.push(center);
        let backward = lit(&world, &backward_order);
        for &key in &keys {
            assert_eq!(
                forward.column(key).unwrap().0[..],
                backward.column(key).unwrap().0[..],
                "{key}"
            );
        }
        assert_matches_reference(&forward, &world, &keys, "terrain");
    }
}

#[test]
fn remove_and_add_again_restores_light() {
    let center = CENTERS[0];
    let keys = square(center, 1);
    let world = terrain_world(&keys);
    let mut light = lit(&world, &keys);
    let before: Vec<Vec<u8>> = keys
        .iter()
        .map(|&k| light.column(k).unwrap().0.to_vec())
        .collect();

    for removed in [center, center + IVec2::X, center + IVec2::ONE] {
        light.remove_column(&world, removed);
        assert!(light.column(removed).is_none());
        let rest: Vec<ColumnPos> = keys.iter().copied().filter(|&k| k != removed).collect();
        assert_matches_reference(&light, &world, &rest, "removed");
        light.add_column(&world, removed);
        let after: Vec<Vec<u8>> = keys
            .iter()
            .map(|&k| light.column(k).unwrap().0.to_vec())
            .collect();
        assert!(after == before, "{removed}");
    }
}

#[test]
fn random_edits_match_full_recompute() {
    let center = CENTERS[0];
    let keys = square(center, 1);
    let terrain = Terrain::new(1337);
    let mut world = terrain_world(&keys);
    let mut light = lit(&world, &keys);
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    let mut next = move |n: i32| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % n as u64) as i32
    };
    let blocks = [
        Block::Air,
        Block::Air,
        Block::Stone,
        Block::Leaves,
        Block::Water,
        Block::Glowstone,
    ];
    let origin = (center + IVec2::NEG_ONE).origin();
    for step in 0..40 {
        let (x, z) = (next(48), next(48));
        let surface = terrain.surface_height(origin.x + x, origin.z + z);
        let y = (surface - 6 + next(10)).clamp(0, WORLD_HEIGHT - 1);
        let pos = origin + IVec3::new(x, y, z);
        let block = blocks[next(blocks.len() as i32) as usize];
        assert!(world.set_block(pos, block));
        let changed = light.block_changed(&world, pos);
        if step % 4 == 3 {
            assert_matches_reference(
                &light,
                &world,
                &keys,
                &format!("step {step} {pos} {block:?}"),
            );
        }
        for key in changed {
            assert!((0..8).contains(&key.y()), "{key}");
        }
    }
    assert_matches_reference(&light, &world, &keys, "end");
}

#[test]
fn changed_sections_cover_every_changed_cell() {
    let center = CENTERS[0];
    let keys = square(center, 1);
    let terrain = Terrain::new(1337);
    let mut world = terrain_world(&keys);
    let mut light = lit(&world, &keys);
    let x = center.origin().x + 15;
    let z = center.origin().z;
    let top = terrain.surface_height(x, z);
    for y in (top - 12..=top).rev() {
        let before = reference(&world, &keys);
        let pos = IVec3::new(x, y, z);
        world.set_block(pos, Block::Air);
        let changed: HashSet<SectionPos> = light.block_changed(&world, pos).into_iter().collect();
        for (cell, level) in reference(&world, &keys) {
            if before.get(&cell).copied().unwrap_or(0) != level {
                for key in affected_sections(cell) {
                    assert!(changed.contains(&key), "{cell}: {key} missing");
                }
            }
        }
    }
}

fn dark_hall() -> (World, Vec<ColumnPos>) {
    let keys = square(ColumnPos::ZERO, 1);
    let world = world_of(&keys, |p| {
        if (10..=40).contains(&p.y) && (-16..32).contains(&p.x) && (-16..32).contains(&p.z) {
            Block::Air
        } else {
            Block::Stone
        }
    });
    (world, keys)
}

fn place(world: &mut World, light: &mut LightMap, pos: IVec3, block: Block) {
    assert!(world.set_block(pos, block));
    light.block_changed(world, pos);
}

#[test]
fn light_bits_are_packed_independently() {
    for sky in 0..=MAX_LIGHT {
        for block in 0..=MAX_LIGHT {
            let packed = pack_light(sky, block);
            assert_eq!((sky_level(packed), block_level(packed)), (sky, block));
            assert_eq!(Channel::Sky.with(packed, 7), pack_light(7, block));
            assert_eq!(Channel::Block.with(packed, 7), pack_light(sky, 7));
        }
    }
    assert_eq!(pack_light(MAX_LIGHT, 0), MAX_LIGHT);
}

#[test]
fn glowstone_fades_by_one_across_column_border() {
    let (mut world, keys) = dark_hall();
    let mut light = lit(&world, &keys);
    let source = IVec3::new(8, 20, 8);
    place(&mut world, &mut light, source, Block::Glowstone);
    assert_eq!(light.block(source), 15);
    for d in 1..=14 {
        let level = 15 - d as u8;
        assert_eq!(light.block(source + IVec3::X * d), level, "x {d}");
        assert_eq!(light.block(source + IVec3::NEG_Z * d), level, "z {d}");
    }
    assert_eq!(light.block(source + IVec3::X * 15), 0);
    assert_eq!(light.sky(source), 0);
    assert_matches_reference(&light, &world, &keys, "placed");
}

#[test]
fn breaking_glowstone_removes_its_light() {
    let (mut world, keys) = dark_hall();
    let mut light = lit(&world, &keys);
    let source = IVec3::new(15, 20, 15);
    place(&mut world, &mut light, source, Block::Glowstone);
    place(&mut world, &mut light, source, Block::Air);
    for y in 0..WORLD_HEIGHT {
        for z in -16..32 {
            for x in -16..32 {
                assert_eq!(light.packed(IVec3::new(x, y, z)), 0, "{x} {y} {z}");
            }
        }
    }
}

#[test]
fn two_sources_do_not_add_up_and_one_can_go() {
    let (mut world, keys) = dark_hall();
    let mut light = lit(&world, &keys);
    let (a, b) = (IVec3::new(10, 20, 8), IVec3::new(12, 20, 8));
    place(&mut world, &mut light, a, Block::Glowstone);
    place(&mut world, &mut light, b, Block::Glowstone);
    assert_eq!(light.block(IVec3::new(11, 20, 8)), 14);
    assert_eq!(light.block(a), 15);
    place(&mut world, &mut light, a, Block::Air);
    assert_eq!(light.block(a), 13);
    assert_eq!(light.block(IVec3::new(11, 20, 8)), 14);
    assert_eq!(light.block(b), 15);
    assert_matches_reference(&light, &world, &keys, "one removed");
}

#[test]
fn opaque_block_shadows_and_removing_wall_lets_light_through() {
    let (mut world, keys) = dark_hall();
    let mut light = lit(&world, &keys);
    let source = IVec3::new(2, 20, 8);
    place(&mut world, &mut light, source, Block::Glowstone);
    for y in 10..=40 {
        for z in -16..32 {
            world.set_block(IVec3::new(4, y, z), Block::Stone);
        }
    }
    for y in 10..=40 {
        for z in -16..32 {
            light.block_changed(&world, IVec3::new(4, y, z));
        }
    }
    assert_eq!(light.block(IVec3::new(3, 20, 8)), 14);
    assert_eq!(light.block(IVec3::new(5, 20, 8)), 0);
    assert_matches_reference(&light, &world, &keys, "walled");
    place(&mut world, &mut light, IVec3::new(4, 20, 8), Block::Air);
    assert_eq!(light.block(IVec3::new(4, 20, 8)), 13);
    assert_eq!(light.block(IVec3::new(5, 20, 8)), 12);
    assert_matches_reference(&light, &world, &keys, "opened");
}

#[test]
fn glowstone_light_does_not_depend_on_load_order() {
    let (mut world, keys) = dark_hall();
    for pos in [
        IVec3::new(15, 20, 15),
        IVec3::new(16, 30, 0),
        IVec3::new(-1, 12, 31),
    ] {
        world.set_block(pos, Block::Glowstone);
    }
    let reversed: Vec<ColumnPos> = keys.iter().rev().copied().collect();
    let a = lit(&world, &keys);
    let b = lit(&world, &reversed);
    assert_matches_reference(&a, &world, &keys, "forward");
    assert_matches_reference(&b, &world, &keys, "reversed");
    let mut c = lit(&world, &keys);
    c.remove_column(&world, ColumnPos::new(1, 1));
    c.add_column(&world, ColumnPos::new(1, 1));
    assert_matches_reference(&c, &world, &keys, "reloaded");
}

#[test]
fn brightness_is_monotonic_and_bounded() {
    assert_eq!(brightness(MAX_LIGHT), 1.0);
    assert!(brightness(0) >= 0.05);
    for level in 1..=MAX_LIGHT {
        assert!(brightness(level) >= brightness(level - 1), "{level}");
    }
    assert!(brightness(MAX_LIGHT) > brightness(MAX_LIGHT - 1));
    assert!(brightness(2) > brightness(1));
}
