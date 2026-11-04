use std::f64::consts::PI;

use super::{MineshaftKind, StructureKind, TempleKind, Variant, showcase};
use crate::biome::Biome;
use crate::biome_map::BiomeMap;
use crate::pos::ColumnPos;
use crate::stream::Stream;
use crate::terrain::{Terrain, WorldType};
use crate::trees::hash;

const SALT_VILLAGE: u64 = 10_387_312;
const SALT_TEMPLE: u64 = 14_357_617;
const SALT_MONUMENT: u64 = 10_387_313;
const SALT_MANSION: u64 = 10_387_319;
const SALT_MINESHAFT: u64 = 10_387_320;
const SALT_STRONGHOLD: u64 = 10_387_321;

const MINESHAFT_CHANCE: f64 = 0.004;
const MINESHAFT_DISTANCE_ODDS: u32 = 80;
const MONUMENT_WATER_RADIUS: i32 = 29;
const MANSION_RADIUS: i32 = 32;
const COLUMN_CENTER: i32 = 8;
const BIOME_CELL_SHIFT: i32 = 2;

pub const STRONGHOLD_COUNT: usize = 128;
const STRONGHOLD_DISTANCE: f64 = 32.0;
const STRONGHOLD_SPREAD: i32 = 3;
const STRONGHOLD_SEARCH: i32 = 112;

pub const VILLAGE_BIOMES: [Biome; 4] = [Biome::Plains, Biome::Desert, Biome::Savanna, Biome::Taiga];
pub const TEMPLE_BIOMES: [Biome; 7] = [
    Biome::Desert,
    Biome::DesertHills,
    Biome::Jungle,
    Biome::JungleHills,
    Biome::Swampland,
    Biome::IcePlains,
    Biome::ColdTaiga,
];
pub const MONUMENT_WATER: [Biome; 5] = [
    Biome::DeepOcean,
    Biome::Ocean,
    Biome::River,
    Biome::FrozenOcean,
    Biome::FrozenRiver,
];
pub const MANSION_BIOMES: [Biome; 2] = [Biome::RoofedForest, Biome::RoofedForestM];

#[derive(Clone, Copy)]
pub(super) struct Grid {
    pub spacing: i32,
    pub separation: i32,
    salt: u64,
    triangular: bool,
}

impl StructureKind {
    pub(super) fn grid(self) -> Option<Grid> {
        let (spacing, separation, salt, triangular) = match self {
            StructureKind::Village => (32, 8, SALT_VILLAGE, false),
            StructureKind::Temple => (32, 8, SALT_TEMPLE, false),
            StructureKind::Monument => (32, 5, SALT_MONUMENT, true),
            StructureKind::Mansion => (80, 20, SALT_MANSION, true),
            StructureKind::Mineshaft | StructureKind::Stronghold => return None,
        };
        Some(Grid {
            spacing,
            separation,
            salt,
            triangular,
        })
    }
}

pub(super) fn stream(seed: i32, x: i32, z: i32, salt: u64) -> Stream {
    Stream::new(u64::from(hash(seed, x, z, salt)) | u64::from(hash(seed, x, z, salt + 1)) << 32)
}

pub(super) fn grid_origin(seed: i32, grid: Grid, rx: i32, rz: i32) -> ColumnPos {
    let mut rng = stream(seed, rx, rz, grid.salt);
    let range = (grid.spacing - grid.separation) as u32;
    let roll = |rng: &mut Stream| {
        if grid.triangular {
            ((rng.below(range) + rng.below(range)) / 2) as i32
        } else {
            rng.below(range) as i32
        }
    };
    let dx = roll(&mut rng);
    let dz = roll(&mut rng);
    ColumnPos::new(rx * grid.spacing + dx, rz * grid.spacing + dz)
}

pub(super) fn mineshaft_at(seed: i32, cx: i32, cz: i32) -> bool {
    let mut rng = stream(seed, cx, cz, SALT_MINESHAFT);
    rng.fraction() < MINESHAFT_CHANCE
        && rng.below(MINESHAFT_DISTANCE_ODDS) < cx.abs().max(cz.abs()) as u32
}

/// Места стартов вида в квадрате `reach` колонок вокруг `center`, без проверки биома.
pub(super) fn origins(
    terrain: &Terrain,
    kind: StructureKind,
    center: ColumnPos,
    reach: i32,
) -> Vec<ColumnPos> {
    let near = |origin: &ColumnPos| {
        (origin.x() - center.x()).abs() <= reach && (origin.z() - center.z()).abs() <= reach
    };
    match terrain.world_type() {
        WorldType::Standard => {}
        WorldType::Structures => {
            return showcase::plots()
                .filter(|plot| plot.kind == kind)
                .map(|plot| plot.column())
                .filter(near)
                .collect();
        }
        WorldType::Flat | WorldType::Debug => return Vec::new(),
    }
    let seed = terrain.seed();
    let (x0, x1, z0, z1) = (
        center.x() - reach,
        center.x() + reach,
        center.z() - reach,
        center.z() + reach,
    );
    if let Some(grid) = kind.grid() {
        let regions = |c0: i32, c1: i32| c0.div_euclid(grid.spacing)..=c1.div_euclid(grid.spacing);
        return regions(z0, z1)
            .flat_map(|rz| regions(x0, x1).map(move |rx| grid_origin(seed, grid, rx, rz)))
            .filter(near)
            .collect();
    }
    match kind {
        StructureKind::Mineshaft => (z0..=z1)
            .flat_map(|cz| (x0..=x1).map(move |cx| (cx, cz)))
            .filter(|&(cx, cz)| mineshaft_at(seed, cx, cz))
            .map(|(cx, cz)| ColumnPos::new(cx, cz))
            .collect(),
        _ => terrain
            .structures()
            .strongholds(terrain)
            .iter()
            .copied()
            .filter(near)
            .collect(),
    }
}

pub(super) fn anchor(terrain: &Terrain, origin: ColumnPos) -> (i32, i32) {
    if terrain.world_type() == WorldType::Structures
        && let Some(plot) = showcase::plots().find(|plot| plot.column() == origin)
    {
        return (plot.center.x, plot.center.y);
    }
    (
        origin.x() * 16 + COLUMN_CENTER,
        origin.z() * 16 + COLUMN_CENTER,
    )
}

/// `BiomeProvider.areBiomesViable`: все клетки четверть-масштабного слоя в радиусе подходят.
fn viable(terrain: &Terrain, x: i32, z: i32, radius: i32, allowed: impl Fn(Biome) -> bool) -> bool {
    if terrain.world_type() != WorldType::Standard {
        return allowed(terrain.biome_at(x, z));
    }
    let i = (x - radius) >> BIOME_CELL_SHIFT;
    let j = (z - radius) >> BIOME_CELL_SHIFT;
    let w = (((x + radius) >> BIOME_CELL_SHIFT) - i + 1) as usize;
    let h = (((z + radius) >> BIOME_CELL_SHIFT) - j + 1) as usize;
    terrain
        .biome_map()
        .coarse(i, j, w, h)
        .data
        .iter()
        .all(|&id| Biome::from_id(id).is_some_and(&allowed))
}

pub(super) fn variant(
    terrain: &Terrain,
    kind: StructureKind,
    origin: ColumnPos,
) -> Option<Variant> {
    let (x, z) = anchor(terrain, origin);
    let center = terrain.biome_at(x, z);
    match kind {
        StructureKind::Village => {
            viable(terrain, x, z, 0, |b| VILLAGE_BIOMES.contains(&b)).then_some(Variant::Plain)
        }
        StructureKind::Temple => {
            if !viable(terrain, x, z, 0, |b| TEMPLE_BIOMES.contains(&b)) {
                return None;
            }
            let kind = match center {
                Biome::Desert | Biome::DesertHills => TempleKind::DesertPyramid,
                Biome::Jungle | Biome::JungleHills => TempleKind::JunglePyramid,
                Biome::Swampland => TempleKind::SwampHut,
                Biome::IcePlains | Biome::ColdTaiga => TempleKind::Igloo,
                _ => return None,
            };
            Some(Variant::Temple(kind))
        }
        StructureKind::Mineshaft => Some(Variant::Mineshaft(if center.is_mesa() {
            MineshaftKind::Mesa
        } else {
            MineshaftKind::Normal
        })),
        StructureKind::Stronghold => Some(Variant::Plain),
        StructureKind::Monument => (center == Biome::DeepOcean
            && viable(terrain, x, z, MONUMENT_WATER_RADIUS, |b| {
                MONUMENT_WATER.contains(&b)
            }))
        .then_some(Variant::Plain),
        StructureKind::Mansion => viable(terrain, x, z, MANSION_RADIUS, |b| {
            MANSION_BIOMES.contains(&b)
        })
        .then_some(Variant::Plain),
    }
}

/// `BiomeProvider.findBiomePosition`: случайная подходящая клетка в квадрате `range` блоков.
fn find_biome_position(
    map: &BiomeMap,
    x: i32,
    z: i32,
    range: i32,
    rng: &mut Stream,
) -> Option<(i32, i32)> {
    let i = (x - range) >> BIOME_CELL_SHIFT;
    let j = (z - range) >> BIOME_CELL_SHIFT;
    let w = (((x + range) >> BIOME_CELL_SHIFT) - i + 1) as usize;
    let h = (((z + range) >> BIOME_CELL_SHIFT) - j + 1) as usize;
    let cells = map.coarse(i, j, w, h);
    let mut found = None;
    let mut matches = 0;
    for (index, &id) in cells.data.iter().enumerate() {
        let strong = Biome::from_id(id).is_some_and(|biome| biome.base_height() > 0.0);
        if strong && (found.is_none() || rng.below(matches + 1) == 0) {
            let (cell_x, cell_z) = ((index % w) as i32, (index / w) as i32);
            found = Some((
                (i + cell_x) << BIOME_CELL_SHIFT,
                (j + cell_z) << BIOME_CELL_SHIFT,
            ));
            matches += 1;
        }
    }
    found
}

/// `ChunkGeneratorSettings` крепостей 1.12: кольца по distance 32, spread 3, всего 128.
pub(super) fn stronghold_ring(terrain: &Terrain) -> Vec<ColumnPos> {
    if terrain.world_type() == WorldType::Structures {
        return showcase::plots()
            .filter(|plot| plot.kind == StructureKind::Stronghold)
            .map(|plot| plot.column())
            .collect();
    }
    if terrain.world_type() != WorldType::Standard {
        return Vec::new();
    }
    let mut rng = stream(terrain.seed(), 0, 0, SALT_STRONGHOLD);
    let mut angle = rng.fraction() * PI * 2.0;
    let (mut ring, mut in_ring, mut spread) = (0, 0, STRONGHOLD_SPREAD);
    let mut out = Vec::with_capacity(STRONGHOLD_COUNT);
    for index in 0..STRONGHOLD_COUNT as i32 {
        let distance = 4.0 * STRONGHOLD_DISTANCE
            + STRONGHOLD_DISTANCE * f64::from(ring) * 6.0
            + (rng.fraction() - 0.5) * STRONGHOLD_DISTANCE * 2.5;
        let mut cx = (angle.cos() * distance + 0.5).floor() as i32;
        let mut cz = (angle.sin() * distance + 0.5).floor() as i32;
        let spot = find_biome_position(
            terrain.biome_map(),
            (cx << 4) + COLUMN_CENTER,
            (cz << 4) + COLUMN_CENTER,
            STRONGHOLD_SEARCH,
            &mut rng,
        );
        if let Some((x, z)) = spot {
            (cx, cz) = (x >> 4, z >> 4);
        }
        out.push(ColumnPos::new(cx, cz));
        angle += PI * 2.0 / f64::from(spread);
        in_ring += 1;
        if in_ring == spread {
            ring += 1;
            in_ring = 0;
            spread += 2 * spread / (ring + 1);
            spread = spread.min(STRONGHOLD_COUNT as i32 - index);
            angle += rng.fraction() * PI * 2.0;
        }
    }
    out
}
