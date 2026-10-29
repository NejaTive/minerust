use glam::IVec3;

use crate::block::Block;
use crate::column::Column;
use crate::pos::ColumnPos;
use crate::section::SECTION_SIZE;
use crate::terrain::{Biome, Terrain, WATER_LEVEL};

pub const CROWN_RADIUS: i32 = 2;
pub const MIN_SPACING: i32 = 3;
const FOREST_DENSITY: f32 = 0.04;
const PLAINS_DENSITY: f32 = 0.002;
const MIN_TRUNK: i32 = 4;
const MAX_TRUNK: i32 = 6;

const SALT_SPOT: u64 = 1;
const SALT_TRUNK: u64 = 2;
const SALT_CORNERS: u64 = 3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Tree {
    pub base: IVec3,
    pub height: i32,
    corners: u32,
}

impl Tree {
    pub fn logs(&self) -> impl Iterator<Item = IVec3> + '_ {
        (0..self.height).map(|dy| self.base + IVec3::Y * dy)
    }

    pub fn leaves(&self) -> impl Iterator<Item = IVec3> + '_ {
        let top = self.base.y + self.height - 1;
        let layers: [(i32, i32); 4] = [(top - 2, 2), (top - 1, 2), (top, 1), (top + 1, 1)];
        layers
            .into_iter()
            .enumerate()
            .flat_map(move |(layer, (y, r))| {
                (-r..=r).flat_map(move |dz| (-r..=r).map(move |dx| (layer, y, r, dx, dz)))
            })
            .filter(|&(layer, _, r, dx, dz)| {
                let corner = dx.abs() == r && dz.abs() == r;
                if !corner {
                    return true;
                }
                let bit = layer * 4 + usize::from(dx > 0) * 2 + usize::from(dz > 0);
                layer != 3 && self.corners & (1 << bit) != 0
            })
            .map(|(_, y, _, dx, dz)| IVec3::new(self.base.x + dx, y, self.base.z + dz))
    }
}

pub fn tree_at(terrain: &Terrain, x: i32, z: i32) -> Option<Tree> {
    let own = candidate(terrain, x, z)?;
    let reach = MIN_SPACING - 1;
    for dz in -reach..=reach {
        for dx in -reach..=reach {
            if (dx, dz) == (0, 0) {
                continue;
            }
            let (nx, nz) = (x + dx, z + dz);
            if candidate(terrain, nx, nz).is_some_and(|other| (other, nx, nz) < (own, x, z)) {
                return None;
            }
        }
    }
    let seed = terrain.seed();
    let trunk = hash(seed, x, z, SALT_TRUNK) % (MAX_TRUNK - MIN_TRUNK + 1) as u32;
    Some(Tree {
        base: IVec3::new(x, terrain.surface_height(x, z) + 1, z),
        height: MIN_TRUNK + trunk as i32,
        corners: hash(seed, x, z, SALT_CORNERS),
    })
}

fn candidate(terrain: &Terrain, x: i32, z: i32) -> Option<u32> {
    let h = hash(terrain.seed(), x, z, SALT_SPOT);
    let roll = (h >> 8) as f32 / (1 << 24) as f32;
    if roll >= FOREST_DENSITY.max(PLAINS_DENSITY) {
        return None;
    }
    let w = terrain.biome_weights(x, z);
    let density = w.forest * FOREST_DENSITY + w.plains * PLAINS_DENSITY;
    if w.dominant() == Biome::Desert || roll >= density {
        return None;
    }
    (terrain.surface(x, z, w) > WATER_LEVEL + 1).then_some(h)
}

pub fn plant(terrain: &Terrain, key: ColumnPos, column: &mut Column) {
    let origin = key.origin();
    let range = |start: i32| start - CROWN_RADIUS..start + SECTION_SIZE + CROWN_RADIUS;
    let trees: Vec<Tree> = range(origin.z)
        .flat_map(|z| range(origin.x).map(move |x| (x, z)))
        .filter_map(|(x, z)| tree_at(terrain, x, z))
        .collect();
    for log in trees.iter().flat_map(Tree::logs) {
        let local = log - origin;
        if Column::contains(local) {
            column.set(local, Block::Log);
        }
    }
    for leaf in trees.iter().flat_map(Tree::leaves) {
        let local = leaf - origin;
        if Column::contains(local) && column.get(local) == Block::Air {
            column.set(local, Block::Leaves);
        }
    }
}

pub(crate) fn hash(seed: i32, x: i32, z: i32, salt: u64) -> u32 {
    let point = (u64::from(x as u32) << 32) | u64::from(z as u32);
    let stream = splitmix64((u64::from(seed as u32) << 8) | salt);
    (splitmix64(point ^ stream) >> 32) as u32
}

fn splitmix64(mut v: u64) -> u64 {
    v = v.wrapping_add(0x9e37_79b9_7f4a_7c15);
    v = (v ^ (v >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    v = (v ^ (v >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    v ^ (v >> 31)
}

#[doc(hidden)]
pub fn border_tree(terrain: &Terrain) -> Tree {
    (0..4096)
        .flat_map(|z| (0..64).map(move |i| (i * SECTION_SIZE + i % 2 * 15, z)))
        .find_map(|(x, z)| tree_at(terrain, x, z))
        .expect("no tree on a column border")
}

#[cfg(test)]
mod tests;
