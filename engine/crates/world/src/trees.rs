use glam::IVec3;

use crate::block::{AXIS_X, AXIS_Y, AXIS_Z, Block, COCOA_STAGE_SHIFT, COCOA_STAGES};
use crate::column::Column;
use crate::pos::ColumnPos;
use crate::section::SECTION_SIZE;
use crate::terrain::{Biome, Terrain, WATER_LEVEL, WorldType};

pub const MAX_REACH: i32 = 9;
const MAX_TREES_PER_CHUNK: f32 = 50.0;
const BLOCKS_PER_CHUNK: f32 = 256.0;

const SALT_SPOT: u64 = 1;
const SALT_TRUNK: u64 = 2;
const SALT_CORNERS: u64 = 3;
const SALT_KIND: u64 = 4;
const SALT_GRID: u64 = 5;
const SALT_GRID_KIND: u64 = 6;
const GRID_CELL: i32 = 4;
const GRID_OFFSET: i32 = 1;
const GRID_MUSHROOM_ONE_IN: u32 = 20;
const MUSHROOM_TALL_ONE_IN: u32 = 12;
const JUNGLE_BIG_OAK_FOOTPRINT: i32 = 3;
const MEGA_TAIGA_PERCENT: u32 = 33;
const MEGA_SPRUCE_ONE_IN: u32 = 13;
const PINE_ONE_IN: u32 = 3;
const HILLS_SPRUCE_OF: u32 = 3;
const PODZOL_SPOTS: [(i32, i32); 4] = [(-1, -1), (2, -1), (-1, 2), (2, 2)];
const PODZOL_RING: i32 = 8;
const PODZOL_EXTRA_SPOTS: u32 = 5;
const PODZOL_RADIUS: i32 = 2;
const PODZOL_HEIGHT: i32 = 2;
const PODZOL_DEPTH: i32 = 3;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum TreeKind {
    Oak,
    Birch,
    Spruce,
    Pine,
    BigOak,
    JungleTree,
    MegaJungle,
    Shrub,
    Acacia,
    DarkOak,
    BrownMushroom,
    RedMushroom,
    SwampOak,
    TallBirch,
    MegaPine,
    MegaSpruce,
}

impl TreeKind {
    pub fn log(self) -> Block {
        match self {
            TreeKind::Oak | TreeKind::BigOak | TreeKind::SwampOak => Block::Log,
            TreeKind::Birch | TreeKind::TallBirch => Block::BirchLog,
            TreeKind::Spruce | TreeKind::Pine | TreeKind::MegaPine | TreeKind::MegaSpruce => {
                Block::SpruceLog
            }
            TreeKind::JungleTree | TreeKind::MegaJungle | TreeKind::Shrub => Block::JungleLog,
            TreeKind::Acacia => Block::AcaciaLog,
            TreeKind::DarkOak => Block::DarkOakLog,
            TreeKind::BrownMushroom | TreeKind::RedMushroom => Block::MushroomStem,
        }
    }

    pub fn leaves(self) -> Block {
        match self {
            TreeKind::Oak | TreeKind::BigOak | TreeKind::Shrub | TreeKind::SwampOak => {
                Block::Leaves
            }
            TreeKind::Birch | TreeKind::TallBirch => Block::BirchLeaves,
            TreeKind::Spruce | TreeKind::Pine | TreeKind::MegaPine | TreeKind::MegaSpruce => {
                Block::SpruceLeaves
            }
            TreeKind::JungleTree | TreeKind::MegaJungle => Block::JungleLeaves,
            TreeKind::Acacia => Block::AcaciaLeaves,
            TreeKind::DarkOak => Block::DarkOakLeaves,
            TreeKind::BrownMushroom => Block::BrownMushroomBlock,
            TreeKind::RedMushroom => Block::RedMushroomBlock,
        }
    }

    fn wide_trunk(self) -> bool {
        matches!(
            self,
            TreeKind::MegaJungle | TreeKind::DarkOak | TreeKind::MegaPine | TreeKind::MegaSpruce
        )
    }

    fn trunk_range(self) -> (i32, i32) {
        match self {
            TreeKind::Oak => (4, 6),
            TreeKind::Birch => (5, 7),
            TreeKind::TallBirch => (5, 13),
            TreeKind::MegaPine | TreeKind::MegaSpruce => (13, 29),
            TreeKind::Spruce => (6, 9),
            TreeKind::Pine => (7, 11),
            TreeKind::BigOak => (6, 9),
            TreeKind::JungleTree => (4, 10),
            TreeKind::MegaJungle => (10, 31),
            TreeKind::Shrub => (1, 1),
            TreeKind::Acacia => (5, 9),
            TreeKind::DarkOak => (6, 9),
            TreeKind::BrownMushroom | TreeKind::RedMushroom => (4, 6),
            TreeKind::SwampOak => (5, 8),
        }
    }

    fn footprint(self) -> i32 {
        match self {
            TreeKind::BigOak => 5,
            TreeKind::Acacia | TreeKind::MegaJungle | TreeKind::MegaPine | TreeKind::MegaSpruce => {
                3
            }
            TreeKind::DarkOak
            | TreeKind::BrownMushroom
            | TreeKind::RedMushroom
            | TreeKind::Shrub => 1,
            _ => 2,
        }
    }
}

fn spacing(a: &Candidate, b: &Candidate) -> i32 {
    a.footprint + b.footprint - 1
}

const MAX_SPACING: i32 = 9;
const COCOA_SALT: u64 = 0xc0c0a;
const COCOA_MIN_HEIGHT: i32 = 5;
const COCOA_TREE_ONE_IN: u32 = 5;
const COCOA_ROWS: i32 = 2;
const COCOA_SIDES: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

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
            TreeKind::Birch | TreeKind::TallBirch => 1 + (self.shape >> 16) as i32 % 2,
            _ => 2,
        }
    }

    fn parts(&self) -> Option<shapes::Parts> {
        Some(match self.kind {
            TreeKind::JungleTree => {
                shapes::jungle_tree(self.base, self.height, self.shape, self.round_crown())
            }
            TreeKind::MegaJungle => shapes::mega_jungle(self.base, self.height, self.shape),
            TreeKind::Shrub => shapes::shrub(self.base, self.shape),
            TreeKind::Acacia => shapes::acacia(self.base, self.height, self.shape),
            TreeKind::DarkOak => shapes::dark_oak(self.base, self.height, self.shape),
            TreeKind::BrownMushroom => shapes::mushroom(self.base, self.height, false),
            TreeKind::RedMushroom => shapes::mushroom(self.base, self.height, true),
            TreeKind::SwampOak => shapes::swamp_oak(self.base, self.height, self.shape),
            TreeKind::Spruce => shapes::taiga_spruce(self.base, self.height, self.shape),
            TreeKind::Pine => shapes::taiga_pine(self.base, self.height, self.shape),
            TreeKind::MegaPine => shapes::mega_pine(self.base, self.height, self.shape, false),
            TreeKind::MegaSpruce => shapes::mega_pine(self.base, self.height, self.shape, true),
            _ => return None,
        })
    }

    pub fn podzol(&self) -> Vec<IVec3> {
        if !matches!(self.kind, TreeKind::MegaPine | TreeKind::MegaSpruce) {
            return Vec::new();
        }
        let mut rng = Rng(u64::from(self.shape) ^ 0x70d);
        let mut centers: Vec<(i32, i32)> = PODZOL_SPOTS.to_vec();
        for _ in 0..PODZOL_EXTRA_SPOTS {
            let roll = rng.below(PODZOL_RING as u32 * PODZOL_RING as u32);
            let (k, l) = (roll % PODZOL_RING, roll / PODZOL_RING);
            if [k, l].iter().any(|&v| v == 0 || v == PODZOL_RING - 1) {
                centers.push((k - 3, l - 3));
            }
        }
        let mut spots = Vec::new();
        for (cx, cz) in centers {
            for dz in -PODZOL_RADIUS..=PODZOL_RADIUS {
                for dx in -PODZOL_RADIUS..=PODZOL_RADIUS {
                    if dx.abs() != PODZOL_RADIUS || dz.abs() != PODZOL_RADIUS {
                        spots.push(self.base + IVec3::new(cx + dx, 0, cz + dz));
                    }
                }
            }
        }
        spots
    }

    pub fn logs(&self) -> Vec<(IVec3, u8)> {
        if let Some(parts) = self.parts() {
            return parts.logs;
        }
        let mut logs: Vec<(IVec3, u8)> = (0..self.height)
            .map(|dy| (self.base + IVec3::Y * dy, AXIS_Y))
            .collect();
        for branch in self.branches() {
            logs.extend(branch.logs());
        }
        logs
    }

    pub fn leaves(&self) -> Vec<IVec3> {
        if let Some(parts) = self.parts() {
            return parts.leaves;
        }
        match self.kind {
            TreeKind::Oak | TreeKind::Birch | TreeKind::TallBirch => self.round_crown(),
            _ => self.big_crown(),
        }
    }

    pub fn vines(&self) -> Vec<(IVec3, u8, i32)> {
        let Some(parts) = self.parts() else {
            return Vec::new();
        };
        parts
            .vines
            .into_iter()
            .map(|v| (v.cell, v.side, v.drop))
            .collect()
    }

    /// `WorldGenTrees.generate` для джунглей: плоды у ствола на высоте `height - 5` и `height - 4`.
    pub fn cocoas(&self) -> Vec<(IVec3, u8)> {
        let mut rng = Rng(u64::from(self.shape) ^ COCOA_SALT);
        if self.kind != TreeKind::JungleTree
            || self.height <= COCOA_MIN_HEIGHT
            || rng.below(COCOA_TREE_ONE_IN) != 0
        {
            return Vec::new();
        }
        let mut pods = Vec::new();
        for row in 0..COCOA_ROWS {
            for (dx, dz) in COCOA_SIDES {
                if rng.below((4 - row) as u32) != 0 {
                    continue;
                }
                let stage = rng.below(u32::from(COCOA_STAGES)) as u8;
                let outward = IVec3::new(-dx, 0, -dz);
                let at = self.base + outward + IVec3::Y * (self.height - COCOA_MIN_HEIGHT + row);
                let data = Block::Cocoa.placement_data(outward).expect("cocoa side");
                pods.push((at, stage << COCOA_STAGE_SHIFT | data));
            }
        }
        pods
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
    footprint: i32,
}

impl Candidate {
    fn new(roll: u32, kind: TreeKind, biome: Biome) -> Self {
        let footprint = match kind {
            TreeKind::BigOak if biome.is_jungle() => JUNGLE_BIG_OAK_FOOTPRINT,
            _ => kind.footprint(),
        };
        Self {
            roll,
            kind,
            footprint,
        }
    }

    fn priority(&self) -> (i32, u32) {
        (-self.footprint, self.roll)
    }
}

pub fn tree_at(terrain: &Terrain, x: i32, z: i32) -> Option<Tree> {
    if terrain.world_type() != WorldType::Standard {
        return None;
    }
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
            let near = dx.abs().max(dz.abs()) < spacing(&own, &other);
            if near && (other.priority(), nx, nz) < (own.priority(), x, z) {
                return None;
            }
        }
    }
    let seed = terrain.seed();
    let (min, max) = own.kind.trunk_range();
    let trunk = hash(seed, x, z, SALT_TRUNK) % (max - min + 1) as u32;
    let shape = hash(seed, x, z, SALT_CORNERS);
    let mut height = min + trunk as i32;
    if matches!(own.kind, TreeKind::BrownMushroom | TreeKind::RedMushroom)
        && shape.is_multiple_of(MUSHROOM_TALL_ONE_IN)
    {
        height *= 2;
    }
    Some(Tree {
        base: IVec3::new(x, ground(terrain, x, z) + 1, z),
        height,
        kind: own.kind,
        shape,
    })
}

fn ground(terrain: &Terrain, x: i32, z: i32) -> i32 {
    terrain.surface_height(x, z) - i32::from(terrain.is_puddle(x, z))
}

fn grid_spot(seed: i32, cell_x: i32, cell_z: i32) -> (i32, i32, u32) {
    let h = hash(seed, cell_x, cell_z, SALT_GRID);
    let x = cell_x * GRID_CELL + GRID_OFFSET + (h % 3) as i32;
    let z = cell_z * GRID_CELL + GRID_OFFSET + (h / 3 % 3) as i32;
    (x, z, h)
}

fn roofed_candidate(terrain: &Terrain, x: i32, z: i32) -> Option<Candidate> {
    let (cell_x, cell_z) = (
        (x - GRID_OFFSET).div_euclid(GRID_CELL),
        (z - GRID_OFFSET).div_euclid(GRID_CELL),
    );
    let (spot_x, spot_z, h) = grid_spot(terrain.seed(), cell_x, cell_z);
    if (spot_x, spot_z) != (x, z) || !terrain.biome_at(x, z).is_roofed() {
        return None;
    }
    if terrain.surface_height(x, z) <= WATER_LEVEL + 1 {
        return None;
    }
    let bits = hash(terrain.seed(), cell_x, cell_z, SALT_GRID_KIND);
    let kind = if (h >> 8) % GRID_MUSHROOM_ONE_IN == 0 {
        if bits & 1 == 0 {
            TreeKind::BrownMushroom
        } else {
            TreeKind::RedMushroom
        }
    } else {
        pick_kind(terrain.biome_at(x, z), bits)
    };
    flat_footprint(terrain, x, z, kind).then(|| Candidate::new(h, kind, terrain.biome_at(x, z)))
}

fn flat_footprint(terrain: &Terrain, x: i32, z: i32, kind: TreeKind) -> bool {
    if !kind.wide_trunk() {
        return true;
    }
    let ground = terrain.surface_height(x, z);
    [(1, 0), (0, 1), (1, 1)]
        .into_iter()
        .all(|(dx, dz)| terrain.surface_height(x + dx, z + dz) == ground)
}

fn candidate(terrain: &Terrain, x: i32, z: i32) -> Option<Candidate> {
    if let Some(grid) = roofed_candidate(terrain, x, z) {
        return Some(grid);
    }
    let h = hash(terrain.seed(), x, z, SALT_SPOT);
    let roll = (h >> 8) as f32 / (1 << 24) as f32;
    if roll >= MAX_TREES_PER_CHUNK / BLOCKS_PER_CHUNK {
        return None;
    }
    let biome = terrain.biome_at(x, z);
    if roll >= trees_per_chunk(biome) / BLOCKS_PER_CHUNK || biome.is_roofed() {
        return None;
    }
    let lowest = if biome.is_swamp() {
        WATER_LEVEL - 1
    } else {
        WATER_LEVEL + 2
    };
    if ground(terrain, x, z) < lowest {
        return None;
    }
    let kind = pick_kind(biome, hash(terrain.seed(), x, z, SALT_KIND));
    flat_footprint(terrain, x, z, kind).then(|| Candidate::new(h, kind, biome))
}

// `treesPerChunk` из `BiomeDecorator` 1.12; у тёмного леса деревья ставит сетка 4×4 (`roofed_candidate`).
fn trees_per_chunk(biome: Biome) -> f32 {
    use Biome::*;
    match biome {
        Forest | ForestHills | BirchForest | BirchForestHills | BirchForestM
        | BirchForestHillsM | Taiga | TaigaHills | TaigaM | ColdTaiga | ColdTaigaHills
        | ColdTaigaM | MegaTaiga | MegaTaigaHills | MegaSpruceTaiga | MegaSpruceTaigaHills => 10.0,
        FlowerForest => 6.0,
        RoofedForest | RoofedForestM => 16.0,
        Jungle | JungleHills | JungleM => MAX_TREES_PER_CHUNK,
        JungleEdge | JungleEdgeM | Swampland | SwamplandM => 2.0,
        Savanna | SavannaPlateau => 1.0,
        SavannaM | SavannaPlateauM => 2.0,
        ExtremeHillsPlus | ExtremeHillsPlusM => 3.0,
        MesaPlateauF | MesaPlateauFM => 5.0,
        Plains | SunflowerPlains => 0.5,
        MushroomIsland => 1.0,
        IcePlains | IceMountains | IceSpikes => 0.1,
        _ => 0.0,
    }
}

fn pick_kind(biome: Biome, bits: u32) -> TreeKind {
    use Biome::*;
    let second = (bits >> 16) % 100;
    match biome {
        Forest | ForestHills | FlowerForest => match second {
            0..55 => TreeKind::Oak,
            55..88 => TreeKind::Birch,
            _ => TreeKind::BigOak,
        },
        BirchForest | BirchForestHills => TreeKind::Birch,
        BirchForestM | BirchForestHillsM => {
            if bits & 1 == 0 {
                TreeKind::TallBirch
            } else {
                TreeKind::Birch
            }
        }
        RoofedForest | RoofedForestM => match second {
            0..80 => TreeKind::DarkOak,
            80..90 => TreeKind::Birch,
            _ => TreeKind::Oak,
        },
        Jungle | JungleHills | JungleM => match second {
            0..10 => TreeKind::BigOak,
            10..55 => TreeKind::Shrub,
            55..70 => TreeKind::MegaJungle,
            _ => TreeKind::JungleTree,
        },
        JungleEdge | JungleEdgeM => match second {
            0..10 => TreeKind::BigOak,
            10..55 => TreeKind::Shrub,
            _ => TreeKind::JungleTree,
        },
        Savanna | SavannaM | SavannaPlateau | SavannaPlateauM => {
            if second < 80 {
                TreeKind::Acacia
            } else {
                TreeKind::Oak
            }
        }
        MegaTaiga | MegaTaigaHills if second < MEGA_TAIGA_PERCENT => {
            if (bits >> 8).is_multiple_of(MEGA_SPRUCE_ONE_IN) {
                TreeKind::MegaSpruce
            } else {
                TreeKind::MegaPine
            }
        }
        MegaSpruceTaiga | MegaSpruceTaigaHills if second < MEGA_TAIGA_PERCENT => {
            TreeKind::MegaSpruce
        }
        Taiga | TaigaHills | TaigaM | ColdTaiga | ColdTaigaHills | ColdTaigaM | MegaTaiga
        | MegaTaigaHills | MegaSpruceTaiga | MegaSpruceTaigaHills => {
            if (bits >> 2).is_multiple_of(PINE_ONE_IN) {
                TreeKind::Pine
            } else {
                TreeKind::Spruce
            }
        }
        ExtremeHillsPlus | ExtremeHillsPlusM => {
            if (bits >> 2) % HILLS_SPRUCE_OF == HILLS_SPRUCE_OF - 1 {
                TreeKind::Oak
            } else {
                TreeKind::Spruce
            }
        }
        IcePlains | IceMountains | IceSpikes => TreeKind::Spruce,
        Swampland | SwamplandM => TreeKind::SwampOak,
        MushroomIsland => {
            if bits & 1 == 0 {
                TreeKind::BrownMushroom
            } else {
                TreeKind::RedMushroom
            }
        }
        _ => TreeKind::Oak,
    }
}

pub fn plant(terrain: &Terrain, key: ColumnPos, column: &mut Column) {
    plant_where(terrain, key, column, |_| true);
}

pub fn plant_where(
    terrain: &Terrain,
    key: ColumnPos,
    column: &mut Column,
    allowed: impl Fn(&Tree) -> bool,
) {
    let origin = key.origin();
    let range = |start: i32| start - MAX_REACH..start + SECTION_SIZE + MAX_REACH;
    let trees: Vec<Tree> = range(origin.z)
        .flat_map(|z| range(origin.x).map(move |x| (x, z)))
        .filter_map(|(x, z)| tree_at(terrain, x, z))
        .filter(|tree| allowed(tree))
        .collect();
    let built: Vec<shapes::Parts> = trees
        .iter()
        .map(|tree| {
            tree.parts().unwrap_or_else(|| shapes::Parts {
                logs: tree.logs(),
                leaves: tree.leaves(),
                vines: Vec::new(),
            })
        })
        .collect();
    for (tree, parts) in trees.iter().zip(&built) {
        for &(log, axis) in &parts.logs {
            let local = log - origin;
            if Column::contains(local) {
                column.set_with_data(local, tree.kind.log(), axis);
            }
        }
    }
    for tree in &trees {
        for spot in tree.podzol() {
            let local = spot - origin;
            if !(0..SECTION_SIZE).contains(&local.x) || !(0..SECTION_SIZE).contains(&local.z) {
                continue;
            }
            for dy in (-PODZOL_DEPTH..=PODZOL_HEIGHT).rev() {
                let at = local + IVec3::Y * dy;
                if !Column::contains(at) {
                    continue;
                }
                let block = column.get(at);
                if matches!(block, Block::Grass | Block::Dirt | Block::CoarseDirt) {
                    column.set(at, Block::Podzol);
                    break;
                }
                if block != Block::Air && dy < 0 {
                    break;
                }
            }
        }
    }
    for (tree, parts) in trees.iter().zip(&built) {
        for &leaf in &parts.leaves {
            let local = leaf - origin;
            if Column::contains(local) && column.get(local) == Block::Air {
                column.set(local, tree.kind.leaves());
            }
        }
    }
    for parts in &built {
        for vine in &parts.vines {
            for drop in 0..=vine.drop {
                let local = vine.cell - IVec3::Y * drop - origin;
                if !Column::contains(local) || column.get(local) != Block::Air {
                    break;
                }
                column.set_with_data(local, Block::Vine, vine.side);
            }
        }
    }
    for pod in trees.iter().flat_map(Tree::cocoas) {
        let local = pod.0 - origin;
        if Column::contains(local) && column.get(local) == Block::Air {
            column.set_with_data(local, Block::Cocoa, pod.1);
        }
    }
    for (tree, parts) in trees.iter().zip(&built) {
        if !terrain.biome_at(tree.base.x, tree.base.z).is_snowy() {
            continue;
        }
        for &leaf in &parts.leaves {
            let local = leaf - origin;
            let above = local + IVec3::Y;
            if Column::contains(above)
                && column.get(local) == tree.kind.leaves()
                && column.get(above) == Block::Air
            {
                column.set(above, Block::Snow);
            }
        }
    }
}

pub(crate) fn hash(seed: i32, x: i32, z: i32, salt: u64) -> u32 {
    let point = (u64::from(x as u32) << 32) | u64::from(z as u32);
    let stream = splitmix64((u64::from(seed as u32) << 8) | salt);
    (splitmix64(point ^ stream) >> 32) as u32
}

pub(crate) fn splitmix64(mut v: u64) -> u64 {
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

mod shapes;

#[cfg(test)]
mod tests;
