use glam::IVec3;

use super::piece::PieceWriter;
use crate::bed::{COLOR_SHIFT, HEAD};
use crate::block::{Block, CAULDRON_LEVELS, COLOR_COUNT};
use crate::chest;

pub const X: IVec3 = IVec3::X;
pub const NEG_X: IVec3 = IVec3::NEG_X;
pub const Z: IVec3 = IVec3::Z;
pub const NEG_Z: IVec3 = IVec3::NEG_Z;
pub const UP: IVec3 = IVec3::Y;
pub const DOWN: IVec3 = IVec3::NEG_Y;

/// Блок и его состояние в осях части; `data` — биты состояния без поворота, `front` дописывает поворот при записи.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cell {
    pub block: Block,
    pub data: u8,
    pub front: Option<IVec3>,
}

pub const AIR: Cell = Cell::of(Block::Air);

impl Cell {
    pub const fn of(block: Block) -> Self {
        Self {
            block,
            data: 0,
            front: None,
        }
    }

    pub const fn with_data(block: Block, data: u8) -> Self {
        Self {
            block,
            data,
            front: None,
        }
    }

    pub const fn front(self, front: IVec3) -> Self {
        Self {
            front: Some(front),
            ..self
        }
    }

    /// Половина кровати: `toward_head` — направление части от ножной половины к изголовью.
    pub fn bed(toward_head: IVec3, head: bool, color: u8) -> Self {
        let data = if head { HEAD } else { 0 } | (color % COLOR_COUNT) << COLOR_SHIFT;
        Self::with_data(Block::Bed, data).front(toward_head)
    }

    pub fn chest(front: IVec3) -> Self {
        Self::chest_half(front, chest::HALF_NONE)
    }

    /// Половина двойного сундука; правая — на `chest::right_of(front)` от левой.
    pub fn chest_half(front: IVec3, half: u8) -> Self {
        Self::with_data(Block::Chest, half << chest::HALF_SHIFT).front(front)
    }

    /// Котёл с водой `level` от 0 до 3.
    pub fn cauldron(level: u8) -> Self {
        Self::with_data(Block::Cauldron, level.min(CAULDRON_LEVELS))
    }

    /// Горшок с растением; растение вне списка горшка оставляет горшок пустым.
    pub fn flower_pot(plant: Option<Block>) -> Self {
        let data = plant.and_then(Block::pot_code).unwrap_or(0);
        Self::with_data(Block::FlowerPot, data)
    }
}

impl From<Block> for Cell {
    fn from(block: Block) -> Self {
        Cell::of(block)
    }
}

impl Block {
    pub const fn front(self, front: IVec3) -> Cell {
        Cell::of(self).front(front)
    }
}

/// `BlockSelector`: клетка по весам; выбор зависит от клетки и соли части, а не от порядка вызовов.
#[derive(Clone, Debug)]
pub struct Selector {
    entries: Vec<(u32, Cell)>,
    total: u32,
}

impl Selector {
    pub fn new(entries: &[(u32, Cell)]) -> Self {
        let total = entries.iter().map(|(weight, _)| weight).sum();
        assert!(total > 0, "selector without weights");
        Self {
            entries: entries.to_vec(),
            total,
        }
    }

    pub fn pick(&self, w: &PieceWriter, x: i32, y: i32, z: i32) -> Cell {
        let mut roll = w.pick(x, y, z, self.total);
        for &(weight, cell) in &self.entries {
            if roll < weight {
                return cell;
            }
            roll -= weight;
        }
        unreachable!("roll is below the total weight")
    }
}
