use std::collections::{HashSet, VecDeque};

use glam::IVec3;

use crate::column::WORLD_HEIGHT;
use crate::world::World;

pub const STEP_LIMIT: usize = 512;

const AROUND: [IVec3; 6] = [
    IVec3::Y,
    IVec3::NEG_Y,
    IVec3::X,
    IVec3::NEG_X,
    IVec3::Z,
    IVec3::NEG_Z,
];

#[derive(Default)]
pub struct Updates {
    queue: VecDeque<IVec3>,
    queued: HashSet<IVec3>,
}

impl Updates {
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn push(&mut self, world: &World, pos: IVec3) {
        if (0..WORLD_HEIGHT).contains(&pos.y) && world.is_loaded(pos) && self.queued.insert(pos) {
            self.queue.push_back(pos);
        }
    }

    pub fn wake(&mut self, world: &World, pos: IVec3) {
        for cell in std::iter::once(IVec3::ZERO).chain(AROUND) {
            self.push(world, pos + cell);
        }
    }

    pub fn step(
        &mut self,
        world: &mut World,
        mut update: impl FnMut(&mut World, IVec3, &mut Vec<IVec3>),
    ) -> Vec<IVec3> {
        let mut changed = Vec::new();
        let mut touched = HashSet::new();
        for _ in 0..self.queue.len().min(STEP_LIMIT) {
            let Some(pos) = self.queue.pop_front() else {
                break;
            };
            if touched.contains(&pos) {
                self.queue.push_back(pos);
                continue;
            }
            self.queued.remove(&pos);
            let before = changed.len();
            update(world, pos, &mut changed);
            for &cell in &changed[before..] {
                touched.insert(cell);
                self.wake(world, cell);
            }
        }
        changed
    }
}
