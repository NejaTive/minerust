use super::*;
use crate::stream::Stream;

const SALT_DECOR: u64 = 33;
const REED_ATTEMPTS: u32 = 10;
const CACTUS_TRIES: u32 = 10;
const CACTUS_SPREAD: u32 = 8;
const CACTUS_SURFACE_REACH: u32 = 8;
const BUSH_TRIES: u32 = 4;
const BUSH_SPREAD: u32 = 8;
const BUSH_SKY_REACH: u32 = 32;
const REED_TRIES: u32 = 20;
const REED_SPREAD: u32 = 4;
const CENTER: i32 = 8;

const HORIZONTAL: [IVec3; 4] = [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z];

struct Counts {
    cacti: u32,
    bushes: u32,
    reeds: u32,
}

fn counts(biome: Biome) -> Counts {
    if biome.is_desert() {
        Counts {
            cacti: 10,
            bushes: 2,
            reeds: 50,
        }
    } else if biome.is_mesa() {
        Counts {
            cacti: 5,
            bushes: 20,
            reeds: 3,
        }
    } else {
        Counts {
            cacti: 0,
            bushes: 0,
            reeds: 0,
        }
    }
}

struct Spot {
    x: i32,
    z: i32,
}

impl Terrain {
    pub(crate) fn decorate(&self, key: ColumnPos, column: &mut Column) {
        if self.world_type != WorldType::Standard {
            return;
        }
        let climate = self.climate(key.x(), key.z());
        let biome = climate.biomes[ColumnClimate::index(CENTER, CENTER)];
        let counts = counts(biome);
        let state = u64::from(hash(self.seed, key.x(), key.z(), SALT_DECOR))
            | u64::from(hash(self.seed, key.x(), key.z(), SALT_DECOR + 1)) << 32;
        let mut rng = Stream::new(state);
        let surface = |spot: &Spot| climate.surface[ColumnClimate::index(spot.x, spot.z)];
        let pick = |rng: &mut Stream| Spot {
            x: rng.below(SECTION_SIZE as u32) as i32,
            z: rng.below(SECTION_SIZE as u32) as i32,
        };
        let near = |rng: &mut Stream, spot: &Spot, spread: u32| {
            let at = Spot {
                x: spot.x + rng.spread(spread),
                z: spot.z + rng.spread(spread),
            };
            (0..SECTION_SIZE)
                .contains(&at.x)
                .then_some(())
                .filter(|()| (0..SECTION_SIZE).contains(&at.z))
                .map(|()| at)
        };

        for _ in 0..counts.cacti {
            let spot = pick(&mut rng);
            let reach = 2 * surface(&spot) as u32;
            if !rng.chance(CACTUS_SURFACE_REACH, reach) {
                continue;
            }
            for _ in 0..CACTUS_TRIES {
                let height = 1 + rng.taper() as i32;
                if let Some(at) = near(&mut rng, &spot, CACTUS_SPREAD) {
                    self.plant_cactus(key, column, &climate, &at, height);
                }
            }
        }
        for _ in 0..counts.bushes {
            let spot = pick(&mut rng);
            if !rng.chance(BUSH_SKY_REACH, surface(&spot) as u32 + BUSH_SKY_REACH) {
                continue;
            }
            for _ in 0..BUSH_TRIES {
                if let Some(at) = near(&mut rng, &spot, BUSH_SPREAD) {
                    let ground = IVec3::new(at.x, surface(&at), at.z);
                    let above = ground + IVec3::Y;
                    if Block::DeadBush.stands_on(column.get(ground))
                        && column.get(above) == Block::Air
                    {
                        column.set(above, Block::DeadBush);
                    }
                }
            }
        }
        for _ in 0..REED_ATTEMPTS + counts.reeds {
            let spot = pick(&mut rng);
            if !rng.chance(1, 2 * surface(&spot) as u32) {
                continue;
            }
            for _ in 0..REED_TRIES {
                let height = 2 + rng.taper() as i32;
                if let Some(at) = near(&mut rng, &spot, REED_SPREAD) {
                    self.plant_reeds(key, column, &climate, &at, height);
                }
            }
        }
    }

    fn plant_cactus(
        &self,
        key: ColumnPos,
        column: &mut Column,
        climate: &ColumnClimate,
        at: &Spot,
        height: i32,
    ) {
        let ground = IVec3::new(
            at.x,
            climate.surface[ColumnClimate::index(at.x, at.z)],
            at.z,
        );
        if !column.get(ground).is_sand() {
            return;
        }
        for y in ground.y + 1..ground.y + 1 + height {
            let pos = IVec3::new(at.x, y, at.z);
            if y >= WORLD_HEIGHT
                || column.get(pos) != Block::Air
                || self.solid_beside(key, column, pos)
            {
                return;
            }
            column.set(pos, Block::Cactus);
        }
    }

    fn solid_beside(&self, key: ColumnPos, column: &Column, pos: IVec3) -> bool {
        HORIZONTAL.iter().any(|&dir| {
            let side = pos + dir;
            if Column::contains(side) {
                return column.get(side).is_solid();
            }
            let origin = key.origin();
            self.surface_height(origin.x + side.x, origin.z + side.z) >= pos.y
        })
    }

    fn plant_reeds(
        &self,
        key: ColumnPos,
        column: &mut Column,
        climate: &ColumnClimate,
        at: &Spot,
        height: i32,
    ) {
        if climate.surface[ColumnClimate::index(at.x, at.z)] != WATER_LEVEL {
            return;
        }
        let ground = IVec3::new(at.x, WATER_LEVEL, at.z);
        let base = ground + IVec3::Y;
        if !Block::Reeds.stands_on(column.get(ground))
            || column.get(base) != Block::Air
            || !self.water_beside(key, column, ground)
        {
            return;
        }
        for y in base.y..base.y + height {
            let pos = IVec3::new(at.x, y, at.z);
            if column.get(pos) != Block::Air {
                return;
            }
            column.set(pos, Block::Reeds);
        }
    }

    fn water_beside(&self, key: ColumnPos, column: &Column, pos: IVec3) -> bool {
        HORIZONTAL.iter().any(|&dir| {
            let side = pos + dir;
            if Column::contains(side) {
                return column.get(side) == Block::Water;
            }
            let (x, z) = (key.origin().x + side.x, key.origin().z + side.z);
            self.surface_height(x, z) < WATER_LEVEL && !self.biome_at(x, z).is_snowy()
        })
    }
}

#[cfg(test)]
mod tests;
