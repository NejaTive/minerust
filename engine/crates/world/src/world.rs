use std::collections::{HashMap, HashSet};

use glam::{IVec2, IVec3};

use crate::pos::{ColumnPos, SectionPos};

use crate::block::Block;
use crate::column::{COLUMN_SECTIONS, Column, WORLD_HEIGHT};
use crate::section::SECTION_SIZE;

/// Загруженные колонки по ключу [`ColumnPos`].
///
/// Колонки с правками игрока при выгрузке не пропадают, а ждут в `stashed`, пока
/// загрузчик не вернёт их через [`World::take_edited`]. Правки целиком — это
/// изменённые загруженные колонки плюс `stashed`. Сохранение пишет не все правки, а
/// только колонки, изменённые с прошлого раза: их отдаёт [`World::drain_dirty`].
pub struct World {
    columns: HashMap<ColumnPos, Column>,
    /// Ключи загруженных колонок, в которых что-то меняли через [`World::set_block`].
    edited: HashSet<ColumnPos>,
    /// Выгруженные изменённые колонки. Без ограничения: их немного, и терять нельзя.
    stashed: HashMap<ColumnPos, Column>,
    /// Колонки, изменённые после прошлого [`World::drain_dirty`], загруженные или
    /// отложенные. Выгрузка признак не снимает: колонку ещё надо сохранить.
    dirty: HashSet<ColumnPos>,
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

    /// Колонка `key` и восемь соседей: индекс `dx + 1 + 3·(dz + 1)`, центр — 4;
    /// незагруженные — `None`. Мешеру: один запрос на колонку вместо хэша на блок.
    pub fn neighborhood(&self, key: ColumnPos) -> [Option<&Column>; 9] {
        std::array::from_fn(|i| self.column(key + IVec2::new(i as i32 % 3 - 1, i as i32 / 3 - 1)))
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

    /// Можно ли поставить `block` в клетку `pos`: там воздух, вода или трава (она
    /// заменяется), а растению и факелу снизу нужен твёрдый блок. Игрока не учитывает.
    pub fn can_place(&self, pos: IVec3, block: Block) -> bool {
        self.block(pos).is_replaceable()
            && (!block.needs_support() || self.block(pos - IVec3::Y).is_solid())
    }

    /// Правка с опорой: ставит `block` и, если клетка перестала быть твёрдой, убирает
    /// стоящие выше блоки, которым нужна опора (трава, цветы, факел). Возвращает
    /// изменённые клетки — первой `pos`, затем снятые; пусто, если правка не прошла
    /// (`set_block`). Одно место для ломания и установки: движок обновляет свет и меши
    /// по каждой клетке списка.
    pub fn set_block_supported(&mut self, pos: IVec3, block: Block) -> Vec<IVec3> {
        if !self.set_block(pos, block) {
            return Vec::new();
        }
        let mut changed = vec![pos];
        let mut cell = pos;
        while !self.block(cell).is_solid() {
            cell += IVec3::Y;
            if !self.block(cell).needs_support() || !self.set_block(cell, Block::Air) {
                break;
            }
            changed.push(cell);
        }
        changed
    }

    pub fn column(&self, key: ColumnPos) -> Option<&Column> {
        self.columns.get(&key)
    }

    pub fn insert(&mut self, key: ColumnPos, column: Column) {
        self.columns.insert(key, column);
    }

    /// Выгружает колонку. Изменённая не пропадает: её копия уходит в отложенные
    /// правки, откуда её заберёт [`World::take_edited`].
    pub fn remove(&mut self, key: ColumnPos) -> Option<Column> {
        let column = self.columns.remove(&key)?;
        if self.edited.remove(&key) {
            self.stashed.insert(key, column.clone());
        }
        Some(column)
    }

    /// Отложенная изменённая колонка, если есть (забирает её из отложенных). Колонка
    /// по-прежнему считается изменённой: вызывающий должен вернуть её через
    /// [`World::insert`], и следующая выгрузка снова её отложит.
    pub fn take_edited(&mut self, key: ColumnPos) -> Option<Column> {
        let column = self.stashed.remove(&key)?;
        self.edited.insert(key);
        Some(column)
    }

    /// Изменена ли колонка (загруженная или отложенная).
    pub fn is_edited(&self, key: ColumnPos) -> bool {
        self.edited.contains(&key) || self.stashed.contains_key(&key)
    }

    /// Ключи колонок, изменённых после прошлого вызова (загруженных и отложенных), в
    /// произвольном порядке; сбрасывает признак.
    pub fn drain_dirty(&mut self) -> Vec<ColumnPos> {
        self.dirty.drain().collect()
    }

    /// Изменённая колонка — загруженная или отложенная.
    pub fn edited_column(&self, key: ColumnPos) -> Option<&Column> {
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
    pub fn import(&mut self, key: ColumnPos, column: Column) {
        if let Some(loaded) = self.columns.get_mut(&key) {
            *loaded = column;
            self.edited.insert(key);
        } else {
            self.stashed.insert(key, column);
        }
        self.dirty.remove(&key);
    }

    /// Ключи загруженных колонок, в произвольном порядке.
    pub fn keys(&self) -> impl Iterator<Item = ColumnPos> + '_ {
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

/// Ключи секций, которые надо перестроить
/// после изменения блока `pos`: его секция и все соседние, к которым он примыкает, —
/// по граням, рёбрам и углам, до 8. Диагональные нужны из-за AO: вершины соседней
/// секции затеняются блоками по диагонали от них. Секции вне 0..COLUMN_SECTIONS по
/// вертикали не возвращаются.
pub fn affected_sections(pos: IVec3) -> Vec<SectionPos> {
    let own = SectionPos::of_block(pos);
    let local = pos.rem_euclid(IVec3::splat(SECTION_SIZE));
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
    keys.retain(|key| (0..COLUMN_SECTIONS).contains(&key.y()));
    keys
}

/// Мировые координаты → (ключ колонки, локальные в ней). `div_euclid`, а не `/`,
/// чтобы x = -1 попадал в колонку -1, а не 0.
fn split(pos: IVec3) -> (ColumnPos, IVec3) {
    let key = ColumnPos::of_block(pos);
    let local = IVec3::new(
        pos.x.rem_euclid(SECTION_SIZE),
        pos.y,
        pos.z.rem_euclid(SECTION_SIZE),
    );
    (key, local)
}

#[cfg(test)]
mod tests;
