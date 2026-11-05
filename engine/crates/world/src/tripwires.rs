use std::collections::{HashMap, HashSet};

use glam::{IVec3, Vec3};

use crate::block::{Block, HOOK_FACING_MASK, HOOK_POWERED};
use crate::column::{COLUMN_SECTIONS, Column};
use crate::plates::Body;
use crate::pos::ColumnPos;
use crate::section::SECTION_SIZE;
use crate::world::World;

pub const RELEASE_TICKS: u32 = 10;
pub const MAX_WIRES: i32 = 40;

const WIRE_HEIGHT: f32 = 0.156_25;

#[derive(Default)]
pub struct Tripwires {
    held: HashMap<IVec3, u32>,
}

impl Tripwires {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn adopt_column(&mut self, key: ColumnPos, column: &Column) {
        let origin = key.origin();
        for sy in 0..COLUMN_SECTIONS {
            let section = column.section(sy);
            if section.data().is_none() {
                continue;
            }
            for y in 0..SECTION_SIZE {
                for z in 0..SECTION_SIZE {
                    for x in 0..SECTION_SIZE {
                        let local = IVec3::new(x, y, z);
                        if section.get(local) == Block::TripwireHook
                            && section.get_data(local) & HOOK_POWERED != 0
                        {
                            self.held.insert(
                                origin + IVec3::new(x, sy * SECTION_SIZE + y, z),
                                RELEASE_TICKS,
                            );
                        }
                    }
                }
            }
        }
    }

    /// Один тик: тела на натянутой нити включают оба крюка, через `RELEASE_TICKS` после ухода — выключают.
    pub fn tick(&mut self, world: &mut World, bodies: &[Body]) -> Vec<IVec3> {
        let mut changed = Vec::new();
        let tripped = tripped_hooks(world, bodies);
        for &pos in &tripped {
            self.held.insert(pos, RELEASE_TICKS);
            let data = world.block_data(pos);
            if data & HOOK_POWERED == 0
                && world.set_block_with_data(pos, Block::TripwireHook, data | HOOK_POWERED)
            {
                changed.push(pos);
            }
        }
        self.held.retain(|pos, timer| {
            if tripped.contains(pos) {
                return true;
            }
            if world.block(*pos) != Block::TripwireHook {
                return false;
            }
            *timer -= 1;
            if *timer > 0 {
                return true;
            }
            let data = world.block_data(*pos);
            if world.set_block_with_data(*pos, Block::TripwireHook, data & !HOOK_POWERED) {
                changed.push(*pos);
            }
            false
        });
        changed
    }
}

/// Крюки на концах натянутой нити в клетке `wire`: два крюка, смотрящие друг на друга, нить между ними.
pub fn line_ends(world: &World, wire: IVec3) -> Option<(IVec3, IVec3)> {
    [IVec3::X, IVec3::Z].into_iter().find_map(|axis| {
        let forward = hook_along(world, wire, axis)?;
        let backward = hook_along(world, wire, -axis)?;
        ((forward - backward).abs().max_element() <= MAX_WIRES + 1).then_some((forward, backward))
    })
}

fn hook_along(world: &World, from: IVec3, dir: IVec3) -> Option<IVec3> {
    for step in 1..=MAX_WIRES + 1 {
        let pos = from + dir * step;
        match world.block(pos) {
            Block::Tripwire => {}
            Block::TripwireHook => {
                let facing =
                    Block::TripwireHook.facing_dir(world.block_data(pos) & HOOK_FACING_MASK);
                return (facing == -dir).then_some(pos);
            }
            _ => return None,
        }
    }
    None
}

fn tripped_hooks(world: &World, bodies: &[Body]) -> HashSet<IVec3> {
    let mut found = HashSet::new();
    for &(min, max) in bodies {
        let lo = min.floor().as_ivec3();
        let hi = max.floor().as_ivec3();
        for y in lo.y..=hi.y {
            for z in lo.z..=hi.z {
                for x in lo.x..=hi.x {
                    let pos = IVec3::new(x, y, z);
                    if world.block(pos) != Block::Tripwire {
                        continue;
                    }
                    let base = pos.as_vec3();
                    let reaches = min.cmplt(base + Vec3::new(1.0, WIRE_HEIGHT, 1.0)).all()
                        && max.cmpgt(base).all();
                    if reaches && let Some((a, b)) = line_ends(world, pos) {
                        found.insert(a);
                        found.insert(b);
                    }
                }
            }
        }
    }
    found
}

#[cfg(test)]
mod tests;
