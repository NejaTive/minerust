use std::hint::black_box;
use std::time::{Duration, Instant};

use super::*;
use world::terrain::Terrain;

/// Мир 14×14 колонок с светом и ключи внутренних 12×12 (у них все соседи загружены).
fn setup() -> (World, LightMap, Vec<ColumnPos>) {
    let terrain = Terrain::new(1337);
    let mut world = World::new();
    for z in -7..7 {
        for x in -7..7 {
            let key = ColumnPos::new(x, z);
            world.insert(key, terrain.generate(key));
        }
    }
    let keys = (-6..6)
        .flat_map(|z| (-6..6).map(move |x| ColumnPos::new(x, z)))
        .collect();
    let mut light = LightMap::new();
    for key in world.keys().collect::<Vec<_>>() {
        light.add_column(&world, key);
    }
    (world, light, keys)
}

/// Среднее время `mesh_world_column` на квадрате колонок 12×12 (соседи загружены);
/// запуск: `cargo test --release mesh_timing -- --ignored --nocapture`.
#[test]
#[ignore]
fn mesh_timing() {
    let (world, light, keys) = setup();
    let start = Instant::now();
    for &key in &keys {
        black_box(mesh_world_column(
            &world,
            &light,
            key,
            MeshOptions::default(),
        ));
    }
    let per_column = start.elapsed() / keys.len() as u32;
    println!("mesh: {per_column:?} per column");
}

/// Время `mesh_world_column` по этапам, на колонку, при AO и без. Кэш (блоки и свет с
/// соседями) меряется отдельно через `Surroundings::fill`. Цикл граней (AO, свет
/// вершин, запись) отдельно не выделить без правки мешера, поэтому его время — это
/// полный меш минус кэш, а запись квадов оценивается пересборкой готовых мешей
/// копированием записей квадов (без расчёта AO и света). Расчёт AO и света
/// вершин — остаток. Запуск:
/// `cargo test --release mesh_stages -- --ignored --nocapture`.
#[test]
#[ignore]
fn mesh_stages() {
    const REPEATS: u32 = 3;
    let (world, light, keys) = setup();
    for options in [
        MeshOptions::default(),
        MeshOptions {
            ambient_occlusion: false,
        },
    ] {
        let (mut total, mut cache, mut write) = (Duration::ZERO, Duration::ZERO, Duration::ZERO);
        let mut quads = 0usize;
        let (mut blocks, mut lights) = new_caches();
        for _ in 0..REPEATS {
            let mut meshes = Vec::new();
            for &key in &keys {
                let around = Surroundings::new(&world, &light, key).unwrap();
                for sy in 0..COLUMN_SECTIONS {
                    if around.columns[4].unwrap().section(sy).is_air() {
                        continue;
                    }
                    let start = Instant::now();
                    around.fill(sy, &mut blocks, &mut lights);
                    black_box((&blocks, &lights));
                    cache += start.elapsed();

                    let start = Instant::now();
                    around.fill(sy, &mut blocks, &mut lights);
                    let data = mesh_cached(&blocks, &lights, options);
                    total += start.elapsed();
                    meshes.push(black_box(data));
                }
            }
            let start = Instant::now();
            for data in &meshes {
                for mesh in [&data.solid, &data.water] {
                    let mut copy = MeshData::new();
                    for quad in &mesh.quads {
                        copy.quads.push(*quad);
                        quads += 1;
                    }
                    black_box(copy);
                }
            }
            write += start.elapsed();
        }
        let per = |d: Duration| d / (REPEATS * keys.len() as u32);
        let (total, cache, write) = (per(total), per(cache), per(write));
        println!(
            "ao={}: total {total:?} (с кэшем) | cache {cache:?} | faces (AO+light+write) {:?} | write ≈ {write:?} | AO+light ≈ {:?} | quads/column {}",
            options.ambient_occlusion,
            total.saturating_sub(cache),
            total.saturating_sub(cache + write),
            quads / (REPEATS as usize * keys.len()),
        );
    }
}
