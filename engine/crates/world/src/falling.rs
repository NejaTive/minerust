use glam::IVec3;

use crate::block::Block;
use crate::updates::Updates;
use crate::world::World;

#[derive(Default)]
pub struct Falling {
    updates: Updates,
}

impl Falling {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.updates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.updates.is_empty()
    }

    pub fn block_changed(&mut self, world: &World, pos: IVec3) {
        self.updates.wake(world, pos);
    }

    pub fn step(&mut self, world: &mut World, occupied: impl Fn(IVec3) -> bool) -> Vec<IVec3> {
        let mut blocked = Vec::new();
        let changed = self.updates.step(world, |world, pos, changed| {
            if update(world, pos, &occupied, changed) {
                blocked.push(pos);
            }
        });
        for pos in blocked {
            self.updates.push(world, pos);
        }
        changed
    }
}

fn update(
    world: &mut World,
    pos: IVec3,
    occupied: &impl Fn(IVec3) -> bool,
    changed: &mut Vec<IVec3>,
) -> bool {
    let block = world.block(pos);
    let below = pos - IVec3::Y;
    if !block.falls() || below.y < 0 || !world.is_loaded(below) || world.block(below).is_solid() {
        return false;
    }
    if occupied(below) {
        return true;
    }
    changed.extend(world.set_block_supported_with_data(below, block, 0));
    changed.extend(world.set_block_supported(pos, Block::Air));
    false
}

#[cfg(test)]
mod tests;
