use std::ops::{Add, AddAssign, Sub, SubAssign};

use glam::{IVec2, IVec3};

use crate::section::SECTION_SIZE;

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

    pub fn of_block(block: IVec3) -> Self {
        Self(IVec2::new(block.x, block.z).div_euclid(IVec2::splat(SECTION_SIZE)))
    }

    pub fn origin(self) -> IVec3 {
        IVec3::new(self.0.x * SECTION_SIZE, 0, self.0.y * SECTION_SIZE)
    }

    pub fn section(self, sy: i32) -> SectionPos {
        SectionPos::new(self.0.x, sy, self.0.y)
    }

    pub fn dist2(self, other: Self) -> i32 {
        (self.0 - other.0).length_squared()
    }
}

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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct SectionPos(pub IVec3);

impl SectionPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self(IVec3::new(x, y, z))
    }

    pub fn of_block(block: IVec3) -> Self {
        Self(block.div_euclid(IVec3::splat(SECTION_SIZE)))
    }

    pub fn column(self) -> ColumnPos {
        ColumnPos::new(self.0.x, self.0.z)
    }

    pub const fn y(self) -> i32 {
        self.0.y
    }

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
mod tests;
