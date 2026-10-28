use super::*;

fn air(_: ColumnPos) -> Column {
    Column::new()
}

fn circle_points(r: i32) -> usize {
    circle_offsets(r).len()
}

fn dist2(a: ColumnPos, b: ColumnPos) -> i32 {
    (a - b).length_squared()
}

/// Один кадр движка: `update`, затем до `steps` шагов `advance`.
/// Проверяет, что снимаются только мешированные колонки, и ведёт их учёт.
fn frame(
    loader: &mut Loader,
    world: &mut World,
    center: ColumnPos,
    meshed: &mut HashSet<ColumnPos>,
    steps: usize,
) -> Vec<Progress> {
    for key in loader.update(center, world) {
        assert!(meshed.remove(&key), "unmesh of {key} without mesh");
    }
    let mut done = Vec::new();
    for _ in 0..steps {
        let progress = loader.advance(world, air);
        if let Progress::Mesh(key) = progress {
            assert!(meshed.insert(key), "{key} meshed twice");
        }
        if progress == Progress::Done {
            break;
        }
        done.push(progress);
    }
    done
}

/// Гоняет загрузчик в `center`, пока он не закончит всю работу.
fn settle(
    loader: &mut Loader,
    world: &mut World,
    center: ColumnPos,
    meshed: &mut HashSet<ColumnPos>,
) {
    while !frame(loader, world, center, meshed, usize::MAX).is_empty() {}
}

/// Загружен ровно круг `radius + 2`, мешируется ровно круг `radius`.
fn assert_circles(world: &World, meshed: &HashSet<ColumnPos>, center: ColumnPos, radius: i32) {
    let r = radius + 2;
    for z in -r - 2..=r + 2 {
        for x in -r - 2..=r + 2 {
            let key = center + IVec2::new(x, z);
            let d2 = dist2(key, center);
            assert_eq!(world.column(key).is_some(), d2 <= r * r, "{key}");
            assert_eq!(meshed.contains(&key), d2 <= radius * radius, "{key}");
        }
    }
}

fn assert_neighbors_loaded(world: &World, meshed: &HashSet<ColumnPos>) {
    for &key in meshed {
        assert!(world.column(key).is_some(), "meshed {key} unloaded");
        for n in NEIGHBORS {
            assert!(
                world.column(key + n).is_some(),
                "neighbor of {key} unloaded"
            );
        }
    }
}

#[test]
fn loads_and_meshes_circles() {
    let center = ColumnPos::new(-5, 3);
    let mut loader = Loader::new(center);
    let mut world = World::new();
    let mut meshed = HashSet::new();
    settle(&mut loader, &mut world, center, &mut meshed);
    assert_circles(&world, &meshed, center, DEFAULT_RADIUS);
    assert_eq!(loader.pending(), 0);
}

#[test]
fn generates_nearest_first_and_meshes_when_neighbors_ready() {
    let mut loader = Loader::new(ColumnPos::ZERO);
    let mut world = World::new();
    let mut meshed = HashSet::new();
    // Центр и 8 соседей — квадрат 3×3, после них центр готов к мешу.
    let first = frame(&mut loader, &mut world, ColumnPos::ZERO, &mut meshed, 10);
    assert_eq!(first[0], Progress::Generated(ColumnPos::ZERO));
    assert!(first[..9].iter().all(|p| matches!(
        p,
        Progress::Generated(key) if key.dist2(ColumnPos::ZERO) <= 2
    )));
    assert_eq!(first[9], Progress::Mesh(ColumnPos::ZERO));

    let mut generated = Vec::new();
    while let progress @ [_, ..] =
        &frame(&mut loader, &mut world, ColumnPos::ZERO, &mut meshed, 7)[..]
    {
        assert!(progress.len() <= 7);
        generated.extend(progress.iter().filter_map(|p| match p {
            Progress::Generated(key) => Some(key.dist2(ColumnPos::ZERO)),
            _ => None,
        }));
    }
    assert!(generated.is_sorted(), "{generated:?}");
    assert_eq!(loader.pending(), 0);
}

#[test]
fn column_without_all_neighbors_is_not_meshed() {
    // Без одного соседа — по грани или по диагонали — колонка ждёт его загрузки.
    for missing in NEIGHBORS {
        let mut loader = Loader::new(ColumnPos::ZERO);
        let mut world = World::new();
        world.insert(ColumnPos::ZERO, Column::new());
        for n in NEIGHBORS.into_iter().filter(|&n| n != missing) {
            world.insert(ColumnPos(n), Column::new());
        }
        loader.update(ColumnPos::ZERO, &mut world);
        assert_eq!(
            loader.advance(&mut world, air),
            Progress::Generated(ColumnPos(missing))
        );
        assert_eq!(
            loader.advance(&mut world, air),
            Progress::Mesh(ColumnPos::ZERO)
        );
    }
}

#[test]
fn unloads_only_beyond_margin() {
    let mut loader = Loader::new(ColumnPos::ZERO);
    let mut world = World::new();
    let mut meshed = HashSet::new();
    settle(&mut loader, &mut world, ColumnPos::ZERO, &mut meshed);
    let before: HashSet<ColumnPos> = world.keys().collect();

    // Сдвиг на 2 колонки: старые данные дальше radius + 2 от нового центра,
    // но не дальше radius + 3, должны остаться.
    let center = ColumnPos::new(2, 0);
    let unload = loader.unload_radius();
    for key in loader.update(center, &mut world) {
        assert!(dist2(key, center) > loader.unmesh_radius().pow(2));
        assert!(meshed.remove(&key));
    }
    for key in &before {
        let d2 = dist2(*key, center);
        assert_eq!(world.column(*key).is_some(), d2 <= unload * unload);
    }
    assert!(
        before
            .iter()
            .any(|&k| dist2(k, center) > loader.gen_radius().pow(2) && world.column(k).is_some())
    );

    // Далёкий прыжок: всё старое выгружается, загруженное ограничено кругом.
    let far = ColumnPos::new(1000, -1000);
    settle(&mut loader, &mut world, far, &mut meshed);
    assert_circles(&world, &meshed, far, DEFAULT_RADIUS);
}

#[test]
fn meshed_columns_keep_their_neighbors() {
    let mut loader = Loader::new(ColumnPos::ZERO);
    let mut world = World::new();
    let mut meshed: HashSet<ColumnPos> = HashSet::new();
    // Петляющий путь с шагами разной длины, включая прыжки через несколько колонок.
    let mut center = ColumnPos::ZERO;
    for i in 0..400 {
        let step = match i % 7 {
            0 => IVec2::new(3, 1),
            1 | 2 => IVec2::new(1, 0),
            3 => IVec2::new(0, -2),
            4 => IVec2::new(-1, 1),
            _ => IVec2::ZERO,
        };
        if i % 100 < 50 {
            center += step;
        } else {
            center -= step;
        }
        frame(&mut loader, &mut world, center, &mut meshed, 6);
        assert_neighbors_loaded(&world, &meshed);
        assert!(world.len() <= circle_points(loader.unload_radius()));
    }
}

#[test]
fn set_radius_up_and_down_settles_to_new_circles() {
    let center = ColumnPos::new(3, -2);
    let mut loader = Loader::new(center);
    let mut world = World::new();
    let mut meshed = HashSet::new();
    settle(&mut loader, &mut world, center, &mut meshed);
    for radius in [12, 5, 16, 4, 9] {
        loader.set_radius(radius);
        assert_eq!(loader.radius(), radius);
        settle(&mut loader, &mut world, center, &mut meshed);
        assert_circles(&world, &meshed, center, radius);
        assert_eq!(loader.pending(), 0);
    }
}

#[test]
fn sharp_shrink_keeps_neighbors_of_meshed_columns() {
    let mut center = ColumnPos::ZERO;
    let mut loader = Loader::new(center);
    let mut world = World::new();
    let mut meshed = HashSet::new();
    loader.set_radius(MAX_RADIUS);
    settle(&mut loader, &mut world, center, &mut meshed);

    loader.set_radius(MIN_RADIUS);
    for i in 0..60 {
        if i % 3 == 0 {
            center += IVec2::ONE;
        }
        for key in loader.update(center, &mut world) {
            assert!(meshed.remove(&key));
        }
        assert_neighbors_loaded(&world, &meshed);
        for _ in 0..5 {
            if let Progress::Mesh(key) = loader.advance(&mut world, air) {
                meshed.insert(key);
            }
            assert_neighbors_loaded(&world, &meshed);
        }
    }
    // После движения на краю остаётся запас гистерезиса, но не больше него.
    settle(&mut loader, &mut world, center, &mut meshed);
    assert_eq!(loader.pending(), 0);
    let r = MIN_RADIUS;
    assert!(meshed.iter().all(|&k| dist2(k, center) <= (r + 1).pow(2)));
    assert!(world.keys().all(|k| dist2(k, center) <= (r + 3).pow(2)));
}

#[test]
fn radius_is_clamped() {
    let mut loader = Loader::new(ColumnPos::ZERO);
    assert_eq!(loader.radius(), DEFAULT_RADIUS);
    loader.set_radius(1);
    assert_eq!(loader.radius(), MIN_RADIUS);
    loader.set_radius(100);
    assert_eq!(loader.radius(), MAX_RADIUS);
}

#[test]
fn warm_up_meshes_small_circle() {
    let center = ColumnPos::new(7, 7);
    let mut loader = Loader::new(center);
    let mut world = World::new();
    let mesh = loader.warm_up(&mut world, air, 2);
    assert_eq!(mesh.len(), circle_points(2));
    assert_eq!(world.len(), circle_points(4));
    assert!(mesh.iter().all(|&k| dist2(k, center) <= 4));
}

#[test]
fn edited_column_comes_back_without_generator() {
    use glam::IVec3;

    use world::block::Block;

    let mut loader = Loader::new(ColumnPos::ZERO);
    let mut world = World::new();
    let mut meshed = HashSet::new();
    settle(&mut loader, &mut world, ColumnPos::ZERO, &mut meshed);
    let pos = IVec3::new(3, 50, -5);
    let key = ColumnPos::new(0, -1);
    assert!(world.set_block(pos, Block::Stone));

    let far = ColumnPos::new(100, 0);
    settle(&mut loader, &mut world, far, &mut meshed);
    assert!(world.column(key).is_none());

    // Обратно: генератор не должен трогать изменённую колонку.
    for key in loader.update(ColumnPos::ZERO, &mut world) {
        meshed.remove(&key);
    }
    loop {
        let progress = loader.advance(&mut world, |k| {
            assert_ne!(k, key, "edited column regenerated");
            Column::new()
        });
        match progress {
            Progress::Done => break,
            Progress::Mesh(k) => assert!(meshed.insert(k)),
            Progress::Generated(_) => {}
        }
    }
    assert_eq!(world.block(pos), Block::Stone);

    // И после второй выгрузки правка снова сохраняется.
    settle(&mut loader, &mut world, far, &mut meshed);
    assert!(world.is_edited(key));
    settle(&mut loader, &mut world, ColumnPos::ZERO, &mut meshed);
    assert_eq!(world.block(pos), Block::Stone);
}

#[test]
fn import_into_unloaded_goes_to_loader() {
    let key = ColumnPos::new(0, 1);
    let pos = glam::IVec3::new(5, 60, 20);
    let mut world = World::new();
    // Правка до импорта: импорт перекрывает и её признак «не сохранено».
    world.insert(key, Column::new());
    world.set_block(pos, world::block::Block::Dirt);
    world.remove(key);
    let mut saved = Column::new();
    saved.set(glam::IVec3::new(5, 60, 4), world::block::Block::Stone);
    world.import(key, saved);
    assert!(world.drain_dirty().is_empty());
    assert!(world.column(key).is_none());
    assert!(world.is_edited(key));

    let mut loader = Loader::new(ColumnPos::ZERO);
    loader.update(ColumnPos::ZERO, &mut world);
    while loader.advance(&mut world, |k| {
        assert_ne!(k, key, "imported column regenerated");
        Column::new()
    }) != Progress::Done
    {}
    assert_eq!(world.block(pos), world::block::Block::Stone);
    assert!(world.drain_dirty().is_empty());
}
