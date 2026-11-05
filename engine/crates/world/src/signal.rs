use glam::IVec3;

use crate::block::{Block, HOOK_POWERED, LEVER_POWERED, PLATE_POWERED, Shape};
use crate::updates::Updates;
use crate::world::World;

pub const MAX_POWER: u8 = 15;

const AROUND: [IVec3; 6] = [
    IVec3::Y,
    IVec3::NEG_Y,
    IVec3::X,
    IVec3::NEG_X,
    IVec3::Z,
    IVec3::NEG_Z,
];

/// Выход источника: `weak` отдаётся соседям, `strong` — только твёрдому блоку в клетке `pos + strong_toward`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Output {
    pub weak: u8,
    pub strong: u8,
    pub strong_toward: IVec3,
}

pub fn output(block: Block, data: u8) -> Option<Output> {
    match block {
        Block::StonePressurePlate if data & PLATE_POWERED != 0 => Some(Output {
            weak: MAX_POWER,
            strong: MAX_POWER,
            strong_toward: IVec3::NEG_Y,
        }),
        Block::Lever if data & LEVER_POWERED != 0 => held_output(block, data),
        Block::TripwireHook if data & HOOK_POWERED != 0 => held_output(block, data),
        _ => None,
    }
}

/// Рычаг и крюк питают сильно только опору, к которой прикреплены.
fn held_output(block: Block, data: u8) -> Option<Output> {
    Some(Output {
        weak: MAX_POWER,
        strong: MAX_POWER,
        strong_toward: block.support_dir(data)?,
    })
}

fn conducts(block: Block) -> bool {
    block.is_solid() && block.def().opaque && block.shape() == Shape::Cube
}

fn output_at(world: &World, pos: IVec3) -> Option<Output> {
    output(world.block(pos), world.block_data(pos))
}

fn strong_power_into(world: &World, pos: IVec3) -> u8 {
    AROUND
        .iter()
        .filter_map(|&side| {
            let source = pos + side;
            output_at(world, source)
                .filter(|out| source + out.strong_toward == pos)
                .map(|out| out.strong)
        })
        .max()
        .unwrap_or(0)
}

/// Сила сигнала, которую клетка `pos` получает со стороны `face` (как `World.getRedstonePower` 1.12).
pub fn power_from(world: &World, pos: IVec3, face: IVec3) -> u8 {
    let neighbor = pos + face;
    if conducts(world.block(neighbor)) {
        strong_power_into(world, neighbor)
    } else {
        output_at(world, neighbor).map_or(0, |out| out.weak)
    }
}

pub fn power(world: &World, pos: IVec3) -> u8 {
    AROUND
        .iter()
        .map(|&face| power_from(world, pos, face))
        .max()
        .unwrap_or(0)
}

pub fn is_powered(world: &World, pos: IVec3) -> bool {
    power(world, pos) > 0
}

#[derive(Default)]
pub struct Signal {
    updates: Updates,
}

pub struct Ignition {
    pub changed: Vec<IVec3>,
    pub ignited: Vec<IVec3>,
}

impl Signal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.updates.is_empty()
    }

    /// Клетка и всё, что читает её сигнал: соседи и соседи твёрдых соседей (сильное питание).
    pub fn block_changed(&mut self, world: &World, pos: IVec3) {
        self.updates.wake(world, pos);
        for side in AROUND {
            let neighbor = pos + side;
            if conducts(world.block(neighbor)) {
                self.updates.wake(world, neighbor);
            }
        }
    }

    pub fn step(&mut self, world: &mut World) -> Ignition {
        let mut ignited = Vec::new();
        let changed = self.updates.step(world, |world, pos, changed| {
            if world.block(pos) == Block::Tnt && is_powered(world, pos) {
                let removed = world.set_block_supported(pos, Block::Air);
                if !removed.is_empty() {
                    ignited.push(pos);
                    changed.extend(removed);
                }
            }
        });
        Ignition { changed, ignited }
    }
}

#[cfg(test)]
mod tests;
