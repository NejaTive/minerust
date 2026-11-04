use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use glam::IVec3;

use crate::block::Block;
use crate::column::{Column, WORLD_HEIGHT};
use crate::pos::ColumnPos;
use crate::section::SECTION_SIZE;

use self::plan::{Plan, RANGE, Segment};

pub const LAVA_LEVEL: i32 = 11;
const ROOF: i32 = 3;
const MIN_Y: i32 = 1;
const MAX_Y: i32 = 248;
const PLAN_CACHE_SOURCES: usize = 8192;
const COLUMN_AREA: usize = (SECTION_SIZE * SECTION_SIZE) as usize;

pub struct Caves {
    seed: i32,
    plans: Mutex<HashMap<(i32, i32), Arc<Plan>>>,
}

#[derive(Clone, Copy)]
enum Shape<'a> {
    Tunnel,
    Ravine(&'a [f32; 256]),
}

impl Caves {
    pub fn new(seed: i32) -> Self {
        Self {
            seed,
            plans: Mutex::new(HashMap::new()),
        }
    }

    /// `MapGenBase.generate`: туннели и овраги всех колонок в радиусе `RANGE`, затем овраги.
    pub fn carve(
        &self,
        key: ColumnPos,
        column: &mut Column,
        surface: impl Fn(i32, i32) -> i32,
        has_tree: impl Fn(i32, i32) -> bool,
    ) {
        let plans: Vec<Arc<Plan>> = (key.x() - RANGE..=key.x() + RANGE)
            .flat_map(|x| (key.z() - RANGE..=key.z() + RANGE).map(move |z| (x, z)))
            .map(|source| self.plan(source))
            .collect();
        let origin = key.origin();
        let mut digger = Digger {
            column,
            origin,
            surface,
            has_tree,
            heights: [None; COLUMN_AREA],
            trees: [None; COLUMN_AREA],
        };
        let (cx, cz) = (f64::from(origin.x + 8), f64::from(origin.z + 8));
        for plan in &plans {
            let Some(bounds) = plan.bounds else { continue };
            if cx < bounds.min_x || cx > bounds.max_x || cz < bounds.min_z || cz > bounds.max_z {
                continue;
            }
            digger.dig_caves(&plan.caves);
        }
        for plan in &plans {
            for ravine in &plan.ravines {
                for segment in &ravine.segments {
                    digger.dig_segment(segment, Shape::Ravine(&ravine.profile));
                }
            }
        }
    }

    fn plan(&self, source: (i32, i32)) -> Arc<Plan> {
        if let Some(found) = self.plans.lock().expect("plan cache poisoned").get(&source) {
            return Arc::clone(found);
        }
        let computed = Arc::new(Plan::new(self.seed, source));
        let mut cache = self.plans.lock().expect("plan cache poisoned");
        if cache.len() >= PLAN_CACHE_SOURCES {
            cache.clear();
        }
        cache.insert(source, Arc::clone(&computed));
        computed
    }
}

struct Digger<'a, S, T> {
    column: &'a mut Column,
    origin: IVec3,
    surface: S,
    has_tree: T,
    heights: [Option<i32>; COLUMN_AREA],
    trees: [Option<bool>; COLUMN_AREA],
}

impl<S: Fn(i32, i32) -> i32, T: Fn(i32, i32) -> bool> Digger<'_, S, T> {
    fn dig_caves(&mut self, segments: &[Segment]) {
        let mut finished_rooms = 0u32;
        for segment in segments {
            if segment.room != 0 && finished_rooms >> segment.room & 1 == 1 {
                continue;
            }
            if self.dig_segment(segment, Shape::Tunnel) && segment.room != 0 {
                finished_rooms |= 1 << segment.room;
            }
        }
    }

    /// Шаг `addTunnel`; `true`, если шаг попал в зону колонки и не упёрся в воду.
    fn dig_segment(&mut self, segment: &Segment, shape: Shape) -> bool {
        let (center, width, height) = (segment.center, segment.width, segment.height);
        let reach = segment.reach();
        let (cx, cz) = (f64::from(self.origin.x + 8), f64::from(self.origin.z + 8));
        if (center.x - cx).abs() > reach || (center.z - cz).abs() > reach {
            return false;
        }
        let local = |value: f64, radius: f64, base: i32| {
            (
                ((value - radius).floor() as i32 - base - 1).max(0),
                ((value + radius).floor() as i32 - base + 1).min(SECTION_SIZE),
            )
        };
        let (x_low, x_high) = local(center.x, width, self.origin.x);
        let (z_low, z_high) = local(center.z, width, self.origin.z);
        let y_low = ((center.y - height).floor() as i32 - 1).max(MIN_Y);
        let y_high = ((center.y + height).floor() as i32 + 1).min(MAX_Y);
        if self.touches_water((x_low, x_high), (z_low, z_high), (y_low, y_high)) {
            return false;
        }
        for x in x_low..x_high {
            let dx = (f64::from(self.origin.x + x) + 0.5 - center.x) / width;
            for z in z_low..z_high {
                let dz = (f64::from(self.origin.z + z) + 0.5 - center.z) / width;
                let flat = dx * dx + dz * dz;
                if flat >= 1.0 {
                    continue;
                }
                let mut top = None;
                for y in (y_low + 1..=y_high).rev() {
                    let dy = (f64::from(y - 1) + 0.5 - center.y) / height;
                    let inside = match shape {
                        Shape::Tunnel => dy > -0.7 && flat + dy * dy < 1.0,
                        Shape::Ravine(profile) => {
                            flat * f64::from(profile[(y - 1) as usize]) + dy * dy / 6.0 < 1.0
                        }
                    };
                    if inside {
                        self.dig(IVec3::new(x, y, z), &mut top);
                    }
                }
            }
        }
        true
    }

    /// `isOceanBlock` по оболочке объёма: внутри берутся только верхний и нижний слои.
    fn touches_water(&self, x: (i32, i32), z: (i32, i32), y: (i32, i32)) -> bool {
        let (y_low, y_high) = y;
        for px in x.0..x.1 {
            for pz in z.0..z.1 {
                let edge = px == x.0 || px == x.1 - 1 || pz == z.0 || pz == z.1 - 1;
                let mut py = y_high + 1;
                while py >= y_low - 1 {
                    if (0..WORLD_HEIGHT).contains(&py)
                        && self.column.get(IVec3::new(px, py, pz)) == Block::Water
                    {
                        return true;
                    }
                    py = if edge || py == y_low - 1 {
                        py - 1
                    } else {
                        y_low - 1
                    };
                }
            }
        }
        false
    }

    /// `digBlock`: лава ниже `LAVA_LEVEL`; под вырезанной травой нижняя земля становится травой.
    fn dig(&mut self, at: IVec3, top: &mut Option<Block>) {
        let block = self.column.get(at);
        top_block(block).inspect(|&found| {
            top.get_or_insert(found);
        });
        let above = at + IVec3::Y;
        if !replaceable(block, self.column.get(above))
            || self.next_to_water(at)
            || self.under_tree(at)
        {
            return;
        }
        if at.y < LAVA_LEVEL {
            self.column.set(at, Block::Lava);
            return;
        }
        self.column.set(at, Block::Air);
        if self.column.get(above) == Block::Snow {
            self.column.set(above, Block::Air);
        }
        let below = at - IVec3::Y;
        if let Some(top) = *top
            && matches!(self.column.get(below), Block::Dirt | Block::CoarseDirt)
        {
            self.column.set(below, top);
        }
    }

    /// Оболочка `touches_water` не видит воду внутри объёма и за краем колонки.
    fn next_to_water(&self, at: IVec3) -> bool {
        [IVec3::Y, IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z]
            .into_iter()
            .map(|d| at + d)
            .any(|n| {
                (0..SECTION_SIZE).contains(&n.x)
                    && (0..SECTION_SIZE).contains(&n.z)
                    && self.column.get(n) == Block::Water
            })
    }

    /// Крыша под деревом остаётся: деревья сажаются по рельефу до пещер.
    fn under_tree(&mut self, at: IVec3) -> bool {
        let index = (at.z * SECTION_SIZE + at.x) as usize;
        let (x, z) = (self.origin.x + at.x, self.origin.z + at.z);
        let height = *self.heights[index].get_or_insert_with(|| (self.surface)(x, z));
        at.y > height - ROOF && *self.trees[index].get_or_insert_with(|| (self.has_tree)(x, z))
    }
}

fn top_block(block: Block) -> Option<Block> {
    match block {
        Block::Grass | Block::Mycelium | Block::Podzol => Some(block),
        Block::SnowyGrass => Some(Block::Grass),
        _ => None,
    }
}

fn replaceable(block: Block, above: Block) -> bool {
    match block {
        Block::Sand | Block::RedSand | Block::Gravel => above != Block::Water,
        _ => {
            block.is_stone()
                || matches!(
                    block,
                    Block::Dirt
                        | Block::CoarseDirt
                        | Block::Podzol
                        | Block::Grass
                        | Block::SnowyGrass
                        | Block::Mycelium
                        | Block::Snow
                        | Block::Sandstone
                        | Block::RedSandstone
                        | Block::Terracotta
                        | Block::WhiteTerracotta
                        | Block::OrangeTerracotta
                        | Block::YellowTerracotta
                        | Block::BrownTerracotta
                        | Block::RedTerracotta
                        | Block::SilverTerracotta
                )
        }
    }
}

mod plan;

#[cfg(test)]
mod tests;
