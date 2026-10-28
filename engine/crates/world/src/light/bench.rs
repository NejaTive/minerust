use super::*;
use crate::terrain::Terrain;

/// Среднее время `add_column` на квадрате 12×12 (колонки добавляются по строкам,
/// как при подгрузке: соседи частью уже есть) и `block_changed`; запуск:
/// `cargo test --release light_timing -- --ignored --nocapture`.
#[test]
#[ignore]
fn light_timing() {
    let terrain = Terrain::new(1337);
    let keys: Vec<IVec2> = (-6..6)
        .flat_map(|z| (-6..6).map(move |x| IVec2::new(x, z)))
        .collect();
    let mut world = World::new();
    for &key in &keys {
        world.insert(key, terrain.generate(key));
    }
    let mut light = LightMap::new();
    let start = std::time::Instant::now();
    for &key in &keys {
        std::hint::black_box(light.add_column(&world, key));
    }
    let per_column = start.elapsed() / keys.len() as u32;
    println!("add_column: {per_column:?} per column");

    let start = std::time::Instant::now();
    let mut edits = 0;
    for x in -40..40 {
        let pos = IVec3::new(x, terrain.surface_height(x, 7), 7);
        world.set_block(pos, Block::Air);
        std::hint::black_box(light.block_changed(&world, pos));
        edits += 1;
    }
    println!("block_changed: {:?} per edit", start.elapsed() / edits);
    println!(
        "memory: {} KiB per column",
        light.memory_bytes() / light.len() / 1024
    );
}

/// Худший частый случай правки: пробить кровлю тёмного зала (свет заливает зал)
/// и заложить обратно (свет снимается). Зал найден для сида 838037976: потолок
/// над (142, −70) — y 61..=64, над ним небо. Запуск:
/// `cargo test --release cave_edit_timing -- --ignored --nocapture`.
#[test]
#[ignore]
fn cave_edit_timing() {
    let terrain = Terrain::new(838037976);
    let center = column_key(142, -70);
    let mut world = World::new();
    for z in -2..=2 {
        for x in -2..=2 {
            let key = center + IVec2::new(x, z);
            world.insert(key, terrain.generate(key));
        }
    }
    let mut light = LightMap::new();
    for key in world.keys().collect::<Vec<_>>() {
        light.add_column(&world, key);
    }
    let hall = IVec3::new(142, 55, -70);
    assert_eq!(light.sky(hall), 0);
    let roof: Vec<IVec3> = (61..=64).map(|y| IVec3::new(142, y, -70)).collect();
    for (block, label) in [(Block::Air, "open"), (Block::Stone, "close")] {
        let mut order = roof.clone();
        if block == Block::Air {
            order.reverse();
        }
        for pos in order {
            world.set_block(pos, block);
            let start = std::time::Instant::now();
            let changed = light.block_changed(&world, pos);
            println!(
                "{label} {pos}: {:?}, {} sections, hall {}",
                start.elapsed(),
                changed.len(),
                light.sky(hall)
            );
        }
    }
}
