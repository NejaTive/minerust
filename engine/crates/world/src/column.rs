use glam::IVec3;

use crate::block::Block;
use crate::section::{SECTION_SIZE, SECTION_VOLUME, Section};

pub const COLUMN_SECTIONS: i32 = 8;
pub const WORLD_HEIGHT: i32 = COLUMN_SECTIONS * SECTION_SIZE;

#[derive(Clone)]
pub struct Column {
    sections: [Section; COLUMN_SECTIONS as usize],
}

impl Column {
    pub fn new() -> Self {
        Self {
            sections: std::array::from_fn(|_| Section::new()),
        }
    }

    pub fn contains(local: IVec3) -> bool {
        local.cmpge(IVec3::ZERO).all()
            && local
                .cmplt(IVec3::new(SECTION_SIZE, WORLD_HEIGHT, SECTION_SIZE))
                .all()
    }

    pub fn section(&self, sy: i32) -> &Section {
        &self.sections[sy as usize]
    }

    pub fn get(&self, local: IVec3) -> Block {
        let (sy, in_section) = Self::split(local);
        self.sections[sy].get(in_section)
    }

    pub fn get_data(&self, local: IVec3) -> u8 {
        let (sy, in_section) = Self::split(local);
        self.sections[sy].get_data(in_section)
    }

    pub fn set_section_data(&mut self, sy: i32, data: Option<Box<[u8; SECTION_VOLUME]>>) {
        self.sections[sy as usize].set_data(data);
    }

    pub fn get_index(&self, index: usize) -> Block {
        self.sections[index / SECTION_VOLUME].get_index(index % SECTION_VOLUME)
    }

    pub fn set(&mut self, local: IVec3, block: Block) {
        self.set_with_data(local, block, 0);
    }

    pub fn set_with_data(&mut self, local: IVec3, block: Block, data: u8) {
        let (sy, in_section) = Self::split(local);
        self.sections[sy].set_with_data(in_section, block, data);
    }

    fn split(local: IVec3) -> (usize, IVec3) {
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
