use glam::IVec3;

use crate::block::{AXIS_X, AXIS_Y, AXIS_Z, Block};
use crate::column::Column;
use crate::pos::ColumnPos;
use crate::section::SECTION_SIZE;
use crate::terrain::{Biome, Terrain, WATER_LEVEL};

pub const MAX_REACH: i32 = 6;
const FOREST_DENSITY: f32 = 0.04;
const PLAINS_DENSITY: f32 = 0.002;
const HIGHLAND_HEIGHT: i32 = 78;
const HIGHLAND_SPRUCE_PERCENT: u32 = 85;

const SALT_SPOT: u64 = 1;
const SALT_TRUNK: u64 = 2;
const SALT_CORNERS: u64 = 3;
const SALT_KIND: u64 = 4;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TreeKind {
    Oak,
    Birch,
    Spruce,
    BigOak,
}

impl TreeKind {
    pub fn log(self) -> Block {
        match self {
            TreeKind::Oak | TreeKind::BigOak => Block::Log,
            TreeKind::Birch => Block::BirchLog,
            TreeKind::Spruce => Block::SpruceLog,
        }
    }

    pub fn leaves(self) -> Block {
        match self {
            TreeKind::Oak | TreeKind::BigOak => Block::Leaves,
            TreeKind::Birch => Block::BirchLeaves,
            TreeKind::Spruce => Block::SpruceLeaves,
        }
    }

    fn trunk_range(self) -> (i32, i32) {
        match self {
            TreeKind::Oak => (4, 6),
            TreeKind::Birch => (5, 7),
            TreeKind::Spruce => (7, 10),
            TreeKind::BigOak => (6, 9),
        }
    }

    fn footprint(self) -> i32 {
        match self {
            TreeKind::BigOak => 5,
            _ => 2,
        }
    }
}

fn spacing(a: TreeKind, b: TreeKind) -> i32 {
    a.footprint() + b.footprint() - 1
}

const MAX_SPACING: i32 = 9;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Tree {
    pub base: IVec3,
    pub height: i32,
    pub kind: TreeKind,
    shape: u32,
}

struct Rng(u64);

impl Rng {
    fn below(&mut self, n: u32) -> i32 {
        self.0 = splitmix64(self.0);
        ((self.0 >> 32) as u32 % n) as i32
    }
}

const SIDES: [(i32, i32, u8); 4] = [
    (1, 0, AXIS_X),
    (-1, 0, AXIS_X),
    (0, 1, AXIS_Z),
    (0, -1, AXIS_Z),
];

impl Tree {
    pub fn top(&self) -> i32 {
        self.base.y + self.height - 1
    }

    pub fn round_radius(&self) -> i32 {
        match self.kind {
            TreeKind::Birch => 1 + (self.shape >> 16) as i32 % 2,
            _ => 2,
        }
    }

    pub fn logs(&self) -> Vec<(IVec3, u8)> {
        let mut logs: Vec<(IVec3, u8)> = (0..self.height)
            .map(|dy| (self.base + IVec3::Y * dy, AXIS_Y))
            .collect();
        for branch in self.branches() {
            logs.extend(branch.logs());
        }
        logs
    }

    pub fn leaves(&self) -> Vec<IVec3> {
        match self.kind {
            TreeKind::Oak | TreeKind::Birch => self.round_crown(),
            TreeKind::Spruce => self.cone_crown(),
            TreeKind::BigOak => self.big_crown(),
        }
    }

    pub fn branches(&self) -> Vec<Branch> {
        if self.kind != TreeKind::BigOak {
            return Vec::new();
        }
        let mut rng = Rng(u64::from(self.shape));
        let count = 2 + rng.below(3) as usize;
        let mut sides = SIDES;
        for i in 0..sides.len() - 1 {
            let other = i + rng.below((sides.len() - i) as u32) as usize;
            sides.swap(i, other);
        }
        sides[..count]
            .iter()
            .map(|&(dx, dz, axis)| {
                let low = self.height / 2;
                let rise = low + rng.below((self.height - 1 - low).max(1) as u32);
                Branch {
                    start: self.base + IVec3::Y * rise,
                    dir: IVec3::new(dx, 0, dz),
                    axis,
                    length: 2 + rng.below(2),
                    radius: 2 + rng.below(2),
                }
            })
            .collect()
    }

    fn round_crown(&self) -> Vec<IVec3> {
        let r = self.round_radius();
        let top = self.top();
        let layers = [
            (top - 2, r),
            (top - 1, r),
            (top, (r - 1).max(1)),
            (top + 1, r - 1),
        ];
        let mut leaves = Vec::new();
        for (layer, (y, r)) in layers.into_iter().enumerate() {
            for dz in -r..=r {
                for dx in -r..=r {
                    let corner = r > 0 && dx.abs() == r && dz.abs() == r;
                    let bit = layer * 4 + usize::from(dx > 0) * 2 + usize::from(dz > 0);
                    if corner && (layer == 3 || self.shape & (1 << bit) == 0) {
                        continue;
                    }
                    leaves.push(IVec3::new(self.base.x + dx, y, self.base.z + dz));
                }
            }
        }
        leaves
    }

    fn spruce_layers(&self) -> impl Iterator<Item = (i32, i32)> {
        let (top, floor) = (self.top(), self.base.y + 2);
        (0..)
            .map(move |i| (top - 2 * i, (1 + (i + 1) / 2).min(3)))
            .take_while(move |&(y, _)| y >= floor)
    }

    fn cone_crown(&self) -> Vec<IVec3> {
        let mut leaves = Vec::new();
        for (y, r) in self.spruce_layers() {
            for dz in -r..=r {
                for dx in -r..=r {
                    if dx.abs() + dz.abs() <= r + r / 2 {
                        leaves.push(IVec3::new(self.base.x + dx, y, self.base.z + dz));
                    }
                }
            }
        }
        let caps = 1 + (self.shape & 1) as i32;
        leaves.extend((1..=caps).map(|dy| IVec3::new(self.base.x, self.top() + dy, self.base.z)));
        leaves
    }

    fn big_crown(&self) -> Vec<IVec3> {
        let mut rng = Rng(u64::from(self.shape) ^ 0xb16);
        let top_ball = (
            IVec3::new(self.base.x, self.base.y + self.height, self.base.z),
            2 + rng.below(2),
        );
        let balls = self.branches().into_iter().map(|b| (b.tip(), b.radius));
        let mut leaves = Vec::new();
        for (center, r) in std::iter::once(top_ball).chain(balls) {
            for dy in -r..=r {
                for dz in -r..=r {
                    for dx in -r..=r {
                        if dx * dx + dy * dy + dz * dz <= r * r + 1 {
                            leaves.push(center + IVec3::new(dx, dy, dz));
                        }
                    }
                }
            }
        }
        leaves
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Branch {
    pub start: IVec3,
    pub dir: IVec3,
    pub axis: u8,
    pub length: i32,
    pub radius: i32,
}

impl Branch {
    pub fn logs(&self) -> impl Iterator<Item = (IVec3, u8)> + '_ {
        (1..=self.length).map(|s| (self.start + self.dir * s + IVec3::Y * s, self.axis))
    }

    pub fn tip(&self) -> IVec3 {
        self.start + self.dir * self.length + IVec3::Y * self.length
    }
}

struct Candidate {
    roll: u32,
    kind: TreeKind,
}

impl Candidate {
    fn priority(&self) -> (bool, u32) {
        (self.kind != TreeKind::BigOak, self.roll)
    }
}

pub fn tree_at(terrain: &Terrain, x: i32, z: i32) -> Option<Tree> {
    let own = candidate(terrain, x, z)?;
    let reach = MAX_SPACING - 1;
    for dz in -reach..=reach {
        for dx in -reach..=reach {
            if (dx, dz) == (0, 0) {
                continue;
            }
            let (nx, nz) = (x + dx, z + dz);
            let Some(other) = candidate(terrain, nx, nz) else {
                continue;
            };
            let near = dx.abs().max(dz.abs()) < spacing(own.kind, other.kind);
            if near && (other.priority(), nx, nz) < (own.priority(), x, z) {
                return None;
            }
        }
    }
    let seed = terrain.seed();
    let (min, max) = own.kind.trunk_range();
    let trunk = hash(seed, x, z, SALT_TRUNK) % (max - min + 1) as u32;
    Some(Tree {
        base: IVec3::new(x, terrain.surface_height(x, z) + 1, z),
        height: min + trunk as i32,
        kind: own.kind,
        shape: hash(seed, x, z, SALT_CORNERS),
    })
}

fn candidate(terrain: &Terrain, x: i32, z: i32) -> Option<Candidate> {
    let h = hash(terrain.seed(), x, z, SALT_SPOT);
    let roll = (h >> 8) as f32 / (1 << 24) as f32;
    if roll >= FOREST_DENSITY.max(PLAINS_DENSITY) {
        return None;
    }
    let w = terrain.biome_weights(x, z);
    let density = w.forest * FOREST_DENSITY + w.plains * PLAINS_DENSITY;
    let biome = w.dominant();
    if biome == Biome::Desert || roll >= density {
        return None;
    }
    let surface = terrain.surface(x, z, w);
    if surface <= WATER_LEVEL + 1 {
        return None;
    }
    let kind = pick_kind(biome, surface, hash(terrain.seed(), x, z, SALT_KIND));
    Some(Candidate { roll: h, kind })
}

fn pick_kind(biome: Biome, surface: i32, bits: u32) -> TreeKind {
    let (first, second) = (bits % 100, (bits >> 16) % 100);
    if surface >= HIGHLAND_HEIGHT && first < HIGHLAND_SPRUCE_PERCENT {
        return TreeKind::Spruce;
    }
    match biome {
        Biome::Forest => match second {
            0..51 => TreeKind::Oak,
            51..82 => TreeKind::Birch,
            82..89 => TreeKind::BigOak,
            _ => TreeKind::Spruce,
        },
        _ => {
            if second < 50 {
                TreeKind::Oak
            } else {
                TreeKind::Birch
            }
        }
    }
}

pub fn plant(terrain: &Terrain, key: ColumnPos, column: &mut Column) {
    let origin = key.origin();
    let range = |start: i32| start - MAX_REACH..start + SECTION_SIZE + MAX_REACH;
    let trees: Vec<Tree> = range(origin.z)
        .flat_map(|z| range(origin.x).map(move |x| (x, z)))
        .filter_map(|(x, z)| tree_at(terrain, x, z))
        .collect();
    for tree in &trees {
        for (log, axis) in tree.logs() {
            let local = log - origin;
            if Column::contains(local) {
                column.set_with_data(local, tree.kind.log(), axis);
            }
        }
    }
    for tree in &trees {
        for leaf in tree.leaves() {
            let local = leaf - origin;
            if Column::contains(local) && column.get(local) == Block::Air {
                column.set(local, tree.kind.leaves());
            }
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
pub fn border_tree(terrain: &Terrain, kind: TreeKind) -> Tree {
    (0..4096)
        .flat_map(|z| (0..64).map(move |i| (i * SECTION_SIZE + i % 2 * 15, z)))
        .filter_map(|(x, z)| tree_at(terrain, x, z))
        .find(|tree| tree.kind == kind)
        .expect("no tree on a column border")
}

#[cfg(test)]
mod tests;
