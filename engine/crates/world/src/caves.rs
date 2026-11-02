use fastnoise_lite::{FastNoiseLite, NoiseType};
use glam::IVec3;

use crate::block::Block;
use crate::column::{Column, WORLD_HEIGHT};
use crate::pos::ColumnPos;
use crate::section::SECTION_SIZE;
use crate::terrain::{WATER_LEVEL, smoothstep};

const CELL: i32 = 4;
const NODES_XZ: usize = (SECTION_SIZE / CELL + 1) as usize;
const NODES_Y: usize = (WORLD_HEIGHT / CELL + 1) as usize;
const FLOOR: i32 = 3;
pub const LAVA_LEVEL: i32 = 10;
const ROOF: i32 = 3;
const WET_ROOF: i32 = 4;

const CHEESE_FREQUENCY: f32 = 0.02;
const CHEESE_THRESHOLD: f32 = 0.6;
const CHEESE_FADE: f32 = 0.6;
const TUNNEL_FREQUENCY: f32 = 0.015;
const TUNNEL_Y_SCALE: f32 = 2.0;
const TUNNEL_A: f32 = 0.2;
const TUNNEL_B: f32 = 0.1;
const DEPTH_FADE: f32 = 24.0;
const BOTTOM_FADE: f32 = 12.0;
const ENTRANCE_FREQUENCY: f32 = 0.01;
const ENTRANCE_EDGES: (f32, f32) = (0.6, 0.8);
const ENTRANCE_BOOST: f32 = 0.4;

type Sample = [f32; 3];

pub struct Caves {
    cheese: FastNoiseLite,
    tunnel_a: FastNoiseLite,
    tunnel_b: FastNoiseLite,
    entrance: FastNoiseLite,
}

#[derive(Clone, Copy, Default)]
struct Limits {
    surface: i32,
    top: i32,
    entrance: f32,
}

impl Caves {
    pub fn new(seed: i32) -> Self {
        let noise = |offset, frequency| {
            let mut noise = FastNoiseLite::with_seed(seed.wrapping_add(offset));
            noise.set_noise_type(Some(NoiseType::OpenSimplex2));
            noise.set_frequency(Some(frequency));
            noise
        };
        Self {
            cheese: noise(10, CHEESE_FREQUENCY),
            tunnel_a: noise(11, TUNNEL_FREQUENCY),
            tunnel_b: noise(12, TUNNEL_FREQUENCY),
            entrance: noise(13, ENTRANCE_FREQUENCY),
        }
    }

    pub fn carve(
        &self,
        key: ColumnPos,
        column: &mut Column,
        surface: impl Fn(i32, i32) -> i32,
        has_tree: impl Fn(i32, i32) -> bool,
    ) {
        let origin = key.origin();
        const SIDE: i32 = SECTION_SIZE + 2;
        let mut heights = [0; (SIDE * SIDE) as usize];
        for z in 0..SIDE {
            for x in 0..SIDE {
                heights[(z * SIDE + x) as usize] = surface(origin.x + x - 1, origin.z + z - 1);
            }
        }
        let cached =
            |x: i32, z: i32| heights[((z - origin.z + 1) * SIDE + x - origin.x + 1) as usize];

        let mut limits = [[Limits::default(); SECTION_SIZE as usize]; SECTION_SIZE as usize];
        let mut top = FLOOR;
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                let l = self.limits(origin.x + x, origin.z + z, cached, &has_tree);
                top = top.max(l.top);
                limits[z as usize][x as usize] = l;
            }
        }
        if top <= FLOOR {
            return;
        }

        let (low, high) = ((FLOOR + 1) / CELL, top / CELL + 1);
        let mut nodes = [[[0.0; 3]; NODES_XZ * NODES_XZ]; NODES_Y];
        for j in low..=high {
            for k in 0..NODES_XZ as i32 {
                for i in 0..NODES_XZ as i32 {
                    let pos = origin + IVec3::new(i, j, k) * CELL;
                    nodes[j as usize][k as usize * NODES_XZ + i as usize] = self.node(pos);
                }
            }
        }

        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                let l = limits[z as usize][x as usize];
                let (i, k) = ((x / CELL) as usize, (z / CELL) as usize);
                let (tx, tz) = (fraction(x % CELL), fraction(z % CELL));
                let mut layers = [[0.0; 3]; NODES_Y];
                for j in low..=l.top / CELL + 1 {
                    let layer = &nodes[j as usize];
                    let at = |di: usize, dk: usize| layer[(k + dk) * NODES_XZ + i + di];
                    layers[j as usize] = bilinear([at(0, 0), at(1, 0), at(0, 1), at(1, 1)], tx, tz);
                }
                for y in FLOOR + 1..=l.top {
                    let local = IVec3::new(x, y, z);
                    if !carvable(column.get(local)) {
                        continue;
                    }
                    let j = (y / CELL) as usize;
                    let sample = lerp(layers[j], layers[j + 1], fraction(y % CELL));
                    if hollow(sample, l.density(y)) {
                        let filling = if y < LAVA_LEVEL {
                            Block::Lava
                        } else {
                            Block::Air
                        };
                        column.set(local, filling);
                        let above = local + IVec3::Y;
                        if Column::contains(above) && column.get(above) == Block::Snow {
                            column.set(above, Block::Air);
                        }
                    }
                }
            }
        }
    }

    fn limits(
        &self,
        x: i32,
        z: i32,
        surface: impl Fn(i32, i32) -> i32,
        has_tree: impl Fn(i32, i32) -> bool,
    ) -> Limits {
        let h = surface(x, z);
        let entrance = self.entrance(x, z);
        let mut top = if entrance > 0.0 && !has_tree(x, z) {
            h
        } else {
            h - ROOF
        };
        let wet_floor = (-1..=1)
            .flat_map(|dz| (-1..=1).map(move |dx| (x + dx, z + dz)))
            .map(|(x, z)| surface(x, z))
            .filter(|&s| s <= WATER_LEVEL + 1)
            .min();
        if let Some(floor) = wet_floor {
            top = top.min(floor - WET_ROOF - 1);
        }
        Limits {
            surface: h,
            top,
            entrance,
        }
    }

    fn entrance(&self, x: i32, z: i32) -> f32 {
        let n = self.entrance.get_noise_2d(x as f32, z as f32);
        smoothstep(ENTRANCE_EDGES.0, ENTRANCE_EDGES.1, n)
    }

    fn node(&self, pos: IVec3) -> Sample {
        let p = pos.as_vec3();
        [
            self.cheese.get_noise_3d(p.x, p.y, p.z),
            self.tunnel_a.get_noise_3d(p.x, p.y * TUNNEL_Y_SCALE, p.z),
            self.tunnel_b.get_noise_3d(p.x, p.y, p.z),
        ]
    }
}

impl Limits {
    fn density(&self, y: i32) -> f32 {
        let depth = smoothstep(0.0, DEPTH_FADE, (self.surface - y) as f32);
        let bottom = smoothstep(FLOOR as f32, FLOOR as f32 + BOTTOM_FADE, y as f32);
        (depth.max(self.entrance) + self.entrance * ENTRANCE_BOOST) * bottom
    }
}

fn hollow([cheese, a, b]: Sample, density: f32) -> bool {
    let hall = cheese > CHEESE_THRESHOLD + (1.0 - density) * CHEESE_FADE;
    hall || (a.abs() < TUNNEL_A * density && b.abs() < TUNNEL_B * density)
}

fn carvable(block: Block) -> bool {
    matches!(
        block,
        Block::Stone | Block::Dirt | Block::Sand | Block::Grass | Block::SnowyGrass
    )
}

fn fraction(offset: i32) -> f32 {
    offset as f32 / CELL as f32
}

fn lerp(a: Sample, b: Sample, t: f32) -> Sample {
    std::array::from_fn(|n| a[n] + (b[n] - a[n]) * t)
}

fn bilinear([c00, c10, c01, c11]: [Sample; 4], tx: f32, tz: f32) -> Sample {
    lerp(lerp(c00, c10, tx), lerp(c01, c11, tx), tz)
}

#[cfg(test)]
mod tests;
