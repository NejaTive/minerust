use glam::IVec3;

use crate::block::Block;
use crate::section::{SECTION_SIZE, SECTION_VOLUME, Section};

pub const COLUMN_SECTIONS: i32 = 8;
pub const WORLD_HEIGHT: i32 = COLUMN_SECTIONS * SECTION_SIZE;

/// Колонка 16×128×16: секции снизу вверх, `sections[0]` — y 0..16.
#[derive(Clone)]
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

    /// Блок по индексу `x + 16·z + 256·y` в колонке (y до 128): тот же порядок, что
    /// внутри секции, так что секция — это 4096 подряд идущих индексов. Для горячих
    /// циклов (свет), где индекс уже посчитан; за пределами колонки — panic.
    pub fn get_index(&self, index: usize) -> Block {
        self.sections[index / SECTION_VOLUME].get_index(index % SECTION_VOLUME)
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
mod tests;
