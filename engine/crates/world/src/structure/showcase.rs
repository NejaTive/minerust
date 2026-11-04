use std::sync::LazyLock;

use glam::{IVec2, IVec3};

use super::{MineshaftKind, StructureKind, TempleKind, Variant};
use crate::biome::Biome;
use crate::pos::ColumnPos;
use crate::terrain::WATER_LEVEL;

/// Левый край первого участка; (0, 0) остаётся свободным для точки появления.
pub const FIRST_PLOT_X: i32 = 16;
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
    /// Сторона квадратного участка; соседние участки стоят вплотную.
    pub width: i32,
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
        let half = self.width / 2;
        (self.center.x - half..self.center.x + half).contains(&x)
            && (self.center.y - half..self.center.y + half).contains(&z)
    }
}

const TEMPLE: i32 = 48;
const VILLAGE: i32 = 192;

const SPECS: [(StructureKind, Variant, Biome, Ground, i32); 13] = [
    (
        StructureKind::Temple,
        Variant::Temple(TempleKind::SwampHut),
        Biome::Swampland,
        Ground::Land,
        TEMPLE,
    ),
    (
        StructureKind::Temple,
        Variant::Temple(TempleKind::Igloo),
        Biome::IcePlains,
        Ground::Land,
        TEMPLE,
    ),
    (
        StructureKind::Temple,
        Variant::Temple(TempleKind::DesertPyramid),
        Biome::Desert,
        Ground::Land,
        TEMPLE,
    ),
    (
        StructureKind::Temple,
        Variant::Temple(TempleKind::JunglePyramid),
        Biome::Jungle,
        Ground::Land,
        TEMPLE,
    ),
    (
        StructureKind::Mineshaft,
        Variant::Mineshaft(MineshaftKind::Normal),
        Biome::Plains,
        Ground::Land,
        VILLAGE,
    ),
    (
        StructureKind::Mineshaft,
        Variant::Mineshaft(MineshaftKind::Mesa),
        Biome::Mesa,
        Ground::Land,
        VILLAGE,
    ),
    (
        StructureKind::Village,
        Variant::Plain,
        Biome::Plains,
        Ground::Land,
        VILLAGE,
    ),
    (
        StructureKind::Village,
        Variant::Plain,
        Biome::Desert,
        Ground::Land,
        VILLAGE,
    ),
    (
        StructureKind::Village,
        Variant::Plain,
        Biome::Savanna,
        Ground::Land,
        VILLAGE,
    ),
    (
        StructureKind::Village,
        Variant::Plain,
        Biome::Taiga,
        Ground::Land,
        VILLAGE,
    ),
    (
        StructureKind::Monument,
        Variant::Plain,
        Biome::DeepOcean,
        Ground::Ocean,
        96,
    ),
    (
        StructureKind::Stronghold,
        Variant::Plain,
        Biome::Plains,
        Ground::Land,
        256,
    ),
    (
        StructureKind::Mansion,
        Variant::Plain,
        Biome::RoofedForest,
        Ground::Land,
        128,
    ),
];

static PLOTS: LazyLock<Vec<Plot>> = LazyLock::new(|| {
    let mut edge = FIRST_PLOT_X;
    SPECS
        .into_iter()
        .map(|(kind, variant, biome, ground, width)| {
            let plot = Plot {
                kind,
                variant,
                center: IVec2::new(edge + width / 2, 0),
                biome,
                ground,
                width,
            };
            edge += width;
            plot
        })
        .collect()
});

pub fn plots() -> impl Iterator<Item = Plot> {
    PLOTS.iter().copied()
}

pub fn plot_at(x: i32, z: i32) -> Option<Plot> {
    PLOTS.iter().copied().find(|plot| plot.contains(x, z))
}

pub fn biome(x: i32, z: i32) -> Biome {
    plot_at(x, z).map_or(Biome::Plains, |plot| plot.biome)
}

pub fn surface(x: i32, z: i32) -> i32 {
    plot_at(x, z).map_or(LAND_SURFACE, |plot| plot.surface())
}
