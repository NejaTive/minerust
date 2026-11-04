use glam::{IVec2, IVec3};

use super::{MineshaftKind, StructureKind, TempleKind, Variant};
use crate::biome::Biome;
use crate::pos::ColumnPos;
use crate::terrain::WATER_LEVEL;

pub const PLOT_STEP: i32 = 192;
pub const PLOT_HALF: i32 = 80;
pub const FIRST_PLOT_X: i32 = 96;
pub const LAND_SURFACE: i32 = WATER_LEVEL + 1;
pub const SEABED_SURFACE: i32 = 40;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ground {
    Land,
    Ocean,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Plot {
    pub kind: StructureKind,
    pub variant: Variant,
    pub center: IVec2,
    pub biome: Biome,
    pub ground: Ground,
}

impl Plot {
    pub fn surface(&self) -> i32 {
        match self.ground {
            Ground::Land => LAND_SURFACE,
            Ground::Ocean => SEABED_SURFACE,
        }
    }

    pub fn column(&self) -> ColumnPos {
        ColumnPos::of_block(IVec3::new(self.center.x, 0, self.center.y))
    }

    fn contains(&self, x: i32, z: i32) -> bool {
        (x - self.center.x).abs() <= PLOT_HALF && (z - self.center.y).abs() <= PLOT_HALF
    }
}

const SPECS: [(StructureKind, Variant, Biome, Ground); 13] = [
    (
        StructureKind::Village,
        Variant::Plain,
        Biome::Plains,
        Ground::Land,
    ),
    (
        StructureKind::Village,
        Variant::Plain,
        Biome::Desert,
        Ground::Land,
    ),
    (
        StructureKind::Village,
        Variant::Plain,
        Biome::Savanna,
        Ground::Land,
    ),
    (
        StructureKind::Village,
        Variant::Plain,
        Biome::Taiga,
        Ground::Land,
    ),
    (
        StructureKind::Temple,
        Variant::Temple(TempleKind::DesertPyramid),
        Biome::Desert,
        Ground::Land,
    ),
    (
        StructureKind::Temple,
        Variant::Temple(TempleKind::JunglePyramid),
        Biome::Jungle,
        Ground::Land,
    ),
    (
        StructureKind::Temple,
        Variant::Temple(TempleKind::SwampHut),
        Biome::Swampland,
        Ground::Land,
    ),
    (
        StructureKind::Temple,
        Variant::Temple(TempleKind::Igloo),
        Biome::IcePlains,
        Ground::Land,
    ),
    (
        StructureKind::Mineshaft,
        Variant::Mineshaft(MineshaftKind::Normal),
        Biome::Plains,
        Ground::Land,
    ),
    (
        StructureKind::Mineshaft,
        Variant::Mineshaft(MineshaftKind::Mesa),
        Biome::Mesa,
        Ground::Land,
    ),
    (
        StructureKind::Stronghold,
        Variant::Plain,
        Biome::Plains,
        Ground::Land,
    ),
    (
        StructureKind::Monument,
        Variant::Plain,
        Biome::DeepOcean,
        Ground::Ocean,
    ),
    (
        StructureKind::Mansion,
        Variant::Plain,
        Biome::RoofedForest,
        Ground::Land,
    ),
];

pub fn plots() -> impl Iterator<Item = Plot> {
    SPECS
        .into_iter()
        .enumerate()
        .map(|(index, (kind, variant, biome, ground))| Plot {
            kind,
            variant,
            center: IVec2::new(FIRST_PLOT_X + index as i32 * PLOT_STEP, 0),
            biome,
            ground,
        })
}

pub fn plot_at(x: i32, z: i32) -> Option<Plot> {
    let index = (x - FIRST_PLOT_X + PLOT_STEP / 2).div_euclid(PLOT_STEP);
    plots()
        .nth(usize::try_from(index).ok()?)
        .filter(|plot| plot.contains(x, z))
}

pub fn biome(x: i32, z: i32) -> Biome {
    plot_at(x, z).map_or(Biome::Plains, |plot| plot.biome)
}

pub fn surface(x: i32, z: i32) -> i32 {
    plot_at(x, z).map_or(LAND_SURFACE, |plot| plot.surface())
}
