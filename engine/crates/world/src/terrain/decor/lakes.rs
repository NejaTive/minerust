use super::*;
use crate::block::FluidKind;

const SALT_LAKES: u64 = 80;
const WATER_ONE_IN: u32 = 4;
const LAVA_ONE_IN: u32 = 8;
const LAVA_SURFACE_ONE_IN: u32 = 10;
const LAVA_HEIGHT_BASE: u32 = 248;
const LAVA_HEIGHT_MIN: u32 = 8;
const BOX_XZ: i32 = 16;
const BOX_Y: i32 = 8;
const WATER_LAYERS: i32 = 4;
const MIN_ELLIPSOIDS: u32 = 4;
const ELLIPSOID_SPAN: u32 = 4;
const FLOOR_LIMIT: i32 = 5;
const SINK: i32 = 4;

type Mask = [bool; (BOX_XZ * BOX_XZ * BOX_Y) as usize];

fn slot(i: i32, j: i32, k: i32) -> usize {
    ((i * BOX_XZ + j) * BOX_Y + k) as usize
}

struct Ground {
    columns: [Arc<ColumnClimate>; 4],
    column_x: i32,
    column_z: i32,
}

impl Ground {
    fn new(terrain: &Terrain, column_x: i32, column_z: i32) -> Self {
        Self {
            columns: [(0, 0), (1, 0), (0, 1), (1, 1)]
                .map(|(dx, dz)| terrain.climate(column_x + dx, column_z + dz)),
            column_x,
            column_z,
        }
    }

    fn block(&self, x: i32, y: i32, z: i32) -> Block {
        let which = ((z >> 4) - self.column_z) * 2 + ((x >> 4) - self.column_x);
        relief_block(&self.columns[which as usize], x, y, z)
    }
}

pub(super) fn relief_block(climate: &ColumnClimate, x: i32, y: i32, z: i32) -> Block {
    if climate.solid[ColumnClimate::index(x, z)].get(y) {
        Block::Stone
    } else if y <= WATER_LEVEL {
        Block::Water
    } else {
        Block::Air
    }
}

fn ellipsoids(rng: &mut Stream) -> Mask {
    let mut mask = [false; (BOX_XZ * BOX_XZ * BOX_Y) as usize];
    let (xz, y) = (f64::from(BOX_XZ), f64::from(BOX_Y));
    for _ in 0..rng.below(ELLIPSOID_SPAN) + MIN_ELLIPSOIDS {
        let sx = rng.fraction() * 6.0 + 3.0;
        let sy = rng.fraction() * 4.0 + 2.0;
        let sz = rng.fraction() * 6.0 + 3.0;
        let cx = rng.fraction() * (xz - sx - 2.0) + 1.0 + sx / 2.0;
        let cy = rng.fraction() * (y - sy - 4.0) + 2.0 + sy / 2.0;
        let cz = rng.fraction() * (xz - sz - 2.0) + 1.0 + sz / 2.0;
        for i in 1..BOX_XZ - 1 {
            for j in 1..BOX_XZ - 1 {
                for k in 1..BOX_Y - 1 {
                    let dx = (f64::from(i) - cx) / (sx / 2.0);
                    let dy = (f64::from(k) - cy) / (sy / 2.0);
                    let dz = (f64::from(j) - cz) / (sz / 2.0);
                    if dx * dx + dy * dy + dz * dz < 1.0 {
                        mask[slot(i, j, k)] = true;
                    }
                }
            }
        }
    }
    mask
}

fn touches_mask(mask: &Mask, i: i32, j: i32, k: i32) -> bool {
    let at = |di: i32, dj: i32, dk: i32| {
        let (i, j, k) = (i + di, j + dj, k + dk);
        (0..BOX_XZ).contains(&i)
            && (0..BOX_XZ).contains(&j)
            && (0..BOX_Y).contains(&k)
            && mask[slot(i, j, k)]
    };
    at(1, 0, 0) || at(-1, 0, 0) || at(0, 1, 0) || at(0, -1, 0) || at(0, 0, 1) || at(0, 0, -1)
}

fn walls_hold(mask: &Mask, kind: FluidKind, block: impl Fn(i32, i32, i32) -> Block) -> bool {
    for i in 0..BOX_XZ {
        for j in 0..BOX_XZ {
            for k in 0..BOX_Y {
                if mask[slot(i, j, k)] || !touches_mask(mask, i, j, k) {
                    continue;
                }
                let found = block(i, j, k);
                if k >= WATER_LAYERS && found.fluid().is_some() {
                    return false;
                }
                if k < WATER_LAYERS && !found.is_solid() && found != kind.block() {
                    return false;
                }
            }
        }
    }
    true
}

pub(in crate::terrain) struct Lake {
    pub(super) kind: FluidKind,
    ox: i32,
    oz: i32,
    pub(super) y0: i32,
    mask: Mask,
    footprint: [bool; (BOX_XZ * BOX_XZ) as usize],
}

impl Lake {
    #[cfg(test)]
    pub(super) fn liquid_cells(&self) -> Vec<IVec3> {
        let mut cells = Vec::new();
        for i in 0..BOX_XZ {
            for j in 0..BOX_XZ {
                for k in 0..WATER_LAYERS {
                    if self.mask[slot(i, j, k)] {
                        cells.push(IVec3::new(self.ox + i, self.y0 + k, self.oz + j));
                    }
                }
            }
        }
        cells
    }

    fn covers(&self, x: i32, z: i32) -> bool {
        let (i, j) = (x - self.ox, z - self.oz);
        (0..BOX_XZ).contains(&i)
            && (0..BOX_XZ).contains(&j)
            && self.footprint[(i * BOX_XZ + j) as usize]
    }
}

impl Terrain {
    pub(crate) fn lake_covers(&self, x: i32, z: i32) -> bool {
        ((z >> 4) - 1..=(z >> 4)).any(|sz| {
            ((x >> 4) - 1..=(x >> 4))
                .any(|sx| self.lakes_of(sx, sz).iter().any(|lake| lake.covers(x, z)))
        })
    }

    pub(super) fn lakes_of(&self, sx: i32, sz: i32) -> Arc<Vec<Lake>> {
        if let Some(found) = self
            .lakes
            .lock()
            .expect("lake cache poisoned")
            .get(&(sx, sz))
        {
            return Arc::clone(found);
        }
        let near_village = (sz..=sz + 1)
            .any(|z| (sx..=sx + 1).any(|x| self.village_touches(ColumnPos::new(x, z))));
        let planned = Arc::new(if near_village {
            Vec::new()
        } else {
            self.plan_lakes(sx, sz)
        });
        let mut cache = self.lakes.lock().expect("lake cache poisoned");
        if cache.len() >= CLIMATE_CACHE_COLUMNS {
            cache.clear();
        }
        cache.insert((sx, sz), Arc::clone(&planned));
        planned
    }

    /// `WorldGenLakes`: озеро смещено на +8 и занимает коробку 16×8×16, поэтому колонку задевают озёра четырёх колонок.
    fn plan_lakes(&self, sx: i32, sz: i32) -> Vec<Lake> {
        let state = u64::from(hash(self.seed, sx, sz, SALT_LAKES))
            | u64::from(hash(self.seed, sx, sz, SALT_LAKES + 1)) << 32;
        let mut rng = Stream::new(state);
        let mut lakes = Vec::new();
        let dry = matches!(
            self.climate(sx + 1, sz + 1).biomes[ColumnClimate::index(0, 0)],
            Biome::Desert | Biome::DesertHills
        );
        if !dry && rng.below(WATER_ONE_IN) == 0 {
            let spot = (rng.below(16), rng.below(256), rng.below(16));
            lakes.extend(self.plan_lake((sx, sz), spot, FluidKind::Water, &mut rng));
        }
        if rng.below(LAVA_ONE_IN) == 0 {
            let rx = rng.below(16);
            let reach = rng.below(LAVA_HEIGHT_BASE) + LAVA_HEIGHT_MIN;
            let y = rng.below(reach);
            let rz = rng.below(16);
            if (y as i32) < SEA_LEVEL || rng.below(LAVA_SURFACE_ONE_IN) == 0 {
                let spot = (rx, y, rz);
                lakes.extend(self.plan_lake((sx, sz), spot, FluidKind::Lava, &mut rng));
            }
        }
        lakes
    }

    fn plan_lake(
        &self,
        (sx, sz): (i32, i32),
        (rx, ry, rz): (u32, u32, u32),
        kind: FluidKind,
        rng: &mut Stream,
    ) -> Option<Lake> {
        let (ox, oz) = (sx * SECTION_SIZE + rx as i32, sz * SECTION_SIZE + rz as i32);
        let mask = ellipsoids(rng);
        let ground = Ground::new(self, sx, sz);
        let mut y = ry as i32;
        while y > FLOOR_LIMIT && ground.block(ox, y, oz) == Block::Air {
            y -= 1;
        }
        if y < FLOOR_LIMIT || y - SINK + BOX_Y >= WORLD_HEIGHT {
            return None;
        }
        let y0 = y - SINK;
        if !walls_hold(&mask, kind, |i, j, k| ground.block(ox + i, y0 + k, oz + j)) {
            return None;
        }
        let mut footprint = [false; (BOX_XZ * BOX_XZ) as usize];
        for i in 0..BOX_XZ {
            for j in 0..BOX_XZ {
                footprint[(i * BOX_XZ + j) as usize] = (0..BOX_Y).any(|k| mask[slot(i, j, k)]);
            }
        }
        Some(Lake {
            kind,
            ox,
            oz,
            y0,
            mask,
            footprint,
        })
    }

    pub(in crate::terrain) fn grow_lakes(&self, key: ColumnPos, column: &mut Column) {
        if self.world_type != WorldType::Standard {
            return;
        }
        for sz in key.z() - 1..=key.z() {
            for sx in key.x() - 1..=key.x() {
                for lake in self.lakes_of(sx, sz).iter() {
                    self.fill_lake(key, column, lake);
                }
            }
        }
    }

    fn fill_lake(&self, key: ColumnPos, column: &mut Column, lake: &Lake) {
        let Lake {
            kind,
            ox,
            oz,
            y0,
            ref mask,
            ..
        } = *lake;
        let origin = key.origin();
        if ox + BOX_XZ - 2 < origin.x
            || ox + 1 >= origin.x + SECTION_SIZE
            || oz + BOX_XZ - 2 < origin.z
            || oz + 1 >= origin.z + SECTION_SIZE
        {
            return;
        }
        let climate = self.climate(key.x(), key.z());
        let local =
            |i: i32, j: i32, k: i32| IVec3::new(ox + i - origin.x, y0 + k, oz + j - origin.z);
        let inside =
            |at: IVec3| (0..SECTION_SIZE).contains(&at.x) && (0..SECTION_SIZE).contains(&at.z);
        for i in 0..BOX_XZ {
            for j in 0..BOX_XZ {
                for k in 0..BOX_Y {
                    let at = local(i, j, k);
                    if inside(at) && mask[slot(i, j, k)] {
                        column.set(
                            at,
                            if k >= WATER_LAYERS {
                                Block::Air
                            } else {
                                kind.block()
                            },
                        );
                    }
                }
            }
        }
        for i in 0..BOX_XZ {
            for j in 0..BOX_XZ {
                for k in WATER_LAYERS..BOX_Y {
                    let at = local(i, j, k);
                    if !inside(at) || !mask[slot(i, j, k)] {
                        continue;
                    }
                    let index = ColumnClimate::index(at.x, at.z);
                    let under = at - IVec3::Y;
                    if column.get(under) == Block::Dirt && at.y > climate.surface[index] {
                        let cover = if climate.biomes[index].is_mushroom_island() {
                            Block::Mycelium
                        } else {
                            Block::Grass
                        };
                        column.set(under, cover);
                    }
                }
            }
        }
        for i in 0..BOX_XZ {
            for j in 0..BOX_XZ {
                for k in 0..WATER_LAYERS {
                    let at = local(i, j, k);
                    if inside(at)
                        && !mask[slot(i, j, k)]
                        && touches_mask(mask, i, j, k)
                        && !column.get(at).is_solid()
                        && column.get(at) != kind.block()
                    {
                        column.set(at, Block::Stone);
                    }
                }
            }
        }
        match kind {
            FluidKind::Lava => {
                for i in 0..BOX_XZ {
                    for j in 0..BOX_XZ {
                        for k in 0..BOX_Y {
                            let at = local(i, j, k);
                            if !inside(at) || mask[slot(i, j, k)] || !touches_mask(mask, i, j, k) {
                                continue;
                            }
                            let roll =
                                hash(self.seed, ox + i, oz + j, SALT_LAKES + 2 + at.y as u64);
                            if (k < WATER_LAYERS || roll & 1 == 1) && column.get(at).is_solid() {
                                column.set(at, Block::Stone);
                            }
                        }
                    }
                }
            }
            FluidKind::Water => {
                for i in 0..BOX_XZ {
                    for j in 0..BOX_XZ {
                        let at = local(i, j, WATER_LAYERS - 1);
                        if inside(at)
                            && column.get(at) == Block::Water
                            && climate.biomes[ColumnClimate::index(at.x, at.z)].temperature()
                                < FREEZE_BELOW
                        {
                            column.set(at, Block::Ice);
                        }
                    }
                }
            }
        }
        for i in 0..BOX_XZ {
            for j in 0..BOX_XZ {
                let mut at = local(i, j, BOX_Y);
                if !inside(at) || !mask[slot(i, j, BOX_Y - 1)] {
                    continue;
                }
                while at.y < WORLD_HEIGHT && column.get(at).is_log() {
                    column.set(at, Block::Air);
                    at.y += 1;
                }
            }
        }
    }
}
