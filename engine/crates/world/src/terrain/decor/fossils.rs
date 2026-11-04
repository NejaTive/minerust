use super::*;
use crate::block::{AXIS_X, AXIS_Y, AXIS_Z};

const SALT_FOSSILS: u64 = 130;
const FOSSIL_ONE_IN: u32 = 64;
const DEPTH_BELOW_SURFACE: i32 = 15;
const DEPTH_SPAN: u32 = 10;
const MIN_FLOOR: i32 = 10;
const BONE_LOST_ONE_IN: u32 = 10;
const COAL_ONE_IN: u32 = 10;

type Cell = (IVec3, u8);
pub(super) type FossilBlocks = Vec<(IVec3, Block, u8)>;

fn spine(variant: u32) -> Vec<Cell> {
    let half: i32 = [6, 6, 5, 7][variant as usize];
    let step: usize = [2, 2, 3, 2][variant as usize];
    let radius: i32 = [2, 3, 3, 2][variant as usize];
    let tail_lift = variant == 3;
    let mut cells = Vec::new();
    for x in -half..=half {
        let lift = i32::from(tail_lift && x.abs() > half - 2);
        cells.push((IVec3::new(x, lift, 0), AXIS_X));
    }
    for x in (-half + 2..=half - 2).step_by(step) {
        for z in -radius..=radius {
            let y = ((radius * radius - z * z) as f32).sqrt().round() as i32;
            if y == 0 {
                continue;
            }
            let axis = if y >= radius - 1 { AXIS_Z } else { AXIS_Y };
            cells.push((IVec3::new(x, y, z), axis));
        }
    }
    if variant == 1 {
        for (dx, dz) in [(1, 0), (1, 1), (1, -1)] {
            cells.push((IVec3::new(half + dx, 0, dz), AXIS_Y));
        }
    }
    cells
}

fn skull(variant: u32) -> Vec<Cell> {
    let (w, h, d) = [(5, 5, 5), (5, 4, 6), (6, 5, 5), (4, 5, 5)][variant as usize];
    let mut cells = Vec::new();
    for x in 0..w {
        for y in 0..h {
            for z in 0..d {
                let edges = [
                    x == 0 || x == w - 1,
                    y == 0 || y == h - 1,
                    z == 0 || z == d - 1,
                ];
                let shell = edges.iter().any(|&e| e);
                let corner = edges.iter().all(|&e| e);
                let front = z == d - 1;
                let eye = front && y == h - 2 && (x == 1 || x == w - 2);
                let teeth = front && y == 0 && x % 2 == 1;
                let open_jaw = y == 0 && !front && x != 0 && x != w - 1 && z != 0;
                if shell && !corner && !eye && !teeth && !open_jaw {
                    cells.push((IVec3::new(x - w / 2, y, z - d / 2), AXIS_Y));
                }
            }
        }
    }
    cells
}

fn rotate(offset: IVec3, turns: u32, mirror: bool) -> IVec3 {
    let mut p = if mirror {
        IVec3::new(-offset.x, offset.y, offset.z)
    } else {
        offset
    };
    for _ in 0..turns {
        p = IVec3::new(-p.z, p.y, p.x);
    }
    p
}

fn turn_axis(axis: u8, turns: u32) -> u8 {
    match (axis, turns % 2) {
        (AXIS_X, 1) => AXIS_Z,
        (AXIS_Z, 1) => AXIS_X,
        _ => axis,
    }
}

impl Terrain {
    /// `WorldGenFossils`: окаменелость колонки `(sx, sz)` как набор блоков в мировых координатах.
    pub(super) fn fossil(&self, sx: i32, sz: i32) -> FossilBlocks {
        let biome = self.climate(sx, sz).biomes[ColumnClimate::index(CENTER, CENTER)];
        if !(biome.is_desert() || biome.is_swamp()) {
            return Vec::new();
        }
        let state = u64::from(hash(self.seed, sx, sz, SALT_FOSSILS))
            | u64::from(hash(self.seed, sx, sz, SALT_FOSSILS + 1)) << 32;
        let mut rng = Stream::new(state);
        if rng.below(FOSSIL_ONE_IN) != 0 {
            return Vec::new();
        }
        let x = sx * SECTION_SIZE + rng.below(SECTION_SIZE as u32) as i32;
        let z = sz * SECTION_SIZE + rng.below(SECTION_SIZE as u32) as i32;
        let turns = rng.below(4);
        let mirror = rng.flip();
        let variant = rng.below(4);
        let shape = if rng.flip() {
            spine(variant)
        } else {
            skull(variant)
        };
        let lower = rng.below(DEPTH_SPAN) as i32;
        let cells: Vec<Cell> = shape
            .into_iter()
            .map(|(offset, axis)| (rotate(offset, turns, mirror), turn_axis(axis, turns)))
            .collect();
        let (mut low, mut high) = (IVec3::MAX, IVec3::MIN);
        for (offset, _) in &cells {
            low = low.min(*offset);
            high = high.max(*offset);
        }
        let floor = [
            (low.x, low.z),
            (low.x, high.z),
            (high.x, low.z),
            (high.x, high.z),
        ]
        .into_iter()
        .map(|(dx, dz)| self.surface_height(x + dx, z + dz))
        .min()
        .unwrap_or(MIN_FLOOR);
        let base_y = (floor - DEPTH_BELOW_SURFACE - lower).max(MIN_FLOOR);
        cells
            .into_iter()
            .filter_map(|(offset, axis)| {
                let bone_lost = rng.below(BONE_LOST_ONE_IN) == 0;
                let coal = rng.below(COAL_ONE_IN) == 0;
                let at = IVec3::new(x + offset.x, base_y + offset.y, z + offset.z);
                match (bone_lost, coal) {
                    (_, true) => Some((at, Block::CoalOre, 0)),
                    (false, false) => Some((at, Block::BoneBlock, axis)),
                    (true, false) => None,
                }
            })
            .collect()
    }

    pub(crate) fn grow_fossils(&self, key: ColumnPos, column: &mut Column) {
        if self.world_type != WorldType::Standard {
            return;
        }
        let origin = key.origin();
        for sz in key.z() - 1..=key.z() + 1 {
            for sx in key.x() - 1..=key.x() + 1 {
                for (at, block, data) in self.fossil(sx, sz) {
                    let local = at - IVec3::new(origin.x, 0, origin.z);
                    if Column::contains(local) && column.get(local) != Block::Bedrock {
                        column.set_with_data(local, block, data);
                    }
                }
            }
        }
    }
}
