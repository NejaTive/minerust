use crate::block::Block;

macro_rules! biomes {
    ($($variant:ident = $id:literal, $name:literal, ($base:literal, $spread:literal), $temperature:literal, $rainfall:literal;)*) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        #[repr(u8)]
        pub enum Biome {
            $($variant = $id),*
        }

        impl Biome {
            pub const ALL: &'static [Biome] = &[$(Biome::$variant),*];

            pub fn from_id(id: i32) -> Option<Biome> {
                match id {
                    $($id => Some(Biome::$variant),)*
                    _ => None,
                }
            }

            pub fn name(self) -> &'static str {
                match self {
                    $(Biome::$variant => $name,)*
                }
            }

            pub fn base_height(self) -> f32 {
                match self {
                    $(Biome::$variant => $base,)*
                }
            }

            pub fn height_variation(self) -> f32 {
                match self {
                    $(Biome::$variant => $spread,)*
                }
            }

            pub fn temperature(self) -> f32 {
                match self {
                    $(Biome::$variant => $temperature,)*
                }
            }

            pub fn rainfall(self) -> f32 {
                match self {
                    $(Biome::$variant => $rainfall,)*
                }
            }
        }
    };
}

biomes! {
    Ocean = 0, "ocean", (-1.0, 0.1), 0.5, 0.5;
    Plains = 1, "plains", (0.125, 0.05), 0.8, 0.4;
    Desert = 2, "desert", (0.125, 0.05), 2.0, 0.0;
    ExtremeHills = 3, "extreme_hills", (1.0, 0.5), 0.2, 0.3;
    Forest = 4, "forest", (0.1, 0.2), 0.7, 0.8;
    Taiga = 5, "taiga", (0.2, 0.2), 0.25, 0.8;
    Swampland = 6, "swampland", (-0.2, 0.1), 0.8, 0.9;
    River = 7, "river", (-0.5, 0.0), 0.5, 0.5;
    FrozenOcean = 10, "frozen_ocean", (-1.0, 0.1), 0.0, 0.5;
    FrozenRiver = 11, "frozen_river", (-0.5, 0.0), 0.0, 0.5;
    IcePlains = 12, "ice_plains", (0.125, 0.05), 0.0, 0.5;
    IceMountains = 13, "ice_mountains", (0.45, 0.3), 0.0, 0.5;
    MushroomIsland = 14, "mushroom_island", (0.2, 1.0), 0.9, 1.0;
    MushroomIslandShore = 15, "mushroom_island_shore", (-1.0, 0.1), 0.9, 1.0;
    Beach = 16, "beach", (0.0, 0.025), 0.8, 0.4;
    DesertHills = 17, "desert_hills", (0.45, 0.3), 2.0, 0.0;
    ForestHills = 18, "forest_hills", (0.45, 0.3), 0.7, 0.8;
    TaigaHills = 19, "taiga_hills", (0.45, 0.3), 0.25, 0.8;
    ExtremeHillsEdge = 20, "extreme_hills_edge", (0.8, 0.3), 0.2, 0.3;
    Jungle = 21, "jungle", (0.1, 0.2), 0.95, 0.9;
    JungleHills = 22, "jungle_hills", (0.45, 0.3), 0.95, 0.9;
    JungleEdge = 23, "jungle_edge", (0.1, 0.2), 0.95, 0.8;
    DeepOcean = 24, "deep_ocean", (-1.8, 0.1), 0.5, 0.5;
    StoneBeach = 25, "stone_beach", (0.1, 0.8), 0.2, 0.3;
    ColdBeach = 26, "cold_beach", (0.0, 0.025), 0.05, 0.3;
    BirchForest = 27, "birch_forest", (0.1, 0.2), 0.6, 0.6;
    BirchForestHills = 28, "birch_forest_hills", (0.45, 0.3), 0.6, 0.6;
    RoofedForest = 29, "roofed_forest", (0.1, 0.2), 0.7, 0.8;
    ColdTaiga = 30, "cold_taiga", (0.2, 0.2), -0.5, 0.4;
    ColdTaigaHills = 31, "cold_taiga_hills", (0.45, 0.3), -0.5, 0.4;
    MegaTaiga = 32, "mega_taiga", (0.2, 0.2), 0.3, 0.8;
    MegaTaigaHills = 33, "mega_taiga_hills", (0.45, 0.3), 0.3, 0.8;
    ExtremeHillsPlus = 34, "extreme_hills_plus", (1.0, 0.5), 0.2, 0.3;
    Savanna = 35, "savanna", (0.125, 0.05), 1.2, 0.0;
    SavannaPlateau = 36, "savanna_plateau", (1.5, 0.025), 1.0, 0.0;
    Mesa = 37, "mesa", (0.1, 0.2), 2.0, 0.0;
    MesaPlateauF = 38, "mesa_plateau_f", (1.5, 0.025), 2.0, 0.0;
    MesaPlateau = 39, "mesa_plateau", (1.5, 0.025), 2.0, 0.0;
    SunflowerPlains = 129, "sunflower_plains", (0.125, 0.05), 0.8, 0.4;
    DesertM = 130, "desert_m", (0.225, 0.25), 2.0, 0.0;
    ExtremeHillsM = 131, "extreme_hills_m", (1.0, 0.5), 0.2, 0.3;
    FlowerForest = 132, "flower_forest", (0.1, 0.4), 0.7, 0.8;
    TaigaM = 133, "taiga_m", (0.3, 0.4), 0.25, 0.8;
    SwamplandM = 134, "swampland_m", (-0.1, 0.3), 0.8, 0.9;
    IceSpikes = 140, "ice_spikes", (0.425, 0.45), 0.0, 0.5;
    JungleM = 149, "jungle_m", (0.2, 0.4), 0.95, 0.9;
    JungleEdgeM = 151, "jungle_edge_m", (0.2, 0.4), 0.95, 0.8;
    BirchForestM = 155, "birch_forest_m", (0.2, 0.4), 0.6, 0.6;
    BirchForestHillsM = 156, "birch_forest_hills_m", (0.45, 0.5), 0.6, 0.6;
    RoofedForestM = 157, "roofed_forest_m", (0.2, 0.4), 0.7, 0.8;
    ColdTaigaM = 158, "cold_taiga_m", (0.3, 0.4), -0.5, 0.4;
    MegaSpruceTaiga = 160, "mega_spruce_taiga", (0.2, 0.2), 0.25, 0.8;
    MegaSpruceTaigaHills = 161, "mega_spruce_taiga_hills", (0.45, 0.3), 0.25, 0.8;
    ExtremeHillsPlusM = 162, "extreme_hills_plus_m", (1.0, 0.5), 0.2, 0.3;
    SavannaM = 163, "savanna_m", (0.3625, 1.225), 1.1, 0.0;
    SavannaPlateauM = 164, "savanna_plateau_m", (1.05, 1.2125), 1.0, 0.0;
    MesaBryce = 165, "mesa_bryce", (0.1, 0.4), 2.0, 0.0;
    MesaPlateauFM = 166, "mesa_plateau_f_m", (0.45, 0.3), 2.0, 0.0;
    MesaPlateauM = 167, "mesa_plateau_m", (0.45, 0.3), 2.0, 0.0;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TempCategory {
    Ocean,
    Cold,
    Medium,
    Warm,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SurfaceBlocks {
    pub top: Block,
    pub filler: Block,
}

impl Biome {
    pub fn id(self) -> u8 {
        self as u8
    }

    pub fn is_ocean(self) -> bool {
        matches!(self, Biome::Ocean | Biome::DeepOcean | Biome::FrozenOcean)
    }

    pub fn is_river(self) -> bool {
        matches!(self, Biome::River | Biome::FrozenRiver)
    }

    pub fn is_snowy(self) -> bool {
        self.temperature() < 0.15
    }

    pub fn is_desert(self) -> bool {
        matches!(self, Biome::Desert | Biome::DesertHills | Biome::DesertM)
    }

    pub fn is_mesa(self) -> bool {
        matches!(
            self,
            Biome::Mesa
                | Biome::MesaPlateauF
                | Biome::MesaPlateau
                | Biome::MesaBryce
                | Biome::MesaPlateauFM
                | Biome::MesaPlateauM
        )
    }

    pub fn is_extreme_hills(self) -> bool {
        matches!(
            self,
            Biome::ExtremeHills
                | Biome::ExtremeHillsPlus
                | Biome::ExtremeHillsM
                | Biome::ExtremeHillsPlusM
                | Biome::ExtremeHillsEdge
        )
    }

    pub fn is_swamp(self) -> bool {
        matches!(self, Biome::Swampland | Biome::SwamplandM)
    }

    pub fn is_mushroom_island(self) -> bool {
        matches!(self, Biome::MushroomIsland | Biome::MushroomIslandShore)
    }

    pub fn is_roofed(self) -> bool {
        matches!(self, Biome::RoofedForest | Biome::RoofedForestM)
    }

    pub fn is_forest(self) -> bool {
        matches!(
            self,
            Biome::Forest
                | Biome::ForestHills
                | Biome::FlowerForest
                | Biome::BirchForest
                | Biome::BirchForestHills
                | Biome::BirchForestM
                | Biome::BirchForestHillsM
                | Biome::RoofedForest
                | Biome::RoofedForestM
        )
    }

    pub fn is_mega_taiga(self) -> bool {
        matches!(
            self,
            Biome::MegaTaiga
                | Biome::MegaTaigaHills
                | Biome::MegaSpruceTaiga
                | Biome::MegaSpruceTaigaHills
        )
    }

    pub fn is_taiga(self) -> bool {
        self.is_mega_taiga()
            || matches!(
                self,
                Biome::Taiga
                    | Biome::TaigaHills
                    | Biome::TaigaM
                    | Biome::ColdTaiga
                    | Biome::ColdTaigaHills
                    | Biome::ColdTaigaM
            )
    }

    pub fn is_jungle(self) -> bool {
        matches!(
            self,
            Biome::Jungle
                | Biome::JungleHills
                | Biome::JungleEdge
                | Biome::JungleM
                | Biome::JungleEdgeM
        )
    }

    pub fn temp_category(self) -> TempCategory {
        if self.is_ocean() {
            TempCategory::Ocean
        } else if self.temperature() < 0.2 {
            TempCategory::Cold
        } else if self.temperature() < 1.0 {
            TempCategory::Medium
        } else {
            TempCategory::Warm
        }
    }

    pub fn mutation(self) -> Option<Biome> {
        Some(match self {
            Biome::Plains => Biome::SunflowerPlains,
            Biome::Desert => Biome::DesertM,
            Biome::ExtremeHills => Biome::ExtremeHillsM,
            Biome::Forest => Biome::FlowerForest,
            Biome::Taiga => Biome::TaigaM,
            Biome::Swampland => Biome::SwamplandM,
            Biome::IcePlains => Biome::IceSpikes,
            Biome::Jungle => Biome::JungleM,
            Biome::JungleEdge => Biome::JungleEdgeM,
            Biome::BirchForest => Biome::BirchForestM,
            Biome::BirchForestHills => Biome::BirchForestHillsM,
            Biome::RoofedForest => Biome::RoofedForestM,
            Biome::ColdTaiga => Biome::ColdTaigaM,
            Biome::MegaTaiga => Biome::MegaSpruceTaiga,
            Biome::MegaTaigaHills => Biome::MegaSpruceTaigaHills,
            Biome::ExtremeHillsPlus => Biome::ExtremeHillsPlusM,
            Biome::Savanna => Biome::SavannaM,
            Biome::SavannaPlateau => Biome::SavannaPlateauM,
            Biome::Mesa => Biome::MesaBryce,
            Biome::MesaPlateauF => Biome::MesaPlateauFM,
            Biome::MesaPlateau => Biome::MesaPlateauM,
            _ => return None,
        })
    }

    pub fn is_mutation(self) -> bool {
        self.id() >= 128
    }

    /// Блоки поверхности без снега; подзол и крупнозём мега-тайги выбирает шум в `Terrain`.
    pub fn surface_blocks(self) -> SurfaceBlocks {
        let (top, filler) = match self {
            Biome::Desert
            | Biome::DesertHills
            | Biome::DesertM
            | Biome::Beach
            | Biome::ColdBeach => (Block::Sand, Block::Sand),
            Biome::StoneBeach => (Block::Stone, Block::Stone),
            Biome::MushroomIsland | Biome::MushroomIslandShore => (Block::Mycelium, Block::Dirt),
            _ if self.is_mesa() => (Block::RedSand, Block::OrangeTerracotta),
            _ => (Block::Grass, Block::Dirt),
        };
        SurfaceBlocks { top, filler }
    }
}

#[cfg(test)]
mod tests;
