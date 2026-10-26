use std::collections::{HashMap, HashSet};

use glam::{IVec2, IVec3};

use crate::block::Block;
use crate::column::{COLUMN_SECTIONS, Column, WORLD_HEIGHT};
use crate::section::SECTION_SIZE;

/// Загруженные колонки; ключ — координаты колонки (мировые x, z / 16, через div_euclid).
///
/// Колонки с правками игрока при выгрузке не пропадают, а ждут в `stashed`, пока
/// загрузчик не вернёт их через [`World::take_edited`]. Правки целиком — это
/// изменённые загруженные колонки плюс `stashed`. Сохранение пишет не все правки, а
/// только колонки, изменённые с прошлого раза: их отдаёт [`World::drain_dirty`].
pub struct World {
    columns: HashMap<IVec2, Column>,
    /// Ключи загруженных колонок, в которых что-то меняли через [`World::set_block`].
    edited: HashSet<IVec2>,
    /// Выгруженные изменённые колонки. Без ограничения: их немного, и терять нельзя.
    stashed: HashMap<IVec2, Column>,
    /// Колонки, изменённые после прошлого [`World::drain_dirty`], загруженные или
    /// отложенные. Выгрузка признак не снимает: колонку ещё надо сохранить.
    dirty: HashSet<IVec2>,
}

impl World {
    pub fn new() -> Self {
        Self {
            columns: HashMap::new(),
            edited: HashSet::new(),
            stashed: HashMap::new(),
            dirty: HashSet::new(),
        }
    }

    /// Блок по мировым координатам. y < 0 — твёрдый (`Stone`: дно мира не мешируется),
    /// y ≥ WORLD_HEIGHT — `Air`, незагруженная колонка — `Air`.
    pub fn block(&self, pos: IVec3) -> Block {
        if pos.y < 0 {
            return Block::Stone;
        }
        if pos.y >= WORLD_HEIGHT {
            return Block::Air;
        }
        let (key, local) = split(pos);
        self.columns
            .get(&key)
            .map_or(Block::Air, |column| column.get(local))
    }

    /// Ставит блок по мировым координатам. `false` (и ничего не меняет), если y вне
    /// 0..WORLD_HEIGHT или колонка не загружена. Колонка помечается изменённой и
    /// несохранённой.
    pub fn set_block(&mut self, pos: IVec3, block: Block) -> bool {
        if !(0..WORLD_HEIGHT).contains(&pos.y) {
            return false;
        }
        let (key, local) = split(pos);
        let Some(column) = self.columns.get_mut(&key) else {
            return false;
        };
        column.set(local, block);
        self.edited.insert(key);
        self.dirty.insert(key);
        true
    }

    pub fn column(&self, key: IVec2) -> Option<&Column> {
        self.columns.get(&key)
    }

    pub fn insert(&mut self, key: IVec2, column: Column) {
        self.columns.insert(key, column);
    }

    /// Выгружает колонку. Изменённая не пропадает: её копия уходит в отложенные
    /// правки, откуда её заберёт [`World::take_edited`].
    pub fn remove(&mut self, key: IVec2) -> Option<Column> {
        let column = self.columns.remove(&key)?;
        if self.edited.remove(&key) {
            self.stashed.insert(key, column.clone());
        }
        Some(column)
    }

    /// Отложенная изменённая колонка, если есть (забирает её из отложенных). Колонка
    /// по-прежнему считается изменённой: вызывающий должен вернуть её через
    /// [`World::insert`], и следующая выгрузка снова её отложит.
    pub fn take_edited(&mut self, key: IVec2) -> Option<Column> {
        let column = self.stashed.remove(&key)?;
        self.edited.insert(key);
        Some(column)
    }

    /// Изменена ли колонка (загруженная или отложенная).
    pub fn is_edited(&self, key: IVec2) -> bool {
        self.edited.contains(&key) || self.stashed.contains_key(&key)
    }

    /// Ключи колонок, изменённых после прошлого вызова (загруженных и отложенных), в
    /// произвольном порядке; сбрасывает признак.
    pub fn drain_dirty(&mut self) -> Vec<IVec2> {
        self.dirty.drain().collect()
    }

    /// Изменённая колонка — загруженная или отложенная.
    pub fn edited_column(&self, key: IVec2) -> Option<&Column> {
        if self.edited.contains(&key) {
            self.columns.get(&key)
        } else {
            self.stashed.get(&key)
        }
    }

    /// Сохранённая колонка из хранилища: если колонка загружена — заменяет её, иначе
    /// кладёт в отложенные, и загрузчик возьмёт её вместо генерации. В обоих случаях
    /// колонка считается изменённой (её снова надо сохранять при выгрузке), но не
    /// несохранённой: в хранилище она уже такая.
    pub fn import(&mut self, key: IVec2, column: Column) {
        if let Some(loaded) = self.columns.get_mut(&key) {
            *loaded = column;
            self.edited.insert(key);
        } else {
            self.stashed.insert(key, column);
        }
        self.dirty.remove(&key);
    }

    /// Ключи загруженных колонок, в произвольном порядке.
    pub fn keys(&self) -> impl Iterator<Item = IVec2> + '_ {
        self.columns.keys().copied()
    }

    pub fn len(&self) -> usize {
        self.columns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

/// Колонка, в которой лежит мировая точка (x, z) в блоках.
pub fn column_key(x: i32, z: i32) -> IVec2 {
    IVec2::new(x, z).div_euclid(IVec2::splat(SECTION_SIZE))
}

/// Ключи секций `(x колонки, номер секции 0..8, z колонки)`, которые надо перестроить
/// после изменения блока `pos`: его секция и все соседние, к которым он примыкает, —
/// по граням, рёбрам и углам, до 8. Диагональные нужны из-за AO: вершины соседней
/// секции затеняются блоками по диагонали от них. Секции вне 0..COLUMN_SECTIONS по
/// вертикали не возвращаются.
pub fn affected_sections(pos: IVec3) -> Vec<IVec3> {
    let size = IVec3::splat(SECTION_SIZE);
    let own = pos.div_euclid(size);
    let local = pos.rem_euclid(size);
    // Сдвиги по оси: своя секция и, если блок на краю, соседняя с этой стороны.
    let span = |axis: usize| match local[axis] {
        0 => -1..=0,
        l if l == SECTION_SIZE - 1 => 0..=1,
        _ => 0..=0,
    };
    let mut keys = Vec::new();
    for dy in span(1) {
        for dz in span(2) {
            for dx in span(0) {
                keys.push(own + IVec3::new(dx, dy, dz));
            }
        }
    }
    keys.retain(|key| (0..COLUMN_SECTIONS).contains(&key.y));
    keys
}

/// Мировые координаты → (ключ колонки, локальные в ней). `div_euclid`, а не `/`,
/// чтобы x = -1 попадал в колонку -1, а не 0.
fn split(pos: IVec3) -> (IVec2, IVec3) {
    let key = column_key(pos.x, pos.z);
    let local = IVec3::new(
        pos.x.rem_euclid(SECTION_SIZE),
        pos.y,
        pos.z.rem_euclid(SECTION_SIZE),
    );
    (key, local)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_coords_land_in_right_column() {
        let mut column = Column::new();
        column.set(IVec3::new(15, 10, 0), Block::Dirt);
        let mut world = World::new();
        world.insert(IVec2::new(-1, 2), column);

        assert_eq!(world.block(IVec3::new(-1, 10, 32)), Block::Dirt);
        assert_eq!(world.block(IVec3::new(-2, 10, 32)), Block::Air);
        assert_eq!(column_key(-1, -16), IVec2::new(-1, -1));
        assert_eq!(column_key(-17, 15), IVec2::new(-2, 0));
    }

    #[test]
    fn outside_height_and_unloaded() {
        let mut column = Column::new();
        column.set(IVec3::new(0, WORLD_HEIGHT - 1, 0), Block::Stone);
        let mut world = World::new();
        world.insert(IVec2::ZERO, column);

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
        world.insert(IVec2::new(3, -4), Column::new());
        assert_eq!(world.len(), 1);
        assert!(world.column(IVec2::new(3, -4)).is_some());
        assert!(world.remove(IVec2::new(3, -4)).is_some());
        assert!(world.column(IVec2::new(3, -4)).is_none());
        assert_eq!(world.len(), 0);
    }

    #[test]
    fn set_block_rejects_unloaded_and_out_of_height() {
        let mut world = World::new();
        world.insert(IVec2::ZERO, Column::new());

        assert!(!world.set_block(IVec3::new(16, 5, 0), Block::Stone));
        assert!(!world.set_block(IVec3::new(0, -1, 0), Block::Air));
        assert!(!world.set_block(IVec3::new(0, WORLD_HEIGHT, 0), Block::Stone));
        assert!(!world.is_edited(IVec2::ZERO));
        assert!(world.column(IVec2::X).is_none());

        assert!(world.set_block(IVec3::new(15, WORLD_HEIGHT - 1, 0), Block::Dirt));
        assert_eq!(
            world.block(IVec3::new(15, WORLD_HEIGHT - 1, 0)),
            Block::Dirt
        );
        assert!(world.is_edited(IVec2::ZERO));
    }

    #[test]
    fn edited_column_survives_unload() {
        let key = IVec2::new(-2, 3);
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
        world.insert(IVec2::ONE, Column::new());
        world.insert(IVec2::ZERO, Column::new());
        world.set_block(IVec3::new(1, 1, 1), Block::Dirt);

        world.remove(IVec2::ONE);
        assert!(world.take_edited(IVec2::ONE).is_none());
        assert!(!world.is_edited(IVec2::ONE));
    }

    #[test]
    fn drain_dirty_reports_each_edit_once() {
        let key = IVec2::new(1, -1);
        let mut world = World::new();
        world.insert(key, Column::new());
        world.insert(IVec2::ZERO, Column::new());
        assert!(world.drain_dirty().is_empty());

        world.set_block(IVec3::new(16, 3, -16), Block::Dirt);
        world.set_block(IVec3::new(17, 3, -16), Block::Dirt);
        assert_eq!(world.drain_dirty(), vec![key]);
        assert!(world.drain_dirty().is_empty());

        world.set_block(IVec3::new(16, 4, -16), Block::Stone);
        assert_eq!(world.drain_dirty(), vec![key]);
        // Загруженная, но не тронутая колонка — не изменённая.
        assert!(world.edited_column(IVec2::ZERO).is_none());
    }

    #[test]
    fn stashed_column_stays_dirty_and_edited() {
        let key = IVec2::new(-3, 0);
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
        let key = IVec2::new(2, 2);
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

    #[test]
    fn import_into_unloaded_goes_to_loader() {
        use crate::loader::{Loader, Progress};

        let key = IVec2::new(0, 1);
        let pos = IVec3::new(5, 60, 20);
        let mut world = World::new();
        // Правка до импорта: импорт перекрывает и её признак «не сохранено».
        world.insert(key, Column::new());
        world.set_block(pos, Block::Dirt);
        world.remove(key);
        let mut saved = Column::new();
        saved.set(IVec3::new(5, 60, 4), Block::Stone);
        world.import(key, saved);
        assert!(world.drain_dirty().is_empty());
        assert!(world.column(key).is_none());
        assert!(world.is_edited(key));

        let mut loader = Loader::new(IVec2::ZERO);
        loader.update(IVec2::ZERO, &mut world);
        while loader.advance(&mut world, |k| {
            assert_ne!(k, key, "imported column regenerated");
            Column::new()
        }) != Progress::Done
        {}
        assert_eq!(world.block(pos), Block::Stone);
        assert!(world.drain_dirty().is_empty());
    }

    fn sorted(mut keys: Vec<IVec3>) -> Vec<IVec3> {
        keys.sort_by_key(|k| k.to_array());
        keys
    }

    #[test]
    fn affected_sections_in_middle_is_one() {
        assert_eq!(
            affected_sections(IVec3::new(5, 40, -7)),
            vec![IVec3::new(0, 2, -1)]
        );
    }

    #[test]
    fn affected_sections_on_column_border() {
        // x = 15 — край колонки 0, сосед справа — колонка 1.
        assert_eq!(
            sorted(affected_sections(IVec3::new(15, 40, 5))),
            sorted(vec![IVec3::new(0, 2, 0), IVec3::new(1, 2, 0)])
        );
        // x = -16 — левый край колонки -1.
        assert_eq!(
            sorted(affected_sections(IVec3::new(-16, 40, 5))),
            sorted(vec![IVec3::new(-1, 2, 0), IVec3::new(-2, 2, 0)])
        );
    }

    #[test]
    fn affected_sections_on_edge() {
        // x = 15 и y = 32: ребро между колонками 0/1 и секциями 1/2.
        assert_eq!(
            sorted(affected_sections(IVec3::new(15, 32, 5))),
            sorted(vec![
                IVec3::new(0, 2, 0),
                IVec3::new(1, 2, 0),
                IVec3::new(0, 1, 0),
                IVec3::new(1, 1, 0),
            ])
        );
    }

    #[test]
    fn affected_sections_in_corner() {
        let mut expected = Vec::new();
        for y in 2..=3 {
            for z in -1..=0 {
                for x in 0..=1 {
                    expected.push(IVec3::new(x, y, z));
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
            vec![IVec3::new(0, 0, 0)]
        );
        assert_eq!(
            affected_sections(IVec3::new(5, WORLD_HEIGHT - 1, 5)),
            vec![IVec3::new(0, COLUMN_SECTIONS - 1, 0)]
        );
        assert_eq!(
            sorted(affected_sections(IVec3::new(5, 16, 5))),
            sorted(vec![IVec3::new(0, 1, 0), IVec3::new(0, 0, 0)])
        );
        // Углы колонки на дне и на верху мира: 4 секции, все в 0..COLUMN_SECTIONS.
        for (pos, sy) in [
            (IVec3::new(0, 0, 0), 0),
            (IVec3::new(15, WORLD_HEIGHT - 1, 15), COLUMN_SECTIONS - 1),
        ] {
            let keys = affected_sections(pos);
            assert_eq!(keys.len(), 4, "{pos}");
            assert!(keys.iter().all(|k| k.y == sy), "{pos}: {keys:?}");
        }
    }
}
