use glam::IVec3;

use crate::block::Block;
use crate::column::COLUMN_SECTIONS;
use crate::light::LightMap;
use crate::pos::ColumnPos;
use crate::section::{SECTION_SIZE, SECTION_VOLUME};
use crate::world::World;

pub const SIMULATION_RADIUS: i32 = 8;
pub const RANDOM_TICKS_PER_SECTION: usize = 3;
pub const STEP_TICKS: u32 = 3;
pub const SPREAD_LIGHT: u8 = 9;
pub const SPREAD_ATTEMPTS: usize = 4;
pub const SNOW_MELT_LIGHT: u8 = 12;

const SPREAD_REACH: i32 = 1;
const SPREAD_BELOW: i32 = 3;
const SPREAD_ABOVE: i32 = 1;

#[derive(Default)]
pub struct Ticked {
    pub changed: Vec<IVec3>,
    pub light_neutral: Vec<IVec3>,
}

pub struct RandomTicks {
    state: u64,
}

impl RandomTicks {
    pub fn new(seed: i32) -> Self {
        Self {
            state: u64::from(seed as u32) ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    pub fn step(
        &mut self,
        world: &mut World,
        light: &LightMap,
        center: ColumnPos,
        view_radius: i32,
    ) -> Ticked {
        let radius = SIMULATION_RADIUS.min(view_radius);
        let mut ticked = Ticked::default();
        for z in center.z() - radius..=center.z() + radius {
            for x in center.x() - radius..=center.x() + radius {
                let key = ColumnPos::new(x, z);
                for sy in 0..COLUMN_SECTIONS {
                    let Some(column) = world.column(key) else {
                        break;
                    };
                    if !column.section(sy).has_random_ticks() {
                        continue;
                    }
                    for _ in 0..RANDOM_TICKS_PER_SECTION {
                        let index = self.below(SECTION_VOLUME) as i32;
                        let pos = key.origin()
                            + IVec3::new(
                                index % SECTION_SIZE,
                                sy * SECTION_SIZE + index / (SECTION_SIZE * SECTION_SIZE),
                                index / SECTION_SIZE % SECTION_SIZE,
                            );
                        self.tick(world, light, pos, &mut ticked);
                    }
                }
            }
        }
        ticked
    }

    fn tick(&mut self, world: &mut World, light: &LightMap, pos: IVec3, ticked: &mut Ticked) {
        let above = pos + IVec3::Y;
        let covered = covers(world.block(above));
        let old = world.block(pos);
        let new = match old {
            Block::Grass | Block::SnowyGrass if covered => Block::Dirt,
            Block::Dirt if !covered && lit(light, above) => match self.spread_source(world, pos) {
                Some(grass) => grass,
                None => return,
            },
            Block::Snow if light.block(pos) >= SNOW_MELT_LIGHT => Block::Air,
            _ => return,
        };
        let changed = world.set_block_supported(pos, new);
        if old.lights_like(new) {
            ticked.light_neutral.push(pos);
        }
        ticked.changed.extend(changed);
    }

    fn spread_source(&mut self, world: &World, pos: IVec3) -> Option<Block> {
        (0..SPREAD_ATTEMPTS).find_map(|_| {
            let source = pos + self.offset();
            let block = world.block(source);
            (block.is_grass() && !covers(world.block(source + IVec3::Y))).then_some(block)
        })
    }

    fn offset(&mut self) -> IVec3 {
        let side = 2 * SPREAD_REACH as usize + 1;
        let height = (SPREAD_BELOW + SPREAD_ABOVE + 1) as usize;
        IVec3::new(
            self.below(side) as i32 - SPREAD_REACH,
            self.below(height) as i32 - SPREAD_BELOW,
            self.below(side) as i32 - SPREAD_REACH,
        )
    }

    fn below(&mut self, bound: usize) -> usize {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        (self.state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 33) as usize % bound
    }
}

fn covers(block: Block) -> bool {
    block.is_opaque() || block.fluid().is_some()
}

fn lit(light: &LightMap, pos: IVec3) -> bool {
    light.sky(pos).max(light.block(pos)) >= SPREAD_LIGHT
}

#[cfg(test)]
mod tests;
