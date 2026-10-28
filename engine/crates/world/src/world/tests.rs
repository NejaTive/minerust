use super::*;

#[test]
fn negative_coords_land_in_right_column() {
    let mut column = Column::new();
    column.set(IVec3::new(15, 10, 0), Block::Dirt);
    let mut world = World::new();
    world.insert(ColumnPos::new(-1, 2), column);

    assert_eq!(world.block(IVec3::new(-1, 10, 32)), Block::Dirt);
    assert_eq!(world.block(IVec3::new(-2, 10, 32)), Block::Air);
    assert_eq!(
        ColumnPos::of_block(IVec3::new(-1, 0, -16)),
        ColumnPos::new(-1, -1)
    );
    assert_eq!(
        ColumnPos::of_block(IVec3::new(-17, 0, 15)),
        ColumnPos::new(-2, 0)
    );
}

#[test]
fn placement_replaces_grass_and_needs_support() {
    let mut world = World::new();
    world.insert(ColumnPos::ZERO, Column::new());
    let floor = IVec3::new(3, 10, 3);
    let above = floor + IVec3::Y;
    world.set_block(floor, Block::Stone);
    world.set_block(above, Block::TallGrass);
    assert!(world.can_place(above, Block::Torch));
    assert!(world.can_place(above, Block::Stone));
    world.set_block(above, Block::Dirt);
    assert!(
        !world.can_place(above, Block::Stone),
        "cubes are not replaced"
    );
    // Без твёрдого блока снизу цветок и факел не ставятся, куб — ставится.
    let air = floor + 3 * IVec3::Y;
    assert!(!world.can_place(air, Block::Poppy) && !world.can_place(air, Block::Torch));
    assert!(world.can_place(air, Block::Stone));
    // Цветок не опора для другого цветка.
    world.set_block(air, Block::Poppy);
    assert!(!world.can_place(air + IVec3::Y, Block::Dandelion));
    world.set_block(floor + 5 * IVec3::Y, Block::Water);
    assert!(world.can_place(floor + 5 * IVec3::Y, Block::Stone));
    assert!(
        !world.can_place(floor + 5 * IVec3::Y, Block::Torch),
        "water is no support"
    );
}

#[test]
fn breaking_support_removes_plant_above() {
    let mut world = World::new();
    world.insert(ColumnPos::ZERO, Column::new());
    let floor = IVec3::new(3, 10, 3);
    world.set_block(floor, Block::Grass);
    world.set_block(floor + IVec3::Y, Block::Poppy);
    world.set_block(floor + 2 * IVec3::Y, Block::Stone);
    let changed = world.set_block_supported(floor, Block::Air);
    assert_eq!(changed, vec![floor, floor + IVec3::Y]);
    assert_eq!(world.block(floor + IVec3::Y), Block::Air);
    assert_eq!(world.block(floor + 2 * IVec3::Y), Block::Stone);
}

#[test]
fn replacing_support_with_solid_keeps_plant() {
    let mut world = World::new();
    world.insert(ColumnPos::ZERO, Column::new());
    let floor = IVec3::new(3, 10, 3);
    world.set_block(floor, Block::Grass);
    world.set_block(floor + IVec3::Y, Block::Torch);
    assert_eq!(world.set_block_supported(floor, Block::Stone), vec![floor]);
    assert_eq!(world.block(floor + IVec3::Y), Block::Torch);
    // Блок, не требующий опоры, над воздухом остаётся.
    world.set_block(floor + 3 * IVec3::Y, Block::Stone);
    assert_eq!(
        world.set_block_supported(floor + 2 * IVec3::Y, Block::Air),
        vec![floor + 2 * IVec3::Y]
    );
    assert_eq!(world.block(floor + 3 * IVec3::Y), Block::Stone);
}

#[test]
fn support_edit_in_unloaded_column_changes_nothing() {
    let mut world = World::new();
    assert!(
        world
            .set_block_supported(IVec3::new(500, 10, 0), Block::Air)
            .is_empty()
    );
}

#[test]
fn outside_height_and_unloaded() {
    let mut column = Column::new();
    column.set(IVec3::new(0, WORLD_HEIGHT - 1, 0), Block::Stone);
    let mut world = World::new();
    world.insert(ColumnPos::ZERO, column);

    assert_eq!(world.block(IVec3::new(0, -1, 0)), Block::Stone);
    assert_eq!(world.block(IVec3::new(500, -1, 500)), Block::Stone);
    assert_eq!(
        world.block(IVec3::new(0, WORLD_HEIGHT - 1, 0)),
        Block::Stone
    );
    assert_eq!(world.block(IVec3::new(0, WORLD_HEIGHT, 0)), Block::Air);
    assert_eq!(world.block(IVec3::new(16, 0, 0)), Block::Air);
}

#[test]
fn insert_remove_len() {
    let mut world = World::new();
    assert!(world.is_empty());
    world.insert(ColumnPos::new(3, -4), Column::new());
    assert_eq!(world.len(), 1);
    assert!(world.column(ColumnPos::new(3, -4)).is_some());
    assert!(world.remove(ColumnPos::new(3, -4)).is_some());
    assert!(world.column(ColumnPos::new(3, -4)).is_none());
    assert_eq!(world.len(), 0);
}

#[test]
fn set_block_rejects_unloaded_and_out_of_height() {
    let mut world = World::new();
    world.insert(ColumnPos::ZERO, Column::new());

    assert!(!world.set_block(IVec3::new(16, 5, 0), Block::Stone));
    assert!(!world.set_block(IVec3::new(0, -1, 0), Block::Air));
    assert!(!world.set_block(IVec3::new(0, WORLD_HEIGHT, 0), Block::Stone));
    assert!(!world.is_edited(ColumnPos::ZERO));
    assert!(world.column(ColumnPos::new(1, 0)).is_none());

    assert!(world.set_block(IVec3::new(15, WORLD_HEIGHT - 1, 0), Block::Dirt));
    assert_eq!(
        world.block(IVec3::new(15, WORLD_HEIGHT - 1, 0)),
        Block::Dirt
    );
    assert!(world.is_edited(ColumnPos::ZERO));
}

#[test]
fn edited_column_survives_unload() {
    let key = ColumnPos::new(-2, 3);
    let pos = IVec3::new(-20, 40, 50);
    let mut world = World::new();
    world.insert(key, Column::new());
    assert!(world.set_block(pos, Block::Stone));

    assert!(world.remove(key).is_some());
    assert_eq!(world.block(pos), Block::Air);
    assert!(world.is_edited(key));

    let column = world.take_edited(key).expect("edited column is stashed");
    world.insert(key, column);
    assert_eq!(world.block(pos), Block::Stone);
    assert!(world.take_edited(key).is_none());

    // Вернувшаяся колонка по-прежнему изменённая: вторая выгрузка снова её отложит.
    world.remove(key);
    assert!(world.take_edited(key).is_some());
}

#[test]
fn untouched_column_is_not_stashed() {
    let mut world = World::new();
    world.insert(ColumnPos::new(1, 1), Column::new());
    world.insert(ColumnPos::ZERO, Column::new());
    world.set_block(IVec3::new(1, 1, 1), Block::Dirt);

    world.remove(ColumnPos::new(1, 1));
    assert!(world.take_edited(ColumnPos::new(1, 1)).is_none());
    assert!(!world.is_edited(ColumnPos::new(1, 1)));
}

#[test]
fn drain_dirty_reports_each_edit_once() {
    let key = ColumnPos::new(1, -1);
    let mut world = World::new();
    world.insert(key, Column::new());
    world.insert(ColumnPos::ZERO, Column::new());
    assert!(world.drain_dirty().is_empty());

    world.set_block(IVec3::new(16, 3, -16), Block::Dirt);
    world.set_block(IVec3::new(17, 3, -16), Block::Dirt);
    assert_eq!(world.drain_dirty(), vec![key]);
    assert!(world.drain_dirty().is_empty());

    world.set_block(IVec3::new(16, 4, -16), Block::Stone);
    assert_eq!(world.drain_dirty(), vec![key]);
    // Загруженная, но не тронутая колонка — не изменённая.
    assert!(world.edited_column(ColumnPos::ZERO).is_none());
}

#[test]
fn stashed_column_stays_dirty_and_edited() {
    let key = ColumnPos::new(-3, 0);
    let pos = IVec3::new(-48, 10, 0);
    let mut world = World::new();
    world.insert(key, Column::new());
    world.set_block(pos, Block::Stone);
    world.remove(key);

    assert_eq!(world.drain_dirty(), vec![key]);
    let column = world.edited_column(key).expect("stashed column is edited");
    assert_eq!(column.get(IVec3::new(0, 10, 0)), Block::Stone);
}

#[test]
fn import_replaces_loaded_column() {
    let key = ColumnPos::new(2, 2);
    let mut world = World::new();
    world.insert(key, Column::new());
    let mut saved = Column::new();
    saved.set(IVec3::new(1, 2, 3), Block::Grass);
    world.import(key, saved);

    assert_eq!(world.block(IVec3::new(33, 2, 35)), Block::Grass);
    assert!(world.is_edited(key));
    assert!(world.edited_column(key).is_some());
    assert!(world.drain_dirty().is_empty());

    // Импортированная колонка снова отложится при выгрузке.
    world.remove(key);
    assert!(world.edited_column(key).is_some());
}

fn sorted(mut keys: Vec<SectionPos>) -> Vec<SectionPos> {
    keys.sort();
    keys
}

#[test]
fn affected_sections_in_middle_is_one() {
    assert_eq!(
        affected_sections(IVec3::new(5, 40, -7)),
        vec![SectionPos::new(0, 2, -1)]
    );
}

#[test]
fn affected_sections_on_column_border() {
    // x = 15 — край колонки 0, сосед справа — колонка 1.
    assert_eq!(
        sorted(affected_sections(IVec3::new(15, 40, 5))),
        sorted(vec![SectionPos::new(0, 2, 0), SectionPos::new(1, 2, 0)])
    );
    // x = -16 — левый край колонки -1.
    assert_eq!(
        sorted(affected_sections(IVec3::new(-16, 40, 5))),
        sorted(vec![SectionPos::new(-1, 2, 0), SectionPos::new(-2, 2, 0)])
    );
}

#[test]
fn affected_sections_on_edge() {
    // x = 15 и y = 32: ребро между колонками 0/1 и секциями 1/2.
    assert_eq!(
        sorted(affected_sections(IVec3::new(15, 32, 5))),
        sorted(vec![
            SectionPos::new(0, 2, 0),
            SectionPos::new(1, 2, 0),
            SectionPos::new(0, 1, 0),
            SectionPos::new(1, 1, 0),
        ])
    );
}

#[test]
fn affected_sections_in_corner() {
    let mut expected = Vec::new();
    for y in 2..=3 {
        for z in -1..=0 {
            for x in 0..=1 {
                expected.push(SectionPos::new(x, y, z));
            }
        }
    }
    assert_eq!(
        sorted(affected_sections(IVec3::new(16, 47, -1))),
        sorted(expected)
    );
}

#[test]
fn affected_sections_stay_in_height() {
    assert_eq!(
        affected_sections(IVec3::new(5, 0, 5)),
        vec![SectionPos::new(0, 0, 0)]
    );
    assert_eq!(
        affected_sections(IVec3::new(5, WORLD_HEIGHT - 1, 5)),
        vec![SectionPos::new(0, COLUMN_SECTIONS - 1, 0)]
    );
    assert_eq!(
        sorted(affected_sections(IVec3::new(5, 16, 5))),
        sorted(vec![SectionPos::new(0, 1, 0), SectionPos::new(0, 0, 0)])
    );
    // Углы колонки на дне и на верху мира: 4 секции, все в 0..COLUMN_SECTIONS.
    for (pos, sy) in [
        (IVec3::new(0, 0, 0), 0),
        (IVec3::new(15, WORLD_HEIGHT - 1, 15), COLUMN_SECTIONS - 1),
    ] {
        let keys = affected_sections(pos);
        assert_eq!(keys.len(), 4, "{pos}");
        assert!(keys.iter().all(|k| k.y() == sy), "{pos}: {keys:?}");
    }
}
