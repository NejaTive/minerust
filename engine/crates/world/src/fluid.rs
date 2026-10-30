use glam::IVec3;

use crate::block::{
    Block, FLUID_FALLING, FLUID_LEVEL_MASK, FLUID_MAX_LEVEL, FLUID_SOURCE, FluidKind,
};
use crate::column::WORLD_HEIGHT;
use crate::updates::Updates;
use crate::world::World;

const NO_SLOPE: u32 = 1000;

const SIDES: [IVec3; 4] = [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z];

struct Params {
    level_step: u8,
    infinite: bool,
    slope_distance: u32,
    hardens_near_water: bool,
    petrifies_foreign_below: bool,
    solidifies_beside_foreign: bool,
}

fn params(kind: FluidKind) -> Params {
    match kind {
        FluidKind::Water => Params {
            level_step: 1,
            infinite: true,
            slope_distance: 4,
            hardens_near_water: false,
            petrifies_foreign_below: false,
            solidifies_beside_foreign: true,
        },
        FluidKind::Lava => Params {
            level_step: 2,
            infinite: false,
            slope_distance: 2,
            hardens_near_water: true,
            petrifies_foreign_below: true,
            solidifies_beside_foreign: false,
        },
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Cell {
    Solid,
    Open,
    Own(u8),
    Foreign,
}

fn probe(world: &World, pos: IVec3, kind: FluidKind) -> Cell {
    if pos.y < 0 || (pos.y < WORLD_HEIGHT && !world.is_loaded(pos)) {
        return Cell::Solid;
    }
    let block = world.block(pos);
    match block.fluid() {
        Some(found) if found == kind => Cell::Own(world.block_data(pos)),
        Some(_) => Cell::Foreign,
        None if block.is_solid() => Cell::Solid,
        None => Cell::Open,
    }
}

fn spread_level(data: u8) -> u8 {
    if data & FLUID_FALLING != 0 {
        0
    } else {
        data & FLUID_LEVEL_MASK
    }
}

pub struct Fluid {
    kind: FluidKind,
    updates: Updates,
}

impl Default for Fluid {
    fn default() -> Self {
        Self::of(FluidKind::Water)
    }
}

impl Fluid {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn of(kind: FluidKind) -> Self {
        Self {
            kind,
            updates: Updates::default(),
        }
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
        let kind = self.kind;
        self.updates.step(world, |world, pos, changed| {
            update(world, pos, kind, changed)
        })
    }
}

fn set(world: &mut World, pos: IVec3, block: Block, data: u8, changed: &mut Vec<IVec3>) {
    changed.extend(world.set_block_supported_with_data(pos, block, data));
}

fn place(world: &mut World, pos: IVec3, kind: FluidKind, data: u8, changed: &mut Vec<IVec3>) {
    if probe(world, pos, kind) != Cell::Open {
        return;
    }
    let foreign_data: Vec<u8> = SIDES
        .into_iter()
        .filter(|&dir| probe(world, pos + dir, kind) == Cell::Foreign)
        .map(|dir| world.block_data(pos + dir))
        .collect();
    let beside_flow_only =
        !foreign_data.is_empty() && foreign_data.iter().all(|&data| data != FLUID_SOURCE);
    if params(kind).solidifies_beside_foreign && beside_flow_only {
        set(world, pos, Block::Cobblestone, 0, changed);
    } else {
        set(world, pos, kind.block(), data, changed);
    }
}

fn touches_water(world: &World, pos: IVec3) -> bool {
    [IVec3::Y, IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z]
        .into_iter()
        .any(|dir| world.block(pos + dir).is_water())
}

fn update(world: &mut World, pos: IVec3, kind: FluidKind, changed: &mut Vec<IVec3>) {
    let Cell::Own(mut data) = probe(world, pos, kind) else {
        return;
    };
    if params(kind).hardens_near_water && touches_water(world, pos) {
        let hardened = if data == FLUID_SOURCE {
            Block::Obsidian
        } else {
            Block::Cobblestone
        };
        return set(world, pos, hardened, 0, changed);
    }
    if data != FLUID_SOURCE {
        match flowing_state(world, pos, kind) {
            None => return set(world, pos, Block::Air, 0, changed),
            Some(state) if state != data => {
                set(world, pos, kind.block(), state, changed);
                data = state;
            }
            Some(_) => {}
        }
    }
    spread(world, pos, kind, data, changed);
}

fn flowing_state(world: &World, pos: IVec3, kind: FluidKind) -> Option<u8> {
    let params = params(kind);
    let mut sources = 0;
    let mut lowest: Option<u8> = None;
    for dir in SIDES {
        if let Cell::Own(data) = probe(world, pos + dir, kind) {
            sources += usize::from(data == FLUID_SOURCE);
            let level = spread_level(data);
            lowest = Some(lowest.map_or(level, |l| l.min(level)));
        }
    }
    if params.infinite
        && sources >= 2
        && matches!(
            probe(world, pos - IVec3::Y, kind),
            Cell::Solid | Cell::Own(FLUID_SOURCE)
        )
    {
        return Some(FLUID_SOURCE);
    }
    if matches!(probe(world, pos + IVec3::Y, kind), Cell::Own(_)) {
        return Some(FLUID_FALLING);
    }
    lowest
        .map(|level| level + params.level_step)
        .filter(|&level| level <= FLUID_MAX_LEVEL)
}

fn spread(world: &mut World, pos: IVec3, kind: FluidKind, data: u8, changed: &mut Vec<IVec3>) {
    let source = data == FLUID_SOURCE;
    let below = pos - IVec3::Y;
    match probe(world, below, kind) {
        Cell::Open => {
            place(world, below, kind, FLUID_FALLING, changed);
            if side_sources(world, pos, kind) >= 3 {
                spread_sides(world, pos, kind, data, changed);
            }
        }
        Cell::Own(under) => {
            if under != FLUID_SOURCE && under & FLUID_FALLING == 0 {
                set(world, below, kind.block(), FLUID_FALLING, changed);
            }
            if source {
                spread_sides(world, pos, kind, data, changed);
            }
        }
        Cell::Foreign if params(kind).petrifies_foreign_below => {
            set(world, below, Block::Stone, 0, changed);
        }
        Cell::Solid | Cell::Foreign => spread_sides(world, pos, kind, data, changed),
    }
}

fn side_sources(world: &World, pos: IVec3, kind: FluidKind) -> usize {
    SIDES
        .into_iter()
        .filter(|&dir| probe(world, pos + dir, kind) == Cell::Own(FLUID_SOURCE))
        .count()
}

fn spread_sides(
    world: &mut World,
    pos: IVec3,
    kind: FluidKind,
    data: u8,
    changed: &mut Vec<IVec3>,
) {
    let level = spread_level(data) + params(kind).level_step;
    if level > FLUID_MAX_LEVEL {
        return;
    }
    for dir in nearest_slope_dirs(world, pos, kind) {
        place(world, pos + dir, kind, level, changed);
    }
}

fn nearest_slope_dirs(world: &World, pos: IVec3, kind: FluidKind) -> Vec<IVec3> {
    let mut best = u32::MAX;
    let mut dirs = Vec::new();
    for dir in SIDES {
        let next = pos + dir;
        if probe(world, next, kind) != Cell::Open {
            continue;
        }
        let distance = if is_hole(world, next, kind) {
            0
        } else {
            slope_distance(world, next, kind, 1, -dir)
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

fn slope_distance(world: &World, pos: IVec3, kind: FluidKind, depth: u32, from: IVec3) -> u32 {
    let mut best = NO_SLOPE;
    for dir in SIDES {
        if dir == from {
            continue;
        }
        let next = pos + dir;
        let passable = match probe(world, next, kind) {
            Cell::Open => true,
            Cell::Own(data) => data != FLUID_SOURCE,
            Cell::Solid | Cell::Foreign => false,
        };
        if !passable {
            continue;
        }
        if is_hole(world, next, kind) {
            return depth;
        }
        if depth < params(kind).slope_distance {
            best = best.min(slope_distance(world, next, kind, depth + 1, -dir));
        }
    }
    best
}

fn is_hole(world: &World, pos: IVec3, kind: FluidKind) -> bool {
    matches!(
        probe(world, pos - IVec3::Y, kind),
        Cell::Open | Cell::Own(_)
    )
}

#[cfg(test)]
mod tests;
