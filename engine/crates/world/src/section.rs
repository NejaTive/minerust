use glam::IVec3;

use crate::block::Block;

pub const SECTION_SIZE: i32 = 16;
pub const SECTION_VOLUME: usize = 4096;

#[derive(Clone)]
pub struct Section {
    blocks: Box<[Block; SECTION_VOLUME]>,
    emitters: u16,
    data: Option<Box<[u8; SECTION_VOLUME]>>,
}

impl Section {
    pub fn new() -> Self {
        Self {
            blocks: Box::new([Block::Air; SECTION_VOLUME]),
            emitters: 0,
            data: None,
        }
    }

    pub fn contains(local: IVec3) -> bool {
        local.cmpge(IVec3::ZERO).all() && local.cmplt(IVec3::splat(SECTION_SIZE)).all()
    }

    pub fn get(&self, local: IVec3) -> Block {
        self.blocks[Self::index(local)]
    }

    pub fn emits_light(&self) -> bool {
        self.emitters > 0
    }

    pub fn get_index(&self, index: usize) -> Block {
        self.blocks[index]
    }

    pub fn row(&self, y: i32, z: i32) -> &[Block] {
        let start = (SECTION_SIZE * z + SECTION_SIZE * SECTION_SIZE * y) as usize;
        &self.blocks[start..start + SECTION_SIZE as usize]
    }

    pub fn data_row(&self, y: i32, z: i32) -> Option<&[u8]> {
        let start = (SECTION_SIZE * z + SECTION_SIZE * SECTION_SIZE * y) as usize;
        self.data
            .as_ref()
            .map(|data| &data[start..start + SECTION_SIZE as usize])
    }

    pub fn is_air(&self) -> bool {
        self.blocks.iter().all(|&b| b == Block::Air)
    }

    pub fn get_data(&self, local: IVec3) -> u8 {
        let index = Self::index(local);
        self.data.as_ref().map_or(0, |data| data[index])
    }

    pub fn data(&self) -> Option<&[u8; SECTION_VOLUME]> {
        self.data.as_deref()
    }

    pub fn set_data(&mut self, data: Option<Box<[u8; SECTION_VOLUME]>>) {
        self.data = data.filter(|data| data.iter().any(|&b| b != 0));
    }

    pub fn set(&mut self, local: IVec3, block: Block) {
        self.set_with_data(local, block, 0);
    }

    pub fn set_with_data(&mut self, local: IVec3, block: Block, data: u8) {
        let index = Self::index(local);
        match &mut self.data {
            Some(bytes) => bytes[index] = data,
            None if data != 0 => self.data.insert(Box::new([0; SECTION_VOLUME]))[index] = data,
            None => {}
        }
        let slot = &mut self.blocks[index];
        self.emitters -= u16::from(slot.emission() > 0);
        self.emitters += u16::from(block.emission() > 0);
        *slot = block;
    }

    fn index(local: IVec3) -> usize {
        assert!(
            Self::contains(local),
            "local coords out of section: {local}"
        );
        (local.x + SECTION_SIZE * local.z + SECTION_SIZE * SECTION_SIZE * local.y) as usize
    }
}

impl Default for Section {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
