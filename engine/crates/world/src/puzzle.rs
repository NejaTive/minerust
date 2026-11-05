use glam::IVec3;

use crate::block::{Block, LEVER_FACING_MASK, LEVER_MOUNT_MASK, LEVER_POWERED};
use crate::world::World;

pub const MARKER: Block = Block::ChiseledStoneBricks;
pub const DOOR: Block = Block::Cobblestone;
pub const SOLUTION: [bool; 3] = [true, false, true];
pub const DOOR_DISTANCE: i32 = 4;

const AXES: [IVec3; 2] = [IVec3::X, IVec3::Z];

impl World {
    /// Три рычага в ряд на стене из резного камня: нужная комбинация убирает дверь через `DOOR_DISTANCE` клеток.
    pub fn solve_lever_puzzle(&mut self, lever: IVec3) -> Vec<IVec3> {
        for axis in AXES {
            for shift in -1..=1 {
                if let Some(changed) = self.update_puzzle(lever + axis * shift) {
                    return changed;
                }
            }
        }
        Vec::new()
    }

    fn update_puzzle(&mut self, center: IVec3) -> Option<Vec<IVec3>> {
        let data = self.block_data(center);
        if self.block(center) != Block::Lever || data & LEVER_MOUNT_MASK != 0 {
            return None;
        }
        let away = Block::Lever.facing_dir(data & LEVER_FACING_MASK);
        if self.block(center - away) != MARKER {
            return None;
        }
        let right = IVec3::new(away.z, 0, -away.x);
        let shape = data & !LEVER_POWERED;
        let mut combination = [false; 3];
        for (slot, offset) in combination.iter_mut().zip(-1..=1) {
            let pos = center + right * offset;
            let lever = self.block_data(pos);
            if self.block(pos) != Block::Lever || lever & !LEVER_POWERED != shape {
                return None;
            }
            *slot = lever & LEVER_POWERED != 0;
        }
        let open = combination == SOLUTION;
        let (from, to) = if open {
            (DOOR, Block::Air)
        } else {
            (Block::Air, DOOR)
        };
        let door = center + away * DOOR_DISTANCE;
        let mut changed = Vec::new();
        for pos in [door, door + IVec3::NEG_Y] {
            if self.block(pos) == from && self.set_block_with_data(pos, to, 0) {
                changed.push(pos);
            }
        }
        Some(changed)
    }
}

#[cfg(test)]
mod tests;
