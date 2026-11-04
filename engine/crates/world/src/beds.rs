use glam::IVec3;

use crate::bed;
use crate::block::Block;
use crate::world::World;

impl World {
    pub fn can_place_bed(&self, foot: IVec3, look: IVec3) -> bool {
        [foot, foot + look].into_iter().all(|pos| {
            self.is_loaded(pos)
                && self.block(pos).is_replaceable()
                && self.block(pos + IVec3::NEG_Y).is_solid()
        })
    }

    /// Ножная половина в `foot`, изголовье на шаг дальше по `look`.
    pub fn place_bed(&mut self, foot: IVec3, look: IVec3, color: u8) -> Vec<IVec3> {
        if !self.can_place_bed(foot, look) {
            return Vec::new();
        }
        let head = foot + look;
        if !self.set_block_with_data(foot, Block::Bed, bed::data(look, false, color))
            || !self.set_block_with_data(head, Block::Bed, bed::data(look, true, color))
        {
            return Vec::new();
        }
        vec![foot, head]
    }
}

#[cfg(test)]
mod tests;
