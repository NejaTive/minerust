use std::collections::{HashMap, HashSet};

use glam::{IVec3, Vec3};

use crate::block::{Block, PLATE_POWERED};
use crate::column::{COLUMN_SECTIONS, Column};
use crate::pos::ColumnPos;
use crate::section::SECTION_SIZE;
use crate::world::World;

pub const RELEASE_TICKS: u32 = 20;

const SENSITIVE_MIN: Vec3 = Vec3::new(0.125, 0.0, 0.125);
const SENSITIVE_MAX: Vec3 = Vec3::new(0.875, 0.25, 0.875);

pub type Body = (Vec3, Vec3);

#[derive(Default)]
pub struct Plates {
    pressed: HashMap<IVec3, u32>,
}

impl Plates {
    pub fn new() -> Self {
        Self::default()
    }

    /// Нажатые плиты появившейся колонки получают отложенную проверку, как отложенный тик плиты 1.12.
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
                        if section.get(local) == Block::StonePressurePlate
                            && section.get_data(local) & PLATE_POWERED != 0
                        {
                            self.pressed.insert(
                                origin + IVec3::new(x, sy * SECTION_SIZE + y, z),
                                RELEASE_TICKS,
                            );
                        }
                    }
                }
            }
        }
    }

    /// Один тик 1.12: нажимает плиты под телами и отпускает те, с которых ушли `RELEASE_TICKS` тиков назад.
    pub fn tick(&mut self, world: &mut World, bodies: &[Body]) -> Vec<IVec3> {
        let mut changed = Vec::new();
        let stepped = stepped_plates(world, bodies);
        for &pos in &stepped {
            self.pressed.insert(pos, RELEASE_TICKS);
            if world.block_data(pos) & PLATE_POWERED == 0
                && world.set_block_with_data(pos, Block::StonePressurePlate, PLATE_POWERED)
            {
                changed.push(pos);
            }
        }
        self.pressed.retain(|pos, timer| {
            if stepped.contains(pos) {
                return true;
            }
            if world.block(*pos) != Block::StonePressurePlate {
                return false;
            }
            *timer -= 1;
            if *timer > 0 {
                return true;
            }
            if world.set_block_with_data(*pos, Block::StonePressurePlate, 0) {
                changed.push(*pos);
            }
            false
        });
        changed
    }
}

fn stepped_plates(world: &World, bodies: &[Body]) -> HashSet<IVec3> {
    let mut found = HashSet::new();
    for &(min, max) in bodies {
        let lo = min.floor().as_ivec3();
        let hi = max.floor().as_ivec3();
        for y in lo.y..=hi.y {
            for z in lo.z..=hi.z {
                for x in lo.x..=hi.x {
                    let pos = IVec3::new(x, y, z);
                    if world.block(pos) != Block::StonePressurePlate {
                        continue;
                    }
                    let base = pos.as_vec3();
                    if min.cmplt(base + SENSITIVE_MAX).all()
                        && max.cmpgt(base + SENSITIVE_MIN).all()
                    {
                        found.insert(pos);
                    }
                }
            }
        }
    }
    found
}

#[cfg(test)]
mod tests;
