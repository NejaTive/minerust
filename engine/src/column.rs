use glam::IVec3;

use crate::block::Block;
use crate::section::{SECTION_SIZE, Section};

pub const COLUMN_SECTIONS: i32 = 8;
pub const WORLD_HEIGHT: i32 = COLUMN_SECTIONS * SECTION_SIZE;

/// Колонка 16×128×16: секции снизу вверх, `sections[0]` — y 0..16.
pub struct Column {
    sections: [Section; COLUMN_SECTIONS as usize],
}

impl Column {
    /// Колонка целиком из воздуха.
    pub fn new() -> Self {
        Self {
            sections: std::array::from_fn(|_| Section::new()),
        }
    }

    /// Лежат ли локальные координаты внутри колонки: x, z в 0..16, y в 0..128.
    pub fn contains(local: IVec3) -> bool {
        local.cmpge(IVec3::ZERO).all()
            && local
                .cmplt(IVec3::new(SECTION_SIZE, WORLD_HEIGHT, SECTION_SIZE))
                .all()
    }

    /// Секция по номеру снизу вверх, 0..8; иначе panic.
    pub fn section(&self, sy: i32) -> &Section {
        &self.sections[sy as usize]
    }

    /// Блок по локальным координатам колонки; за пределами колонки — panic.
    pub fn get(&self, local: IVec3) -> Block {
        let (sy, in_section) = Self::split(local);
        self.sections[sy].get(in_section)
    }

    /// Ставит блок по локальным координатам колонки; за пределами колонки — panic.
    pub fn set(&mut self, local: IVec3, block: Block) {
        let (sy, in_section) = Self::split(local);
        self.sections[sy].set(in_section, block);
    }

    /// Локальные координаты колонки → (номер секции, координаты в ней).
    fn split(local: IVec3) -> (usize, IVec3) {
        // Без проверки тоже был бы panic, но из индекса массива или секции, с чужими координатами.
        assert!(Self::contains(local), "local coords out of column: {local}");
        let sy = local.y / SECTION_SIZE;
        (
            sy as usize,
            IVec3::new(local.x, local.y - sy * SECTION_SIZE, local.z),
        )
    }
}

impl Default for Column {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_then_get_lands_in_right_section() {
        let mut column = Column::new();
        let pos = IVec3::new(3, 37, 15);
        column.set(pos, Block::Stone);
        assert_eq!(column.get(pos), Block::Stone);
        assert_eq!(column.section(2).get(IVec3::new(3, 5, 15)), Block::Stone);
        assert_eq!(column.get(IVec3::new(3, 36, 15)), Block::Air);
    }

    #[test]
    #[should_panic]
    fn get_above_column_panics() {
        Column::new().get(IVec3::new(0, WORLD_HEIGHT, 0));
    }

    #[test]
    #[should_panic]
    fn get_below_column_panics() {
        Column::new().get(IVec3::new(0, -1, 0));
    }
}
