use std::collections::HashMap;

use glam::{IVec2, IVec3};

use crate::block::Block;
use crate::column::{Column, WORLD_HEIGHT};
use crate::section::SECTION_SIZE;

/// Загруженные колонки; ключ — координаты колонки (мировые x, z / 16, через div_euclid).
pub struct World {
    columns: HashMap<IVec2, Column>,
}

impl World {
    pub fn new() -> Self {
        Self {
            columns: HashMap::new(),
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

    pub fn column(&self, key: IVec2) -> Option<&Column> {
        self.columns.get(&key)
    }

    pub fn insert(&mut self, key: IVec2, column: Column) {
        self.columns.insert(key, column);
    }

    pub fn remove(&mut self, key: IVec2) -> Option<Column> {
        self.columns.remove(&key)
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
}
