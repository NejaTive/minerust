use glam::IVec3;

use crate::block::{Block, FLUID_FALLING, FLUID_LEVEL_MASK, FLUID_MAX_LEVEL, FLUID_SOURCE};
use crate::column::WORLD_HEIGHT;
use crate::updates::Updates;
use crate::world::World;

const SLOPE_DISTANCE: u32 = 4;
const NO_SLOPE: u32 = 1000;

const SIDES: [IVec3; 4] = [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z];
#[derive(Clone, Copy, PartialEq, Eq)]
enum Cell {
    Solid,
    Open,
    Water(u8),
}

fn probe(world: &World, pos: IVec3) -> Cell {
    if pos.y < 0 || (pos.y < WORLD_HEIGHT && !world.is_loaded(pos)) {
        return Cell::Solid;
    }
    let block = world.block(pos);
    if block.is_water() {
        Cell::Water(world.block_data(pos))
    } else if block.is_solid() {
        Cell::Solid
    } else {
        Cell::Open
    }
}

fn spread_level(data: u8) -> u8 {
    if data & FLUID_FALLING != 0 {
        0
    } else {
        data & FLUID_LEVEL_MASK
    }
}

#[derive(Default)]
pub struct Fluid {
    updates: Updates,
}

impl Fluid {
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

    pub fn step(&mut self, world: &mut World) -> Vec<IVec3> {
        self.updates.step(world, update)
    }
}

fn set(world: &mut World, pos: IVec3, block: Block, data: u8, changed: &mut Vec<IVec3>) {
    changed.extend(world.set_block_supported_with_data(pos, block, data));
}

fn place(world: &mut World, pos: IVec3, data: u8, changed: &mut Vec<IVec3>) {
    if probe(world, pos) == Cell::Open {
        set(world, pos, Block::Water, data, changed);
    }
}

fn update(world: &mut World, pos: IVec3, changed: &mut Vec<IVec3>) {
    let Cell::Water(mut data) = probe(world, pos) else {
        return;
    };
    if data != FLUID_SOURCE {
        match flowing_state(world, pos) {
            None => return set(world, pos, Block::Air, 0, changed),
            Some(state) if state != data => {
                set(world, pos, Block::Water, state, changed);
                data = state;
            }
            Some(_) => {}
        }
    }
    spread(world, pos, data, changed);
}

fn flowing_state(world: &World, pos: IVec3) -> Option<u8> {
    let mut sources = 0;
    let mut lowest: Option<u8> = None;
    for dir in SIDES {
        if let Cell::Water(data) = probe(world, pos + dir) {
            sources += usize::from(data == FLUID_SOURCE);
            let level = spread_level(data);
            lowest = Some(lowest.map_or(level, |l| l.min(level)));
        }
    }
    if sources >= 2
        && matches!(
            probe(world, pos - IVec3::Y),
            Cell::Solid | Cell::Water(FLUID_SOURCE)
        )
    {
        return Some(FLUID_SOURCE);
    }
    if matches!(probe(world, pos + IVec3::Y), Cell::Water(_)) {
        return Some(FLUID_FALLING);
    }
    lowest
        .map(|level| level + 1)
        .filter(|&level| level <= FLUID_MAX_LEVEL)
}

fn spread(world: &mut World, pos: IVec3, data: u8, changed: &mut Vec<IVec3>) {
    let source = data == FLUID_SOURCE;
    let below = pos - IVec3::Y;
    match probe(world, below) {
        Cell::Open => {
            place(world, below, FLUID_FALLING, changed);
            if side_sources(world, pos) >= 3 {
                spread_sides(world, pos, data, changed);
            }
        }
        Cell::Water(under) => {
            if under != FLUID_SOURCE && under & FLUID_FALLING == 0 {
                set(world, below, Block::Water, FLUID_FALLING, changed);
            }
            if source {
                spread_sides(world, pos, data, changed);
            }
        }
        Cell::Solid => spread_sides(world, pos, data, changed),
    }
}

fn side_sources(world: &World, pos: IVec3) -> usize {
    SIDES
        .into_iter()
        .filter(|&dir| probe(world, pos + dir) == Cell::Water(FLUID_SOURCE))
        .count()
}

fn spread_sides(world: &mut World, pos: IVec3, data: u8, changed: &mut Vec<IVec3>) {
    let level = spread_level(data) + 1;
    if level > FLUID_MAX_LEVEL {
        return;
    }
    for dir in nearest_slope_dirs(world, pos) {
        place(world, pos + dir, level, changed);
    }
}

fn nearest_slope_dirs(world: &World, pos: IVec3) -> Vec<IVec3> {
    let mut best = u32::MAX;
    let mut dirs = Vec::new();
    for dir in SIDES {
        let next = pos + dir;
        if probe(world, next) != Cell::Open {
            continue;
        }
        let distance = if is_hole(world, next) {
            0
        } else {
            slope_distance(world, next, 1, -dir)
        };
        if distance < best {
            best = distance;
            dirs.clear();
        }
        if distance <= best {
            dirs.push(dir);
        }
    }
    dirs
}

fn slope_distance(world: &World, pos: IVec3, depth: u32, from: IVec3) -> u32 {
    let mut best = NO_SLOPE;
    for dir in SIDES {
        if dir == from {
            continue;
        }
        let next = pos + dir;
        let passable = match probe(world, next) {
            Cell::Open => true,
            Cell::Water(data) => data != FLUID_SOURCE,
            Cell::Solid => false,
        };
        if !passable {
            continue;
        }
        if is_hole(world, next) {
            return depth;
        }
        if depth < SLOPE_DISTANCE {
            best = best.min(slope_distance(world, next, depth + 1, -dir));
        }
    }
    best
}

fn is_hole(world: &World, pos: IVec3) -> bool {
    matches!(probe(world, pos - IVec3::Y), Cell::Open | Cell::Water(_))
}

#[cfg(test)]
mod tests;
