pub const DEFAULT_TILE_SIZE: u32 = 32;

pub const PIXEL_GRID: u32 = 16;

pub const TILE_COUNT: usize = Tile::ALL.len();

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
    GrassTop,
    GrassSide,
    Dirt,
    Stone,
    Sand,
    LogSide,
    LogTop,
    Leaves,
    Water,
    Glowstone,
    TallGrass,
    Poppy,
    Dandelion,
    Torch,
    Gravel,
    Lava,
    Obsidian,
    Cobblestone,
    BirchLogSide,
    BirchLogTop,
    BirchLeaves,
    SpruceLogSide,
    SpruceLogTop,
    SpruceLeaves,
    Bedrock,
    SnowyGrassSide,
    Snow,
    Ice,
    CactusTop,
    CactusSide,
    CactusBottom,
    DeadBush,
    RedSand,
    Terracotta,
    TerracottaWhite,
    TerracottaOrange,
    TerracottaYellow,
    TerracottaBrown,
    TerracottaRed,
    TerracottaSilver,
    Reeds,
    SandstoneTop,
    SandstoneSide,
    SandstoneBottom,
    RedSandstoneTop,
    RedSandstoneSide,
    RedSandstoneBottom,
    JungleLogSide,
    JungleLogTop,
    JungleLeaves,
    AcaciaLogSide,
    AcaciaLogTop,
    AcaciaLeaves,
    DarkOakLogSide,
    DarkOakLogTop,
    DarkOakLeaves,
    Vine,
    MelonSide,
    MelonTop,
    MushroomSkinBrown,
    MushroomSkinRed,
    MushroomStem,
    MushroomInside,
    Waterlily,
    MyceliumTop,
    MyceliumSide,
    BrownMushroom,
    RedMushroom,
    Clay,
    IcePacked,
    PodzolTop,
    PodzolSide,
    CoarseDirt,
    MossyCobblestone,
    Fern,
    DoubleGrassBottom,
    DoubleGrassTop,
    DoubleFernBottom,
    DoubleFernTop,
    SunflowerBottom,
    SunflowerTop,
    SunflowerFront,
    SunflowerBack,
    SyringaBottom,
    SyringaTop,
    RoseBushBottom,
    RoseBushTop,
    PeonyBottom,
    PeonyTop,
    Allium,
    BlueOrchid,
    Houstonia,
    OxeyeDaisy,
    TulipRed,
    TulipOrange,
    TulipWhite,
    TulipPink,
    PumpkinSide,
    PumpkinTop,
    PumpkinFace,
    StoneGranite,
    StoneDiorite,
    StoneAndesite,
    CoalOre,
    IronOre,
    GoldOre,
    RedstoneOre,
    DiamondOre,
    LapisOre,
    EmeraldOre,
}

impl Tile {
    pub const ALL: [Tile; 110] = [
        Tile::GrassTop,
        Tile::GrassSide,
        Tile::Dirt,
        Tile::Stone,
        Tile::Sand,
        Tile::LogSide,
        Tile::LogTop,
        Tile::Leaves,
        Tile::Water,
        Tile::Glowstone,
        Tile::TallGrass,
        Tile::Poppy,
        Tile::Dandelion,
        Tile::Torch,
        Tile::Gravel,
        Tile::Lava,
        Tile::Obsidian,
        Tile::Cobblestone,
        Tile::BirchLogSide,
        Tile::BirchLogTop,
        Tile::BirchLeaves,
        Tile::SpruceLogSide,
        Tile::SpruceLogTop,
        Tile::SpruceLeaves,
        Tile::Bedrock,
        Tile::SnowyGrassSide,
        Tile::Snow,
        Tile::Ice,
        Tile::CactusTop,
        Tile::CactusSide,
        Tile::CactusBottom,
        Tile::DeadBush,
        Tile::RedSand,
        Tile::Terracotta,
        Tile::TerracottaWhite,
        Tile::TerracottaOrange,
        Tile::TerracottaYellow,
        Tile::TerracottaBrown,
        Tile::TerracottaRed,
        Tile::TerracottaSilver,
        Tile::Reeds,
        Tile::SandstoneTop,
        Tile::SandstoneSide,
        Tile::SandstoneBottom,
        Tile::RedSandstoneTop,
        Tile::RedSandstoneSide,
        Tile::RedSandstoneBottom,
        Tile::JungleLogSide,
        Tile::JungleLogTop,
        Tile::JungleLeaves,
        Tile::AcaciaLogSide,
        Tile::AcaciaLogTop,
        Tile::AcaciaLeaves,
        Tile::DarkOakLogSide,
        Tile::DarkOakLogTop,
        Tile::DarkOakLeaves,
        Tile::Vine,
        Tile::MelonSide,
        Tile::MelonTop,
        Tile::MushroomSkinBrown,
        Tile::MushroomSkinRed,
        Tile::MushroomStem,
        Tile::MushroomInside,
        Tile::Waterlily,
        Tile::MyceliumTop,
        Tile::MyceliumSide,
        Tile::BrownMushroom,
        Tile::RedMushroom,
        Tile::Clay,
        Tile::IcePacked,
        Tile::PodzolTop,
        Tile::PodzolSide,
        Tile::CoarseDirt,
        Tile::MossyCobblestone,
        Tile::Fern,
        Tile::DoubleGrassBottom,
        Tile::DoubleGrassTop,
        Tile::DoubleFernBottom,
        Tile::DoubleFernTop,
        Tile::SunflowerBottom,
        Tile::SunflowerTop,
        Tile::SunflowerFront,
        Tile::SunflowerBack,
        Tile::SyringaBottom,
        Tile::SyringaTop,
        Tile::RoseBushBottom,
        Tile::RoseBushTop,
        Tile::PeonyBottom,
        Tile::PeonyTop,
        Tile::Allium,
        Tile::BlueOrchid,
        Tile::Houstonia,
        Tile::OxeyeDaisy,
        Tile::TulipRed,
        Tile::TulipOrange,
        Tile::TulipWhite,
        Tile::TulipPink,
        Tile::PumpkinSide,
        Tile::PumpkinTop,
        Tile::PumpkinFace,
        Tile::StoneGranite,
        Tile::StoneDiorite,
        Tile::StoneAndesite,
        Tile::CoalOre,
        Tile::IronOre,
        Tile::GoldOre,
        Tile::RedstoneOre,
        Tile::DiamondOre,
        Tile::LapisOre,
        Tile::EmeraldOre,
    ];

    pub fn sway(self) -> f32 {
        match self {
            Tile::Leaves
            | Tile::BirchLeaves
            | Tile::SpruceLeaves
            | Tile::JungleLeaves
            | Tile::AcaciaLeaves
            | Tile::DarkOakLeaves => 1.0,
            Tile::TallGrass
            | Tile::Poppy
            | Tile::Dandelion
            | Tile::DeadBush
            | Tile::Reeds
            | Tile::Fern
            | Tile::DoubleGrassBottom
            | Tile::DoubleGrassTop
            | Tile::DoubleFernBottom
            | Tile::DoubleFernTop
            | Tile::SunflowerBottom
            | Tile::SunflowerTop
            | Tile::SyringaBottom
            | Tile::SyringaTop
            | Tile::RoseBushBottom
            | Tile::RoseBushTop
            | Tile::PeonyBottom
            | Tile::PeonyTop
            | Tile::Allium
            | Tile::BlueOrchid
            | Tile::Houstonia
            | Tile::OxeyeDaisy
            | Tile::TulipRed
            | Tile::TulipOrange
            | Tile::TulipWhite
            | Tile::TulipPink => 2.0,
            _ => 0.0,
        }
    }

    pub fn emissive(self) -> bool {
        matches!(self, Tile::Glowstone | Tile::Lava | Tile::Torch)
    }

    pub const fn layer(self) -> u32 {
        self as u32
    }

    pub fn is_cutout(self) -> bool {
        matches!(
            self,
            Tile::Leaves
                | Tile::BirchLeaves
                | Tile::SpruceLeaves
                | Tile::JungleLeaves
                | Tile::AcaciaLeaves
                | Tile::DarkOakLeaves
                | Tile::Vine
                | Tile::TallGrass
                | Tile::Poppy
                | Tile::Dandelion
                | Tile::Torch
                | Tile::CactusTop
                | Tile::CactusSide
                | Tile::CactusBottom
                | Tile::DeadBush
                | Tile::Reeds
                | Tile::Waterlily
                | Tile::BrownMushroom
                | Tile::RedMushroom
                | Tile::Fern
                | Tile::DoubleGrassBottom
                | Tile::DoubleGrassTop
                | Tile::DoubleFernBottom
                | Tile::DoubleFernTop
                | Tile::SunflowerBottom
                | Tile::SunflowerTop
                | Tile::SyringaBottom
                | Tile::SyringaTop
                | Tile::RoseBushBottom
                | Tile::RoseBushTop
                | Tile::PeonyBottom
                | Tile::PeonyTop
                | Tile::Allium
                | Tile::BlueOrchid
                | Tile::Houstonia
                | Tile::OxeyeDaisy
                | Tile::TulipRed
                | Tile::TulipOrange
                | Tile::TulipWhite
                | Tile::TulipPink
                | Tile::SunflowerFront
                | Tile::SunflowerBack
        )
    }
}

/// Прибавляется к колыханию в `.w` записи таблицы анимации; читает `shader.wgsl`.
pub const EMISSIVE_FLAG: f32 = 4.0;

pub const TORCH_PIXELS: (u32, u32) = (7, 9);
pub const TORCH_TOP_ROW: u32 = 6;

#[cfg(test)]
mod tests;
