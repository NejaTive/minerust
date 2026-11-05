use glam::IVec3;

use crate::block::{Block, FARMLAND_WET, FLUID_SOURCE, VINE_TOP};
use crate::column::{COLUMN_SECTIONS, WORLD_HEIGHT};
use crate::light::{LightMap, MAX_LIGHT};
use crate::pos::ColumnPos;
use crate::section::{SECTION_SIZE, SECTION_VOLUME};
use crate::terrain::Terrain;
use crate::world::World;

pub const SIMULATION_RADIUS: i32 = 8;
pub const RANDOM_TICKS_PER_SECTION: usize = 3;
pub const STEP_TICKS: u32 = 3;
pub const SPREAD_LIGHT: u8 = 9;
pub const SPREAD_ATTEMPTS: usize = 4;
pub const SNOW_MELT_LIGHT: u8 = 12;
pub const ICE_MELT_LIGHT: u8 = 9;
pub const FREEZE_LIGHT: u8 = 10;
pub const WEATHER_CHANCE: usize = 16;
pub const STACK_MAX_HEIGHT: i32 = 3;
pub const STACK_GROWTH_PERIOD: usize = 16;
pub const VINE_GROWTH_PERIOD: usize = 4;
pub const WATER_REACH: i32 = 4;
pub const CROP_MIN_LIGHT: u8 = 8;
pub const CROP_GROW_LIGHT: u8 = 9;
pub const CROP_BASE_CHANCE: f32 = 25.0;
pub const BEETROOT_SKIP_PERIOD: usize = 3;

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
        terrain: &Terrain,
        center: ColumnPos,
        view_radius: i32,
        precipitation: bool,
    ) -> Ticked {
        let radius = SIMULATION_RADIUS.min(view_radius);
        let mut ticked = Ticked::default();
        for z in center.z() - radius..=center.z() + radius {
            for x in center.x() - radius..=center.x() + radius {
                let key = ColumnPos::new(x, z);
                if world.column(key).is_some() {
                    for _ in 0..STEP_TICKS {
                        if self.below(WEATHER_CHANCE) == 0 {
                            let (dx, dz) = (self.below(16) as i32, self.below(16) as i32);
                            let column = key.origin() + IVec3::new(dx, 0, dz);
                            self.weather_tick(
                                world,
                                light,
                                terrain,
                                column,
                                precipitation,
                                &mut ticked,
                            );
                        }
                    }
                }
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
                        self.tick(world, light, terrain, pos, precipitation, &mut ticked);
                    }
                }
            }
        }
        ticked
    }

    fn tick(
        &mut self,
        world: &mut World,
        light: &LightMap,
        terrain: &Terrain,
        pos: IVec3,
        precipitation: bool,
        ticked: &mut Ticked,
    ) {
        let above = pos + IVec3::Y;
        let covered = covers(world.block(above));
        let old = world.block(pos);
        let new = match old {
            Block::Grass | Block::SnowyGrass | Block::Mycelium if covered => Block::Dirt,
            Block::Dirt if !covered && lit(light, above) => match self.spread_source(world, pos) {
                Some(grass) => grass,
                None => return,
            },
            Block::SnowyGrass if !snow_above(world.block(above)) => Block::Grass,
            Block::Ice if light.block(pos) >= ICE_MELT_LIGHT => Block::Water,
            Block::Snow if light.block(pos) >= SNOW_MELT_LIGHT => {
                self.unsnow(world, pos - IVec3::Y, ticked);
                Block::Air
            }
            Block::Cactus | Block::Reeds => return self.grow_stack(world, pos, old, ticked),
            Block::Vine => return self.grow_vine(world, pos, ticked),
            Block::Farmland => {
                return self.tick_farmland(world, light, terrain, pos, precipitation, ticked);
            }
            crop if crop.is_crop() => return self.tick_crop(world, light, pos, crop, ticked),
            _ => return,
        };
        let changed = world.set_block_supported(pos, new);
        if old.lights_like(new) {
            ticked.light_neutral.push(pos);
        }
        ticked.changed.extend(changed);
    }

    fn weather_tick(
        &mut self,
        world: &mut World,
        light: &LightMap,
        terrain: &Terrain,
        column: IVec3,
        precipitation: bool,
        ticked: &mut Ticked,
    ) {
        let Some(top) = (0..WORLD_HEIGHT).rev().find(|&y| {
            let block = world.block(IVec3::new(column.x, y, column.z));
            block.is_solid() || block.fluid().is_some()
        }) else {
            return;
        };
        let pos = IVec3::new(column.x, top + 1, column.z);
        let below = pos - IVec3::Y;
        if light.block(pos) >= FREEZE_LIGHT {
            return;
        }
        if world.block(below) == Block::Water
            && world.block_data(below) == FLUID_SOURCE
            && terrain.freezes_at(below.x, below.y, below.z)
        {
            self.replace(world, below, Block::Ice, ticked);
        } else if precipitation
            && world.block(pos) == Block::Air
            && terrain.freezes_at(pos.x, pos.y, pos.z)
            && world.can_place(pos, Block::Snow)
        {
            if world.block(below) == Block::Grass {
                self.replace(world, below, Block::SnowyGrass, ticked);
            }
            self.replace(world, pos, Block::Snow, ticked);
        }
    }

    fn unsnow(&mut self, world: &mut World, pos: IVec3, ticked: &mut Ticked) {
        if world.block(pos) == Block::SnowyGrass {
            self.replace(world, pos, Block::Grass, ticked);
        }
    }

    fn replace(&mut self, world: &mut World, pos: IVec3, new: Block, ticked: &mut Ticked) {
        let old = world.block(pos);
        let changed = world.set_block_supported(pos, new);
        if old.lights_like(new) {
            ticked.light_neutral.push(pos);
        }
        ticked.changed.extend(changed);
    }

    fn grow_stack(&mut self, world: &mut World, pos: IVec3, block: Block, ticked: &mut Ticked) {
        let top = pos + IVec3::Y;
        let height = (0..STACK_MAX_HEIGHT)
            .take_while(|&d| world.block(pos - IVec3::Y * d) == block)
            .count() as i32;
        if height >= STACK_MAX_HEIGHT
            || world.block(top) != Block::Air
            || self.below(STACK_GROWTH_PERIOD) != 0
            || !world.can_place(top, block)
        {
            return;
        }
        let changed = world.set_block_supported(top, block);
        if !changed.is_empty() {
            ticked.light_neutral.push(top);
        }
        ticked.changed.extend(changed);
    }

    fn grow_vine(&mut self, world: &mut World, pos: IVec3, ticked: &mut Ticked) {
        let below = pos - IVec3::Y;
        if self.below(VINE_GROWTH_PERIOD) != 0 || world.block(below) != Block::Air {
            return;
        }
        let sides = world.block_data(pos) & !VINE_TOP;
        let data = world.vine_sides(below, sides);
        if data == 0 {
            return;
        }
        let changed = world.set_block_supported_with_data(below, Block::Vine, data);
        if !changed.is_empty() {
            ticked.light_neutral.push(below);
        }
        ticked.changed.extend(changed);
    }

    fn tick_farmland(
        &mut self,
        world: &mut World,
        light: &LightMap,
        terrain: &Terrain,
        pos: IVec3,
        precipitation: bool,
        ticked: &mut Ticked,
    ) {
        let moisture = world.block_data(pos);
        let above = pos + IVec3::Y;
        let wet =
            has_water_near(world, pos) || rains_at(world, light, terrain, above, precipitation);
        if wet {
            if moisture < FARMLAND_WET {
                self.replace_with_data(world, pos, Block::Farmland, FARMLAND_WET, ticked);
            }
        } else if moisture > 0 {
            self.replace_with_data(world, pos, Block::Farmland, moisture - 1, ticked);
        } else if !world.block(above).is_crop() {
            self.replace(world, pos, Block::Dirt, ticked);
        }
    }

    fn tick_crop(
        &mut self,
        world: &mut World,
        light: &LightMap,
        pos: IVec3,
        crop: Block,
        ticked: &mut Ticked,
    ) {
        if light.sky(pos).max(light.block(pos)) < CROP_MIN_LIGHT {
            return self.replace(world, pos, Block::Air, ticked);
        }
        if crop == Block::Beetroots && self.below(BEETROOT_SKIP_PERIOD) == 0 {
            return;
        }
        let age = world.block_data(pos);
        if age >= crop.crop_max_age()
            || light.sky(pos + IVec3::Y).max(light.block(pos + IVec3::Y)) < CROP_GROW_LIGHT
        {
            return;
        }
        let period = (CROP_BASE_CHANCE / growth_chance(world, pos, crop)) as usize + 1;
        if self.below(period) == 0 {
            self.replace_with_data(world, pos, crop, age + 1, ticked);
        }
    }

    fn replace_with_data(
        &mut self,
        world: &mut World,
        pos: IVec3,
        new: Block,
        data: u8,
        ticked: &mut Ticked,
    ) {
        let changed = world.set_block_supported_with_data(pos, new, data);
        if !changed.is_empty() {
            ticked.light_neutral.push(pos);
        }
        ticked.changed.extend(changed);
    }

    fn spread_source(&mut self, world: &World, pos: IVec3) -> Option<Block> {
        (0..SPREAD_ATTEMPTS).find_map(|_| {
            let source = pos + self.offset();
            let block = world.block(source);
            ((block.is_grass() || block == Block::Mycelium)
                && !covers(world.block(source + IVec3::Y)))
            .then_some(if block.is_grass() {
                Block::Grass
            } else {
                block
            })
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

fn has_water_near(world: &World, pos: IVec3) -> bool {
    (-WATER_REACH..=WATER_REACH).any(|dz| {
        (-WATER_REACH..=WATER_REACH)
            .any(|dx| (0..=1).any(|dy| world.block(pos + IVec3::new(dx, dy, dz)).is_water()))
    })
}

fn rains_at(
    world: &World,
    light: &LightMap,
    terrain: &Terrain,
    pos: IVec3,
    precipitation: bool,
) -> bool {
    precipitation
        && world.block(pos).fluid().is_none()
        && light.sky(pos) == MAX_LIGHT
        && !terrain.biome_at(pos.x, pos.z).is_desert()
        && !terrain.freezes_at(pos.x, pos.y, pos.z)
}

/// `BlockCrops.getGrowthChance`: сухая грядка даёт 1, влажная 3, соседние в четверть; одинаковые культуры рядом делят шанс пополам.
fn growth_chance(world: &World, pos: IVec3, crop: Block) -> f32 {
    let soil = pos - IVec3::Y;
    let mut chance = 1.0;
    for dz in -1..=1 {
        for dx in -1..=1 {
            let at = soil + IVec3::new(dx, 0, dz);
            let mut share = match (world.block(at), world.block_data(at)) {
                (Block::Farmland, 0) => 1.0,
                (Block::Farmland, _) => 3.0,
                _ => 0.0,
            };
            if dx != 0 || dz != 0 {
                share /= 4.0;
            }
            chance += share;
        }
    }
    let same = |dx: i32, dz: i32| world.block(pos + IVec3::new(dx, 0, dz)) == crop;
    let in_row = (same(-1, 0) || same(1, 0)) && (same(0, -1) || same(0, 1));
    let diagonal = same(-1, -1) || same(1, -1) || same(1, 1) || same(-1, 1);
    if in_row || diagonal {
        chance / 2.0
    } else {
        chance
    }
}

fn snow_above(block: Block) -> bool {
    matches!(block, Block::Snow | Block::SnowBlock)
}

fn covers(block: Block) -> bool {
    block.is_opaque() || block.fluid().is_some()
}

fn lit(light: &LightMap, pos: IVec3) -> bool {
    light.sky(pos).max(light.block(pos)) >= SPREAD_LIGHT
}

#[cfg(test)]
mod tests;
