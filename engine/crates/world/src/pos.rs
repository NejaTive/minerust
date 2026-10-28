use std::ops::{Add, AddAssign, Sub, SubAssign};

use glam::{IVec2, IVec3};

use crate::section::SECTION_SIZE;

/// Ключ колонки: (x, z) в колонках, то есть мировой блок `div_euclid(16)`. Поле открыто
/// ради арифметики glam; тип нужен, чтобы не спутать ключ с позицией блока и с ключом секции.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct ColumnPos(pub IVec2);

impl ColumnPos {
    pub const ZERO: Self = Self(IVec2::ZERO);

    pub const fn new(x: i32, z: i32) -> Self {
        Self(IVec2::new(x, z))
    }

    pub const fn x(self) -> i32 {
        self.0.x
    }

    pub const fn z(self) -> i32 {
        self.0.y
    }

    /// Колонка, в которой лежит блок с мировыми координатами `block` (в блоках).
    pub fn of_block(block: IVec3) -> Self {
        Self(IVec2::new(block.x, block.z).div_euclid(IVec2::splat(SECTION_SIZE)))
    }

    /// Мировой угол колонки (min x, 0, min z) в блоках.
    pub fn origin(self) -> IVec3 {
        IVec3::new(self.0.x * SECTION_SIZE, 0, self.0.y * SECTION_SIZE)
    }

    /// Ключ секции этой колонки с номером `sy` (0..COLUMN_SECTIONS).
    pub fn section(self, sy: i32) -> SectionPos {
        SectionPos::new(self.0.x, sy, self.0.y)
    }

    /// Квадрат расстояния до `other` в колонках.
    pub fn dist2(self, other: Self) -> i32 {
        (self.0 - other.0).length_squared()
    }
}

/// Сдвиг ключа на целое число колонок.
impl Add<IVec2> for ColumnPos {
    type Output = Self;

    fn add(self, offset: IVec2) -> Self {
        Self(self.0 + offset)
    }
}

impl AddAssign<IVec2> for ColumnPos {
    fn add_assign(&mut self, offset: IVec2) {
        self.0 += offset;
    }
}

impl SubAssign<IVec2> for ColumnPos {
    fn sub_assign(&mut self, offset: IVec2) {
        self.0 -= offset;
    }
}

/// Разность ключей — вектор в колонках.
impl Sub for ColumnPos {
    type Output = IVec2;

    fn sub(self, other: Self) -> IVec2 {
        self.0 - other.0
    }
}

impl Ord for ColumnPos {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.to_array().cmp(&other.0.to_array())
    }
}

impl PartialOrd for ColumnPos {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::fmt::Display for ColumnPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "column {}", self.0)
    }
}

/// Ключ секции: (x колонки, номер секции по высоте, z колонки). Поле открыто по той же
/// причине, что у [`ColumnPos`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct SectionPos(pub IVec3);

impl SectionPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self(IVec3::new(x, y, z))
    }

    /// Секция, в которой лежит блок с мировыми координатами `block` (в блоках).
    pub fn of_block(block: IVec3) -> Self {
        Self(block.div_euclid(IVec3::splat(SECTION_SIZE)))
    }

    /// Колонка, которой принадлежит секция.
    pub fn column(self) -> ColumnPos {
        ColumnPos::new(self.0.x, self.0.z)
    }

    /// Номер секции в колонке (0..COLUMN_SECTIONS у существующих секций).
    pub const fn y(self) -> i32 {
        self.0.y
    }

    /// Мировой угол секции (min x, min y, min z) в блоках.
    pub fn origin(self) -> IVec3 {
        self.0 * SECTION_SIZE
    }
}

impl Add<IVec3> for SectionPos {
    type Output = Self;

    fn add(self, offset: IVec3) -> Self {
        Self(self.0 + offset)
    }
}

impl Ord for SectionPos {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.to_array().cmp(&other.0.to_array())
    }
}

impl PartialOrd for SectionPos {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::fmt::Display for SectionPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "section {}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_to_keys_uses_floor_division() {
        assert_eq!(
            ColumnPos::of_block(IVec3::new(-1, 5, -16)),
            ColumnPos::new(-1, -1)
        );
        assert_eq!(
            ColumnPos::of_block(IVec3::new(-17, 5, 15)),
            ColumnPos::new(-2, 0)
        );
        assert_eq!(
            SectionPos::of_block(IVec3::new(5, 40, -7)),
            SectionPos::new(0, 2, -1)
        );
        assert_eq!(
            SectionPos::of_block(IVec3::new(5, 40, -7)).column(),
            ColumnPos::new(0, -1)
        );
    }

    #[test]
    fn origins_are_world_corners() {
        assert_eq!(ColumnPos::new(-2, 3).origin(), IVec3::new(-32, 0, 48));
        assert_eq!(SectionPos::new(-2, 3, 1).origin(), IVec3::new(-32, 48, 16));
        assert_eq!(ColumnPos::new(1, -1).section(4), SectionPos::new(1, 4, -1));
    }
}
