use std::sync::LazyLock;

use glam::{IVec3, Vec3};

use crate::boxes::{BoxList, VoxelBox};
use crate::textures::TextureSet;
use crate::tile::{PIXEL_GRID, TORCH_PIXELS, TORCH_TOP_ROW, Tile};

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Block {
    #[default]
    Air,
    Grass,
    Dirt,
    Stone,
    Sand,
    Log,
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
    BirchLog,
    BirchLeaves,
    SpruceLog,
    SpruceLeaves,
    Bedrock,
    SnowyGrass,
    Snow,
    Ice,
    Barrier,
    Cactus,
    DeadBush,
    RedSand,
    Terracotta,
    WhiteTerracotta,
    OrangeTerracotta,
    YellowTerracotta,
    BrownTerracotta,
    RedTerracotta,
    SilverTerracotta,
    Reeds,
    Sandstone,
    RedSandstone,
    JungleLog,
    JungleLeaves,
    AcaciaLog,
    AcaciaLeaves,
    DarkOakLog,
    DarkOakLeaves,
    Vine,
    Melon,
    BrownMushroomBlock,
    RedMushroomBlock,
    MushroomStem,
    Waterlily,
    Mycelium,
    BrownMushroom,
    RedMushroom,
    Clay,
    IcePacked,
    Podzol,
    CoarseDirt,
    MossyCobblestone,
    Fern,
    DoubleGrassLower,
    DoubleGrassUpper,
    DoubleFernLower,
    DoubleFernUpper,
    SunflowerLower,
    SunflowerUpper,
    LilacLower,
    LilacUpper,
    RoseBushLower,
    RoseBushUpper,
    PeonyLower,
    PeonyUpper,
    Allium,
    BlueOrchid,
    Houstonia,
    OxeyeDaisy,
    RedTulip,
    OrangeTulip,
    WhiteTulip,
    PinkTulip,
    Pumpkin,
    Granite,
    Diorite,
    Andesite,
    CoalOre,
    IronOre,
    GoldOre,
    RedstoneOre,
    DiamondOre,
    LapisOre,
    EmeraldOre,
    Cocoa,
    SnowBlock,
    MonsterEgg,
    BoneBlock,
    OakPlanks,
    SprucePlanks,
    BirchPlanks,
    JunglePlanks,
    AcaciaPlanks,
    DarkOakPlanks,
    StoneBricks,
    MossyStoneBricks,
    CrackedStoneBricks,
    ChiseledStoneBricks,
    ChiseledSandstone,
    SmoothSandstone,
    ChiseledRedSandstone,
    SmoothRedSandstone,
    Bricks,
    Glass,
    Wool,
    Bookshelf,
    CraftingTable,
    OakSlab,
    SpruceSlab,
    BirchSlab,
    JungleSlab,
    AcaciaSlab,
    DarkOakSlab,
    StoneSlab,
    SandstoneSlab,
    CobblestoneSlab,
    BrickSlab,
    StoneBrickSlab,
    RedSandstoneSlab,
    OakStairs,
    SpruceStairs,
    BirchStairs,
    JungleStairs,
    AcaciaStairs,
    DarkOakStairs,
    CobblestoneStairs,
    BrickStairs,
    StoneBrickStairs,
    SandstoneStairs,
    RedSandstoneStairs,
    Carpet,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum FluidKind {
    Water,
    Lava,
}

impl FluidKind {
    pub fn block(self) -> Block {
        match self {
            FluidKind::Water => Block::Water,
            FluidKind::Lava => Block::Lava,
        }
    }
}

pub const WATER_SURFACE: f32 = 0.875;
pub const SNOW_UNITS: u8 = 2;
pub const SNOW_HEIGHT: f32 = SNOW_UNITS as f32 / 16.0;
pub const CACTUS_INSET_UNITS: u8 = 1;
pub const CACTUS_INSET: f32 = CACTUS_INSET_UNITS as f32 / 16.0;
pub const CACTUS_HEIGHT: f32 = 15.0 / 16.0;
pub const LILY_UNITS: u8 = 1;
pub const LILY_HEIGHT: f32 = LILY_UNITS as f32 / 16.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RenderLayer {
    Opaque,
    Cutout,
    Translucent,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shape {
    Cube,
    Cross,
    Torch,
    Layer,
    Cactus,
    Vine,
    Lily,
    Sunflower,
    Cocoa,
    Boxes,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BoxKind {
    None,
    Slab,
    Stairs,
    Carpet,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DataKind {
    None,
    Axis,
    Attachment,
    Fluid,
    Sides,
    Facing,
    Cocoa,
    Color,
    Slab,
    Stairs,
}

pub const AXIS_Y: u8 = 0;
pub const AXIS_X: u8 = 1;
pub const AXIS_Z: u8 = 2;

pub const FACING_NEG_Z: u8 = 0;
pub const FACING_POS_X: u8 = 1;
pub const FACING_POS_Z: u8 = 2;
pub const FACING_NEG_X: u8 = 3;

pub const SLAB_BOTTOM: u8 = 0;
pub const SLAB_TOP: u8 = 1;
pub const SLAB_DOUBLE: u8 = 2;

pub const STAIRS_FACING_MASK: u8 = 3;
pub const STAIRS_TOP: u8 = 4;

pub const COLOR_COUNT: u8 = 16;
pub const CARPET_UNITS: u8 = 1;

pub const COLOR_NAMES: [&str; COLOR_COUNT as usize] = [
    "white",
    "orange",
    "magenta",
    "light_blue",
    "yellow",
    "lime",
    "pink",
    "gray",
    "silver",
    "cyan",
    "purple",
    "blue",
    "brown",
    "green",
    "red",
    "black",
];

pub const WOOL_LABELS: [&str; COLOR_COUNT as usize] = [
    "белая шерсть",
    "оранжевая шерсть",
    "пурпурная шерсть",
    "голубая шерсть",
    "жёлтая шерсть",
    "лаймовая шерсть",
    "розовая шерсть",
    "серая шерсть",
    "светло-серая шерсть",
    "бирюзовая шерсть",
    "фиолетовая шерсть",
    "синяя шерсть",
    "коричневая шерсть",
    "зелёная шерсть",
    "красная шерсть",
    "чёрная шерсть",
];

pub const CARPET_LABELS: [&str; COLOR_COUNT as usize] = [
    "белый ковёр",
    "оранжевый ковёр",
    "пурпурный ковёр",
    "голубой ковёр",
    "жёлтый ковёр",
    "лаймовый ковёр",
    "розовый ковёр",
    "серый ковёр",
    "светло-серый ковёр",
    "бирюзовый ковёр",
    "фиолетовый ковёр",
    "синий ковёр",
    "коричневый ковёр",
    "зелёный ковёр",
    "красный ковёр",
    "чёрный ковёр",
];

pub const WOOL_TILES: [Tile; COLOR_COUNT as usize] = [
    Tile::WoolWhite,
    Tile::WoolOrange,
    Tile::WoolMagenta,
    Tile::WoolLightBlue,
    Tile::WoolYellow,
    Tile::WoolLime,
    Tile::WoolPink,
    Tile::WoolGray,
    Tile::WoolSilver,
    Tile::WoolCyan,
    Tile::WoolPurple,
    Tile::WoolBlue,
    Tile::WoolBrown,
    Tile::WoolGreen,
    Tile::WoolRed,
    Tile::WoolBlack,
];

pub const COCOA_STAGES: u8 = 3;
pub const COCOA_DIRECTION_MASK: u8 = 3;
pub const COCOA_STAGE_SHIFT: u32 = 2;

#[derive(Clone, Copy)]
pub struct CocoaPod {
    pub width: i32,
    pub bottom: i32,
    pub top: i32,
}

pub const COCOA_PODS: [CocoaPod; COCOA_STAGES as usize] = [
    CocoaPod {
        width: 4,
        bottom: 7,
        top: 12,
    },
    CocoaPod {
        width: 6,
        bottom: 5,
        top: 12,
    },
    CocoaPod {
        width: 8,
        bottom: 2,
        top: 11,
    },
];
pub const COCOA_BACK: i32 = 15;
pub const COCOA_STEM: (i32, i32) = (7, 9);
pub const COCOA_STEM_DEPTH: i32 = 2;

pub const ATTACH_FLOOR: u8 = 0;
pub const ATTACH_POS_X: u8 = 1;
pub const ATTACH_NEG_X: u8 = 2;
pub const ATTACH_POS_Z: u8 = 3;
pub const ATTACH_NEG_Z: u8 = 4;

pub const VINE_POS_X: u8 = 1;
pub const VINE_NEG_X: u8 = 2;
pub const VINE_POS_Z: u8 = 4;
pub const VINE_NEG_Z: u8 = 8;
pub const VINE_TOP: u8 = 16;
pub const VINE_SIDES: [(u8, IVec3); 5] = [
    (VINE_POS_X, IVec3::X),
    (VINE_NEG_X, IVec3::NEG_X),
    (VINE_POS_Z, IVec3::Z),
    (VINE_NEG_Z, IVec3::NEG_Z),
    (VINE_TOP, IVec3::Y),
];
pub const VINE_INSET_UNITS: u8 = 1;
pub const VINE_INSET: f32 = VINE_INSET_UNITS as f32 / 16.0;

pub const FLUID_SOURCE: u8 = 0;
pub const FLUID_MAX_LEVEL: u8 = 7;
pub const FLUID_LEVEL_MASK: u8 = 0b111;
pub const FLUID_FALLING: u8 = 0b1000;

// шестнадцатые доли блока
const FLUID_LEVEL_HEIGHT_UNITS: [u8; 8] = [14, 12, 11, 9, 7, 5, 4, 2];

pub fn fluid_height_units(data: u8) -> u8 {
    if data & FLUID_FALLING != 0 {
        16
    } else {
        FLUID_LEVEL_HEIGHT_UNITS[usize::from(data & FLUID_LEVEL_MASK)]
    }
}

pub fn fluid_height(data: u8) -> f32 {
    f32::from(fluid_height_units(data)) / 16.0
}

pub const TORCH_WALL_SHIFT: u32 = 4;
pub const TORCH_WALL_RISE: u32 = 3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Facing {
    Top,
    Bottom,
    Side,
}

#[derive(Clone, Copy, Debug)]
pub struct BlockDef {
    pub name: &'static str,
    pub label: &'static str,
    pub selectable: bool,
    pub solid: bool,
    pub opaque: bool,
    pub targetable: bool,
    pub replaceable: bool,
    pub needs_support: bool,
    pub falls: bool,
    pub unbreakable: bool,
    pub random_ticks: bool,
    pub slippery: bool,
    pub data: DataKind,
    pub fluid: Option<FluidKind>,
    pub emission: u8,
    pub layer: RenderLayer,
    pub shape: Shape,
    pub boxes: BoxKind,
    pub light_blocking: bool,
    pub joins_same: bool,
    pub top: Option<Tile>,
    pub side: Option<Tile>,
    pub bottom: Option<Tile>,
}

impl BlockDef {
    const CUBE: BlockDef = BlockDef {
        name: "",
        label: "",
        selectable: true,
        solid: true,
        opaque: true,
        targetable: true,
        replaceable: false,
        needs_support: false,
        falls: false,
        unbreakable: false,
        random_ticks: false,
        slippery: false,
        data: DataKind::None,
        fluid: None,
        emission: 0,
        layer: RenderLayer::Opaque,
        shape: Shape::Cube,
        boxes: BoxKind::None,
        light_blocking: false,
        joins_same: false,
        top: None,
        side: None,
        bottom: None,
    };

    const fn uniform(name: &'static str, label: &'static str, tile: Tile) -> BlockDef {
        BlockDef {
            name,
            label,
            top: Some(tile),
            side: Some(tile),
            bottom: Some(tile),
            ..Self::CUBE
        }
    }

    const fn log(name: &'static str, label: &'static str, side: Tile, end: Tile) -> BlockDef {
        BlockDef {
            data: DataKind::Axis,
            top: Some(end),
            bottom: Some(end),
            ..Self::uniform(name, label, side)
        }
    }

    const fn boxed(
        name: &'static str,
        label: &'static str,
        kind: BoxKind,
        data: DataKind,
        (top, side, bottom): (Tile, Tile, Tile),
    ) -> BlockDef {
        BlockDef {
            opaque: false,
            light_blocking: true,
            data,
            shape: Shape::Boxes,
            boxes: kind,
            top: Some(top),
            bottom: Some(bottom),
            ..Self::uniform(name, label, side)
        }
    }

    const fn slab(
        name: &'static str,
        label: &'static str,
        top: Tile,
        side: Tile,
        bottom: Tile,
    ) -> BlockDef {
        Self::boxed(
            name,
            label,
            BoxKind::Slab,
            DataKind::Slab,
            (top, side, bottom),
        )
    }

    const fn stairs(
        name: &'static str,
        label: &'static str,
        top: Tile,
        side: Tile,
        bottom: Tile,
    ) -> BlockDef {
        Self::boxed(
            name,
            label,
            BoxKind::Stairs,
            DataKind::Stairs,
            (top, side, bottom),
        )
    }

    const fn leaves(name: &'static str, label: &'static str, tile: Tile) -> BlockDef {
        BlockDef {
            opaque: false,
            layer: RenderLayer::Cutout,
            ..Self::uniform(name, label, tile)
        }
    }

    const fn plant(name: &'static str, label: &'static str, tile: Tile) -> BlockDef {
        BlockDef {
            targetable: true,
            solid: false,
            opaque: false,
            needs_support: true,
            layer: RenderLayer::Cutout,
            shape: Shape::Cross,
            ..Self::uniform(name, label, tile)
        }
    }
}

/// формата сохранения: новые блоки только в конец, вместе с вариантом [`Block`] и
pub const BLOCKS: [BlockDef; 138] = [
    BlockDef {
        name: "air",
        label: "воздух",
        selectable: false,
        solid: false,
        opaque: false,
        targetable: false,
        replaceable: true,
        ..BlockDef::CUBE
    },
    BlockDef {
        top: Some(Tile::GrassTop),
        side: Some(Tile::GrassSide),
        bottom: Some(Tile::Dirt),
        random_ticks: true,
        ..BlockDef::uniform("grass", "трава", Tile::Dirt)
    },
    BlockDef {
        random_ticks: true,
        ..BlockDef::uniform("dirt", "земля", Tile::Dirt)
    },
    BlockDef::uniform("stone", "камень", Tile::Stone),
    BlockDef {
        falls: true,
        ..BlockDef::uniform("sand", "песок", Tile::Sand)
    },
    BlockDef::log("log", "бревно", Tile::LogSide, Tile::LogTop),
    BlockDef::leaves("leaves", "листва", Tile::Leaves),
    BlockDef {
        solid: false,
        opaque: false,
        targetable: false,
        replaceable: true,
        data: DataKind::Fluid,
        fluid: Some(FluidKind::Water),
        layer: RenderLayer::Translucent,
        ..BlockDef::uniform("water", "вода", Tile::Water)
    },
    BlockDef {
        emission: 15,
        ..BlockDef::uniform("glowstone", "светокамень", Tile::Glowstone)
    },
    BlockDef {
        replaceable: true,
        ..BlockDef::plant("tall_grass", "высокая трава", Tile::TallGrass)
    },
    BlockDef::plant("poppy", "мак", Tile::Poppy),
    BlockDef::plant("dandelion", "одуванчик", Tile::Dandelion),
    BlockDef {
        shape: Shape::Torch,
        data: DataKind::Attachment,
        emission: 14,
        ..BlockDef::plant("torch", "факел", Tile::Torch)
    },
    BlockDef {
        falls: true,
        ..BlockDef::uniform("gravel", "гравий", Tile::Gravel)
    },
    BlockDef {
        solid: false,
        opaque: false,
        targetable: false,
        replaceable: true,
        data: DataKind::Fluid,
        fluid: Some(FluidKind::Lava),
        emission: 15,
        ..BlockDef::uniform("lava", "лава", Tile::Lava)
    },
    BlockDef::uniform("obsidian", "обсидиан", Tile::Obsidian),
    BlockDef::uniform("cobblestone", "булыжник", Tile::Cobblestone),
    BlockDef::log(
        "birch_log",
        "берёзовое бревно",
        Tile::BirchLogSide,
        Tile::BirchLogTop,
    ),
    BlockDef::leaves("birch_leaves", "берёзовая листва", Tile::BirchLeaves),
    BlockDef::log(
        "spruce_log",
        "еловое бревно",
        Tile::SpruceLogSide,
        Tile::SpruceLogTop,
    ),
    BlockDef::leaves("spruce_leaves", "хвоя", Tile::SpruceLeaves),
    BlockDef {
        unbreakable: true,
        ..BlockDef::uniform("bedrock", "коренная порода", Tile::Bedrock)
    },
    BlockDef {
        top: Some(Tile::Snow),
        side: Some(Tile::SnowyGrassSide),
        bottom: Some(Tile::Dirt),
        random_ticks: true,
        ..BlockDef::uniform("snowy_grass", "заснеженная трава", Tile::Dirt)
    },
    BlockDef {
        solid: false,
        opaque: false,
        replaceable: true,
        needs_support: true,
        random_ticks: true,
        shape: Shape::Layer,
        ..BlockDef::uniform("snow", "снег", Tile::Snow)
    },
    BlockDef {
        opaque: false,
        slippery: true,
        random_ticks: true,
        layer: RenderLayer::Translucent,
        ..BlockDef::uniform("ice", "лёд", Tile::Ice)
    },
    BlockDef {
        selectable: false,
        opaque: false,
        name: "barrier",
        label: "барьер",
        ..BlockDef::CUBE
    },
    BlockDef {
        opaque: false,
        needs_support: true,
        random_ticks: true,
        layer: RenderLayer::Cutout,
        shape: Shape::Cactus,
        top: Some(Tile::CactusTop),
        side: Some(Tile::CactusSide),
        bottom: Some(Tile::CactusBottom),
        ..BlockDef::uniform("cactus", "кактус", Tile::CactusSide)
    },
    BlockDef::plant("dead_bush", "сухой куст", Tile::DeadBush),
    BlockDef {
        falls: true,
        ..BlockDef::uniform("red_sand", "красный песок", Tile::RedSand)
    },
    BlockDef::uniform("terracotta", "терракота", Tile::Terracotta),
    BlockDef::uniform("white_terracotta", "белая терракота", Tile::TerracottaWhite),
    BlockDef::uniform(
        "orange_terracotta",
        "оранжевая терракота",
        Tile::TerracottaOrange,
    ),
    BlockDef::uniform(
        "yellow_terracotta",
        "жёлтая терракота",
        Tile::TerracottaYellow,
    ),
    BlockDef::uniform(
        "brown_terracotta",
        "коричневая терракота",
        Tile::TerracottaBrown,
    ),
    BlockDef::uniform("red_terracotta", "красная терракота", Tile::TerracottaRed),
    BlockDef::uniform(
        "silver_terracotta",
        "светло-серая терракота",
        Tile::TerracottaSilver,
    ),
    BlockDef {
        random_ticks: true,
        ..BlockDef::plant("reeds", "сахарный тростник", Tile::Reeds)
    },
    BlockDef {
        top: Some(Tile::SandstoneTop),
        bottom: Some(Tile::SandstoneBottom),
        ..BlockDef::uniform("sandstone", "песчаник", Tile::SandstoneSide)
    },
    BlockDef {
        top: Some(Tile::RedSandstoneTop),
        bottom: Some(Tile::RedSandstoneBottom),
        ..BlockDef::uniform("red_sandstone", "красный песчаник", Tile::RedSandstoneSide)
    },
    BlockDef::log(
        "jungle_log",
        "тропическое бревно",
        Tile::JungleLogSide,
        Tile::JungleLogTop,
    ),
    BlockDef::leaves("jungle_leaves", "тропическая листва", Tile::JungleLeaves),
    BlockDef::log(
        "acacia_log",
        "бревно акации",
        Tile::AcaciaLogSide,
        Tile::AcaciaLogTop,
    ),
    BlockDef::leaves("acacia_leaves", "листва акации", Tile::AcaciaLeaves),
    BlockDef::log(
        "dark_oak_log",
        "бревно тёмного дуба",
        Tile::DarkOakLogSide,
        Tile::DarkOakLogTop,
    ),
    BlockDef::leaves(
        "dark_oak_leaves",
        "листва тёмного дуба",
        Tile::DarkOakLeaves,
    ),
    BlockDef {
        solid: false,
        opaque: false,
        random_ticks: true,
        data: DataKind::Sides,
        layer: RenderLayer::Cutout,
        shape: Shape::Vine,
        ..BlockDef::uniform("vine", "лианы", Tile::Vine)
    },
    BlockDef {
        top: Some(Tile::MelonTop),
        bottom: Some(Tile::MelonTop),
        ..BlockDef::uniform("melon", "арбуз", Tile::MelonSide)
    },
    BlockDef {
        top: Some(Tile::MushroomSkinBrown),
        bottom: Some(Tile::MushroomInside),
        ..BlockDef::uniform(
            "brown_mushroom_block",
            "коричневый грибной блок",
            Tile::MushroomSkinBrown,
        )
    },
    BlockDef {
        top: Some(Tile::MushroomSkinRed),
        bottom: Some(Tile::MushroomInside),
        ..BlockDef::uniform(
            "red_mushroom_block",
            "красный грибной блок",
            Tile::MushroomSkinRed,
        )
    },
    BlockDef {
        top: Some(Tile::MushroomInside),
        bottom: Some(Tile::MushroomInside),
        ..BlockDef::uniform("mushroom_stem", "ножка гриба", Tile::MushroomStem)
    },
    BlockDef {
        shape: Shape::Lily,
        ..BlockDef::plant("waterlily", "кувшинка", Tile::Waterlily)
    },
    BlockDef {
        top: Some(Tile::MyceliumTop),
        side: Some(Tile::MyceliumSide),
        bottom: Some(Tile::Dirt),
        random_ticks: true,
        ..BlockDef::uniform("mycelium", "мицелий", Tile::MyceliumSide)
    },
    BlockDef::plant("brown_mushroom", "коричневый гриб", Tile::BrownMushroom),
    BlockDef::plant("red_mushroom", "красный гриб", Tile::RedMushroom),
    BlockDef::uniform("clay", "глина", Tile::Clay),
    BlockDef {
        slippery: true,
        ..BlockDef::uniform("ice_packed", "плотный лёд", Tile::IcePacked)
    },
    BlockDef {
        top: Some(Tile::PodzolTop),
        side: Some(Tile::PodzolSide),
        bottom: Some(Tile::Dirt),
        ..BlockDef::uniform("podzol", "подзол", Tile::PodzolSide)
    },
    BlockDef::uniform("coarse_dirt", "крупнозём", Tile::CoarseDirt),
    BlockDef::uniform(
        "mossy_cobblestone",
        "замшелый булыжник",
        Tile::MossyCobblestone,
    ),
    BlockDef::plant("fern", "папоротник", Tile::Fern),
    BlockDef {
        selectable: false,
        ..BlockDef::plant(
            "double_grass_lower",
            "высокая трава (низ)",
            Tile::DoubleGrassBottom,
        )
    },
    BlockDef {
        selectable: false,
        ..BlockDef::plant(
            "double_grass_upper",
            "высокая трава (верх)",
            Tile::DoubleGrassTop,
        )
    },
    BlockDef {
        selectable: false,
        ..BlockDef::plant(
            "double_fern_lower",
            "большой папоротник (низ)",
            Tile::DoubleFernBottom,
        )
    },
    BlockDef {
        selectable: false,
        ..BlockDef::plant(
            "double_fern_upper",
            "большой папоротник (верх)",
            Tile::DoubleFernTop,
        )
    },
    BlockDef {
        selectable: false,
        ..BlockDef::plant("sunflower_lower", "подсолнух (низ)", Tile::SunflowerBottom)
    },
    BlockDef {
        selectable: false,
        shape: Shape::Sunflower,
        ..BlockDef::plant("sunflower_upper", "подсолнух (верх)", Tile::SunflowerTop)
    },
    BlockDef {
        selectable: false,
        ..BlockDef::plant("lilac_lower", "сирень (низ)", Tile::SyringaBottom)
    },
    BlockDef {
        selectable: false,
        ..BlockDef::plant("lilac_upper", "сирень (верх)", Tile::SyringaTop)
    },
    BlockDef {
        selectable: false,
        ..BlockDef::plant(
            "rose_bush_lower",
            "розовый куст (низ)",
            Tile::RoseBushBottom,
        )
    },
    BlockDef {
        selectable: false,
        ..BlockDef::plant("rose_bush_upper", "розовый куст (верх)", Tile::RoseBushTop)
    },
    BlockDef {
        selectable: false,
        ..BlockDef::plant("peony_lower", "пион (низ)", Tile::PeonyBottom)
    },
    BlockDef {
        selectable: false,
        ..BlockDef::plant("peony_upper", "пион (верх)", Tile::PeonyTop)
    },
    BlockDef::plant("allium", "аллиум", Tile::Allium),
    BlockDef::plant("blue_orchid", "синяя орхидея", Tile::BlueOrchid),
    BlockDef::plant("houstonia", "хоустония", Tile::Houstonia),
    BlockDef::plant("oxeye_daisy", "ромашка", Tile::OxeyeDaisy),
    BlockDef::plant("red_tulip", "красный тюльпан", Tile::TulipRed),
    BlockDef::plant("orange_tulip", "оранжевый тюльпан", Tile::TulipOrange),
    BlockDef::plant("white_tulip", "белый тюльпан", Tile::TulipWhite),
    BlockDef::plant("pink_tulip", "розовый тюльпан", Tile::TulipPink),
    BlockDef {
        data: DataKind::Facing,
        top: Some(Tile::PumpkinTop),
        bottom: Some(Tile::PumpkinTop),
        ..BlockDef::uniform("pumpkin", "тыква", Tile::PumpkinSide)
    },
    BlockDef::uniform("granite", "гранит", Tile::StoneGranite),
    BlockDef::uniform("diorite", "диорит", Tile::StoneDiorite),
    BlockDef::uniform("andesite", "андезит", Tile::StoneAndesite),
    BlockDef::uniform("coal_ore", "угольная руда", Tile::CoalOre),
    BlockDef::uniform("iron_ore", "железная руда", Tile::IronOre),
    BlockDef::uniform("gold_ore", "золотая руда", Tile::GoldOre),
    BlockDef::uniform("redstone_ore", "редстоуновая руда", Tile::RedstoneOre),
    BlockDef::uniform("diamond_ore", "алмазная руда", Tile::DiamondOre),
    BlockDef::uniform("lapis_ore", "лазуритовая руда", Tile::LapisOre),
    BlockDef::uniform("emerald_ore", "изумрудная руда", Tile::EmeraldOre),
    BlockDef {
        solid: false,
        opaque: false,
        needs_support: true,
        data: DataKind::Cocoa,
        layer: RenderLayer::Cutout,
        shape: Shape::Cocoa,
        ..BlockDef::uniform("cocoa", "какао-бобы", Tile::CocoaStage2)
    },
    BlockDef::uniform("snow_block", "снежный блок", Tile::Snow),
    BlockDef::uniform("monster_egg", "яйцо чешуйницы", Tile::Stone),
    BlockDef::log(
        "bone_block",
        "костяной блок",
        Tile::BoneBlockSide,
        Tile::BoneBlockTop,
    ),
    BlockDef::uniform("oak_planks", "дубовые доски", Tile::PlanksOak),
    BlockDef::uniform("spruce_planks", "еловые доски", Tile::PlanksSpruce),
    BlockDef::uniform("birch_planks", "берёзовые доски", Tile::PlanksBirch),
    BlockDef::uniform("jungle_planks", "тропические доски", Tile::PlanksJungle),
    BlockDef::uniform("acacia_planks", "доски акации", Tile::PlanksAcacia),
    BlockDef::uniform("dark_oak_planks", "доски тёмного дуба", Tile::PlanksDarkOak),
    BlockDef::uniform("stone_bricks", "каменные кирпичи", Tile::StoneBrick),
    BlockDef::uniform(
        "mossy_stone_bricks",
        "замшелые каменные кирпичи",
        Tile::StoneBrickMossy,
    ),
    BlockDef::uniform(
        "cracked_stone_bricks",
        "треснутые каменные кирпичи",
        Tile::StoneBrickCracked,
    ),
    BlockDef::uniform(
        "chiseled_stone_bricks",
        "резные каменные кирпичи",
        Tile::StoneBrickCarved,
    ),
    BlockDef {
        top: Some(Tile::SandstoneTop),
        bottom: Some(Tile::SandstoneBottom),
        ..BlockDef::uniform(
            "chiseled_sandstone",
            "резной песчаник",
            Tile::SandstoneCarved,
        )
    },
    BlockDef {
        top: Some(Tile::SandstoneTop),
        bottom: Some(Tile::SandstoneBottom),
        ..BlockDef::uniform(
            "smooth_sandstone",
            "гладкий песчаник",
            Tile::SandstoneSmooth,
        )
    },
    BlockDef {
        top: Some(Tile::RedSandstoneTop),
        bottom: Some(Tile::RedSandstoneBottom),
        ..BlockDef::uniform(
            "chiseled_red_sandstone",
            "резной красный песчаник",
            Tile::RedSandstoneCarved,
        )
    },
    BlockDef {
        top: Some(Tile::RedSandstoneTop),
        bottom: Some(Tile::RedSandstoneBottom),
        ..BlockDef::uniform(
            "smooth_red_sandstone",
            "гладкий красный песчаник",
            Tile::RedSandstoneSmooth,
        )
    },
    BlockDef::uniform("bricks", "кирпичи", Tile::Brick),
    BlockDef {
        opaque: false,
        joins_same: true,
        layer: RenderLayer::Cutout,
        ..BlockDef::uniform("glass", "стекло", Tile::Glass)
    },
    BlockDef {
        data: DataKind::Color,
        ..BlockDef::uniform("wool", "шерсть", Tile::WoolWhite)
    },
    BlockDef {
        top: Some(Tile::PlanksOak),
        bottom: Some(Tile::PlanksOak),
        ..BlockDef::uniform("bookshelf", "книжная полка", Tile::Bookshelf)
    },
    BlockDef {
        top: Some(Tile::CraftingTableTop),
        bottom: Some(Tile::PlanksOak),
        ..BlockDef::uniform("crafting_table", "верстак", Tile::CraftingTableSide)
    },
    BlockDef::slab(
        "oak_slab",
        "дубовая плита",
        Tile::PlanksOak,
        Tile::PlanksOak,
        Tile::PlanksOak,
    ),
    BlockDef::slab(
        "spruce_slab",
        "еловая плита",
        Tile::PlanksSpruce,
        Tile::PlanksSpruce,
        Tile::PlanksSpruce,
    ),
    BlockDef::slab(
        "birch_slab",
        "берёзовая плита",
        Tile::PlanksBirch,
        Tile::PlanksBirch,
        Tile::PlanksBirch,
    ),
    BlockDef::slab(
        "jungle_slab",
        "тропическая плита",
        Tile::PlanksJungle,
        Tile::PlanksJungle,
        Tile::PlanksJungle,
    ),
    BlockDef::slab(
        "acacia_slab",
        "плита акации",
        Tile::PlanksAcacia,
        Tile::PlanksAcacia,
        Tile::PlanksAcacia,
    ),
    BlockDef::slab(
        "dark_oak_slab",
        "плита тёмного дуба",
        Tile::PlanksDarkOak,
        Tile::PlanksDarkOak,
        Tile::PlanksDarkOak,
    ),
    BlockDef::slab(
        "stone_slab",
        "каменная плита",
        Tile::StoneSlabTop,
        Tile::StoneSlabSide,
        Tile::StoneSlabTop,
    ),
    BlockDef::slab(
        "sandstone_slab",
        "плита из песчаника",
        Tile::SandstoneTop,
        Tile::SandstoneSide,
        Tile::SandstoneBottom,
    ),
    BlockDef::slab(
        "cobblestone_slab",
        "плита из булыжника",
        Tile::Cobblestone,
        Tile::Cobblestone,
        Tile::Cobblestone,
    ),
    BlockDef::slab(
        "brick_slab",
        "кирпичная плита",
        Tile::Brick,
        Tile::Brick,
        Tile::Brick,
    ),
    BlockDef::slab(
        "stone_brick_slab",
        "плита из каменных кирпичей",
        Tile::StoneBrick,
        Tile::StoneBrick,
        Tile::StoneBrick,
    ),
    BlockDef::slab(
        "red_sandstone_slab",
        "плита из красного песчаника",
        Tile::RedSandstoneTop,
        Tile::RedSandstoneSide,
        Tile::RedSandstoneBottom,
    ),
    BlockDef::stairs(
        "oak_stairs",
        "дубовые ступени",
        Tile::PlanksOak,
        Tile::PlanksOak,
        Tile::PlanksOak,
    ),
    BlockDef::stairs(
        "spruce_stairs",
        "еловые ступени",
        Tile::PlanksSpruce,
        Tile::PlanksSpruce,
        Tile::PlanksSpruce,
    ),
    BlockDef::stairs(
        "birch_stairs",
        "берёзовые ступени",
        Tile::PlanksBirch,
        Tile::PlanksBirch,
        Tile::PlanksBirch,
    ),
    BlockDef::stairs(
        "jungle_stairs",
        "тропические ступени",
        Tile::PlanksJungle,
        Tile::PlanksJungle,
        Tile::PlanksJungle,
    ),
    BlockDef::stairs(
        "acacia_stairs",
        "ступени акации",
        Tile::PlanksAcacia,
        Tile::PlanksAcacia,
        Tile::PlanksAcacia,
    ),
    BlockDef::stairs(
        "dark_oak_stairs",
        "ступени тёмного дуба",
        Tile::PlanksDarkOak,
        Tile::PlanksDarkOak,
        Tile::PlanksDarkOak,
    ),
    BlockDef::stairs(
        "cobblestone_stairs",
        "ступени из булыжника",
        Tile::Cobblestone,
        Tile::Cobblestone,
        Tile::Cobblestone,
    ),
    BlockDef::stairs(
        "brick_stairs",
        "кирпичные ступени",
        Tile::Brick,
        Tile::Brick,
        Tile::Brick,
    ),
    BlockDef::stairs(
        "stone_brick_stairs",
        "ступени из каменных кирпичей",
        Tile::StoneBrick,
        Tile::StoneBrick,
        Tile::StoneBrick,
    ),
    BlockDef::stairs(
        "sandstone_stairs",
        "ступени из песчаника",
        Tile::SandstoneTop,
        Tile::SandstoneSide,
        Tile::SandstoneBottom,
    ),
    BlockDef::stairs(
        "red_sandstone_stairs",
        "ступени из красного песчаника",
        Tile::RedSandstoneTop,
        Tile::RedSandstoneSide,
        Tile::RedSandstoneBottom,
    ),
    BlockDef {
        opaque: false,
        needs_support: true,
        data: DataKind::Color,
        shape: Shape::Boxes,
        boxes: BoxKind::Carpet,
        ..BlockDef::uniform("carpet", "ковёр", Tile::WoolWhite)
    },
];

impl Block {
    pub const ALL: [Block; 138] = [
        Block::Air,
        Block::Grass,
        Block::Dirt,
        Block::Stone,
        Block::Sand,
        Block::Log,
        Block::Leaves,
        Block::Water,
        Block::Glowstone,
        Block::TallGrass,
        Block::Poppy,
        Block::Dandelion,
        Block::Torch,
        Block::Gravel,
        Block::Lava,
        Block::Obsidian,
        Block::Cobblestone,
        Block::BirchLog,
        Block::BirchLeaves,
        Block::SpruceLog,
        Block::SpruceLeaves,
        Block::Bedrock,
        Block::SnowyGrass,
        Block::Snow,
        Block::Ice,
        Block::Barrier,
        Block::Cactus,
        Block::DeadBush,
        Block::RedSand,
        Block::Terracotta,
        Block::WhiteTerracotta,
        Block::OrangeTerracotta,
        Block::YellowTerracotta,
        Block::BrownTerracotta,
        Block::RedTerracotta,
        Block::SilverTerracotta,
        Block::Reeds,
        Block::Sandstone,
        Block::RedSandstone,
        Block::JungleLog,
        Block::JungleLeaves,
        Block::AcaciaLog,
        Block::AcaciaLeaves,
        Block::DarkOakLog,
        Block::DarkOakLeaves,
        Block::Vine,
        Block::Melon,
        Block::BrownMushroomBlock,
        Block::RedMushroomBlock,
        Block::MushroomStem,
        Block::Waterlily,
        Block::Mycelium,
        Block::BrownMushroom,
        Block::RedMushroom,
        Block::Clay,
        Block::IcePacked,
        Block::Podzol,
        Block::CoarseDirt,
        Block::MossyCobblestone,
        Block::Fern,
        Block::DoubleGrassLower,
        Block::DoubleGrassUpper,
        Block::DoubleFernLower,
        Block::DoubleFernUpper,
        Block::SunflowerLower,
        Block::SunflowerUpper,
        Block::LilacLower,
        Block::LilacUpper,
        Block::RoseBushLower,
        Block::RoseBushUpper,
        Block::PeonyLower,
        Block::PeonyUpper,
        Block::Allium,
        Block::BlueOrchid,
        Block::Houstonia,
        Block::OxeyeDaisy,
        Block::RedTulip,
        Block::OrangeTulip,
        Block::WhiteTulip,
        Block::PinkTulip,
        Block::Pumpkin,
        Block::Granite,
        Block::Diorite,
        Block::Andesite,
        Block::CoalOre,
        Block::IronOre,
        Block::GoldOre,
        Block::RedstoneOre,
        Block::DiamondOre,
        Block::LapisOre,
        Block::EmeraldOre,
        Block::Cocoa,
        Block::SnowBlock,
        Block::MonsterEgg,
        Block::BoneBlock,
        Block::OakPlanks,
        Block::SprucePlanks,
        Block::BirchPlanks,
        Block::JunglePlanks,
        Block::AcaciaPlanks,
        Block::DarkOakPlanks,
        Block::StoneBricks,
        Block::MossyStoneBricks,
        Block::CrackedStoneBricks,
        Block::ChiseledStoneBricks,
        Block::ChiseledSandstone,
        Block::SmoothSandstone,
        Block::ChiseledRedSandstone,
        Block::SmoothRedSandstone,
        Block::Bricks,
        Block::Glass,
        Block::Wool,
        Block::Bookshelf,
        Block::CraftingTable,
        Block::OakSlab,
        Block::SpruceSlab,
        Block::BirchSlab,
        Block::JungleSlab,
        Block::AcaciaSlab,
        Block::DarkOakSlab,
        Block::StoneSlab,
        Block::SandstoneSlab,
        Block::CobblestoneSlab,
        Block::BrickSlab,
        Block::StoneBrickSlab,
        Block::RedSandstoneSlab,
        Block::OakStairs,
        Block::SpruceStairs,
        Block::BirchStairs,
        Block::JungleStairs,
        Block::AcaciaStairs,
        Block::DarkOakStairs,
        Block::CobblestoneStairs,
        Block::BrickStairs,
        Block::StoneBrickStairs,
        Block::SandstoneStairs,
        Block::RedSandstoneStairs,
        Block::Carpet,
    ];

    pub fn def(self) -> &'static BlockDef {
        &BLOCKS[self as usize]
    }

    pub fn is_unbreakable(self) -> bool {
        self.def().unbreakable
    }

    pub fn is_stone(self) -> bool {
        matches!(
            self,
            Block::Stone | Block::Granite | Block::Diorite | Block::Andesite
        )
    }

    pub fn is_solid(self) -> bool {
        self.def().solid
    }

    pub fn collision_height(self) -> f32 {
        match self.shape() {
            Shape::Layer => SNOW_HEIGHT,
            Shape::Cactus => CACTUS_HEIGHT,
            Shape::Boxes => 1.0,
            _ if self.is_solid() => 1.0,
            _ => 0.0,
        }
    }

    pub fn is_slippery(self) -> bool {
        self.def().slippery
    }

    pub fn is_grass(self) -> bool {
        matches!(self, Block::Grass | Block::SnowyGrass)
    }

    pub fn is_targetable(self) -> bool {
        self.def().targetable
    }

    pub fn shape(self) -> Shape {
        self.def().shape
    }

    pub fn bounds(self, data: u8) -> (Vec3, Vec3) {
        match self.shape() {
            Shape::Torch => {
                let unit = 1.0 / PIXEL_GRID as f32;
                let (lo, hi) = (TORCH_PIXELS.0 as f32 * unit, TORCH_PIXELS.1 as f32 * unit);
                let height = (PIXEL_GRID - TORCH_TOP_ROW) as f32 * unit;
                let Some(support) = self.support_dir(data).filter(|dir| dir.y == 0) else {
                    return (Vec3::new(lo, 0.0, lo), Vec3::new(hi, height, hi));
                };
                let shift = TORCH_WALL_SHIFT as f32 * unit;
                let rise = TORCH_WALL_RISE as f32 * unit;
                let min = Vec3::new(0.0, rise, lo);
                let max = Vec3::new(hi - lo + shift, rise + height, hi);
                let mut turns = 0;
                let mut dir = IVec3::NEG_X;
                while dir != support {
                    dir = IVec3::new(-dir.z, 0, dir.x);
                    turns += 1;
                }
                let rotate = |p: Vec3| {
                    let mut p = p - Vec3::new(0.5, 0.0, 0.5);
                    for _ in 0..turns {
                        p = Vec3::new(-p.z, p.y, p.x);
                    }
                    p + Vec3::new(0.5, 0.0, 0.5)
                };
                let (a, b) = (rotate(min), rotate(max));
                (a.min(b), a.max(b))
            }
            Shape::Cocoa => {
                let pod =
                    COCOA_PODS[usize::from(data >> COCOA_STAGE_SHIFT).min(COCOA_PODS.len() - 1)];
                let unit = 1.0 / PIXEL_GRID as f32;
                let half = pod.width as f32 / 2.0;
                let min = Vec3::new(
                    8.0 - half,
                    pod.bottom as f32,
                    (COCOA_BACK - pod.width) as f32,
                );
                let max = Vec3::new(8.0 + half, pod.top as f32, COCOA_BACK as f32);
                let turns = ((data & COCOA_DIRECTION_MASK) + 2) % 4;
                let rotate = |p: Vec3| {
                    let mut p = p * unit - Vec3::new(0.5, 0.0, 0.5);
                    for _ in 0..turns {
                        p = Vec3::new(-p.z, p.y, p.x);
                    }
                    p + Vec3::new(0.5, 0.0, 0.5)
                };
                let (a, b) = (rotate(min), rotate(max));
                (a.min(b), a.max(b))
            }
            Shape::Boxes => self.boxes(data, |_| (Block::Air, 0)).bounds(),
            Shape::Layer => (Vec3::ZERO, Vec3::new(1.0, SNOW_HEIGHT, 1.0)),
            Shape::Lily => (Vec3::ZERO, Vec3::new(1.0, LILY_HEIGHT, 1.0)),
            Shape::Cactus => (
                Vec3::new(CACTUS_INSET, 0.0, CACTUS_INSET),
                Vec3::new(1.0 - CACTUS_INSET, CACTUS_HEIGHT, 1.0 - CACTUS_INSET),
            ),
            Shape::Vine => {
                let (mut min, mut max) = (Vec3::ONE, Vec3::ZERO);
                for (bit, dir) in VINE_SIDES {
                    if data & bit == 0 {
                        continue;
                    }
                    let axis = dir.abs().max_position();
                    let (mut lo, mut hi) = (Vec3::ZERO, Vec3::ONE);
                    if dir[axis] > 0 {
                        lo[axis] = 1.0 - VINE_INSET;
                    } else {
                        hi[axis] = VINE_INSET;
                    }
                    min = min.min(lo);
                    max = max.max(hi);
                }
                (min.min(max), max)
            }
            _ => (Vec3::ZERO, Vec3::ONE),
        }
    }

    pub fn box_kind(self) -> BoxKind {
        self.def().boxes
    }

    pub fn is_stairs(self) -> bool {
        self.box_kind() == BoxKind::Stairs
    }

    pub fn is_slab(self) -> bool {
        self.box_kind() == BoxKind::Slab
    }

    /// Коробки блока в 1/16; `around` отдаёт блок и `data` соседа по смещению от клетки,
    /// нужен только ступеням для формы углов.
    pub fn boxes(self, data: u8, around: impl Fn(IVec3) -> (Block, u8)) -> BoxList {
        let unit = |min, max| BoxList::of(&[VoxelBox::new(min, max)]);
        match self.box_kind() {
            BoxKind::None => BoxList::of(&[VoxelBox::FULL]),
            BoxKind::Carpet => unit([0, 0, 0], [16, CARPET_UNITS, 16]),
            BoxKind::Slab => match data {
                SLAB_DOUBLE => BoxList::of(&[VoxelBox::FULL]),
                SLAB_TOP => unit([0, 8, 0], [16, 16, 16]),
                _ => unit([0, 0, 0], [16, 8, 16]),
            },
            BoxKind::Stairs => {
                let turns = ((data & STAIRS_FACING_MASK) + 3) % 4;
                let top = data & STAIRS_TOP != 0;
                let base = VoxelBox::new([0, 0, 0], [16, 8, 16]);
                let (back, front_side) = match self.stair_shape(data, around) {
                    StairShape::Straight => (VoxelBox::new([8, 8, 0], [16, 16, 16]), None),
                    StairShape::OuterLeft => (VoxelBox::new([8, 8, 0], [16, 16, 8]), None),
                    StairShape::OuterRight => (VoxelBox::new([8, 8, 8], [16, 16, 16]), None),
                    StairShape::InnerLeft => (
                        VoxelBox::new([8, 8, 0], [16, 16, 16]),
                        Some(VoxelBox::new([0, 8, 0], [8, 16, 8])),
                    ),
                    StairShape::InnerRight => (
                        VoxelBox::new([8, 8, 0], [16, 16, 16]),
                        Some(VoxelBox::new([0, 8, 8], [8, 16, 16])),
                    ),
                };
                let mut list = BoxList::of(&[base, back]);
                if let Some(extra) = front_side {
                    list.push(extra);
                }
                list.map(|b| {
                    let b = b.turned(turns);
                    if top { b.flipped_y() } else { b }
                })
            }
        }
    }

    /// Форма углов ступени по соседям, как `BlockStairs.getActualState`.
    pub fn stair_shape(self, data: u8, around: impl Fn(IVec3) -> (Block, u8)) -> StairShape {
        let facing = self.facing_dir(data & STAIRS_FACING_MASK);
        let left = IVec3::new(facing.z, 0, -facing.x);
        let half = data & STAIRS_TOP;
        let stairs = |offset: IVec3| {
            let (block, other) = around(offset);
            (block.is_stairs() && other & STAIRS_TOP == half)
                .then(|| block.facing_dir(other & STAIRS_FACING_MASK))
        };
        let differs = |offset: IVec3| {
            let (block, other) = around(offset);
            !block.is_stairs()
                || other & STAIRS_TOP != half
                || block.facing_dir(other & STAIRS_FACING_MASK) != facing
        };
        if let Some(side) = stairs(facing)
            && side.dot(facing) == 0
            && differs(-side)
        {
            return if side == left {
                StairShape::OuterLeft
            } else {
                StairShape::OuterRight
            };
        }
        if let Some(side) = stairs(-facing)
            && side.dot(facing) == 0
            && differs(side)
        {
            return if side == left {
                StairShape::InnerLeft
            } else {
                StairShape::InnerRight
            };
        }
        StairShape::Straight
    }

    /// Соседние блоки одного вида не рисуют общую грань.
    pub fn joins_same(self) -> bool {
        self.def().joins_same
    }

    pub fn blocks_light(self) -> bool {
        let def = self.def();
        def.opaque || def.light_blocking
    }

    /// Плиты и ступени: свет клетки берётся у соседей (`useNeighborBrightness`).
    pub fn lights_from_neighbors(self) -> bool {
        let def = self.def();
        def.light_blocking && !def.opaque
    }

    pub fn data_kind(self) -> DataKind {
        self.def().data
    }

    pub fn support_dir(self, data: u8) -> Option<IVec3> {
        if !self.needs_support() {
            return None;
        }
        if self.data_kind() == DataKind::Cocoa {
            return Some(self.facing_dir(data & COCOA_DIRECTION_MASK));
        }
        if self.data_kind() != DataKind::Attachment {
            return Some(IVec3::NEG_Y);
        }
        Some(match data {
            ATTACH_POS_X => IVec3::X,
            ATTACH_NEG_X => IVec3::NEG_X,
            ATTACH_POS_Z => IVec3::Z,
            ATTACH_NEG_Z => IVec3::NEG_Z,
            _ => IVec3::NEG_Y,
        })
    }

    pub fn placement_state(self, placement: &Placement) -> Option<u8> {
        let upper = placement.normal != IVec3::Y
            && (placement.normal == IVec3::NEG_Y || placement.hit_y > 0.5);
        match self.data_kind() {
            DataKind::Color => Some(placement.variant % COLOR_COUNT),
            DataKind::Slab => Some(if upper { SLAB_TOP } else { SLAB_BOTTOM }),
            DataKind::Stairs => {
                Some(facing_code(placement.look) | if upper { STAIRS_TOP } else { 0 })
            }
            _ => self.placement_data(placement.normal),
        }
    }

    pub fn placement_data(self, normal: IVec3) -> Option<u8> {
        match self.data_kind() {
            DataKind::None
            | DataKind::Fluid
            | DataKind::Color
            | DataKind::Slab
            | DataKind::Stairs => Some(0),
            DataKind::Sides => match normal {
                IVec3::X => Some(VINE_NEG_X),
                IVec3::NEG_X => Some(VINE_POS_X),
                IVec3::Z => Some(VINE_NEG_Z),
                IVec3::NEG_Z => Some(VINE_POS_Z),
                IVec3::NEG_Y => Some(VINE_TOP),
                _ => None,
            },
            DataKind::Facing => Some(match normal {
                IVec3::X => FACING_POS_X,
                IVec3::NEG_X => FACING_NEG_X,
                IVec3::Z => FACING_POS_Z,
                _ => FACING_NEG_Z,
            }),
            DataKind::Cocoa => match normal {
                IVec3::X => Some(FACING_NEG_X),
                IVec3::NEG_X => Some(FACING_POS_X),
                IVec3::Z => Some(FACING_NEG_Z),
                IVec3::NEG_Z => Some(FACING_POS_Z),
                _ => None,
            },
            DataKind::Axis => Some(match normal.abs().max_position() {
                0 => AXIS_X,
                1 => AXIS_Y,
                _ => AXIS_Z,
            }),
            DataKind::Attachment => match normal {
                IVec3::Y => Some(ATTACH_FLOOR),
                IVec3::NEG_Y => None,
                IVec3::X => Some(ATTACH_NEG_X),
                IVec3::NEG_X => Some(ATTACH_POS_X),
                IVec3::Z => Some(ATTACH_NEG_Z),
                IVec3::NEG_Z => Some(ATTACH_POS_Z),
                _ => None,
            },
        }
    }

    pub fn states(self) -> impl Iterator<Item = u8> {
        let states: &[u8] = match self.data_kind() {
            DataKind::None => &[0],
            DataKind::Axis => &[AXIS_Y, AXIS_X, AXIS_Z],
            DataKind::Attachment => &[
                ATTACH_FLOOR,
                ATTACH_POS_X,
                ATTACH_NEG_X,
                ATTACH_POS_Z,
                ATTACH_NEG_Z,
            ],
            DataKind::Facing => &[FACING_NEG_Z, FACING_POS_X, FACING_POS_Z, FACING_NEG_X],
            DataKind::Cocoa => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            DataKind::Color => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
            DataKind::Slab => &[SLAB_BOTTOM, SLAB_TOP, SLAB_DOUBLE],
            DataKind::Stairs => &[0, 1, 2, 3, 4, 5, 6, 7],
            DataKind::Sides => &[VINE_POS_X, VINE_NEG_X, VINE_POS_Z, VINE_NEG_Z, VINE_TOP],
            DataKind::Fluid => &[
                FLUID_SOURCE,
                1,
                2,
                3,
                4,
                5,
                6,
                FLUID_MAX_LEVEL,
                FLUID_FALLING,
            ],
        };
        states.iter().copied()
    }

    pub fn facing_dir(self, data: u8) -> IVec3 {
        match data {
            FACING_POS_X => IVec3::X,
            FACING_POS_Z => IVec3::Z,
            FACING_NEG_X => IVec3::NEG_X,
            _ => IVec3::NEG_Z,
        }
    }

    pub fn axis(self, data: u8) -> usize {
        match data {
            AXIS_X => 0,
            AXIS_Z => 2,
            _ => 1,
        }
    }

    pub fn is_replaceable(self) -> bool {
        self.def().replaceable
    }

    pub fn is_sand(self) -> bool {
        matches!(self, Block::Sand | Block::RedSand)
    }

    pub fn is_terracotta(self) -> bool {
        matches!(
            self,
            Block::Terracotta
                | Block::WhiteTerracotta
                | Block::OrangeTerracotta
                | Block::YellowTerracotta
                | Block::BrownTerracotta
                | Block::RedTerracotta
                | Block::SilverTerracotta
        )
    }

    pub fn is_leaves(self) -> bool {
        matches!(
            self,
            Block::Leaves
                | Block::BirchLeaves
                | Block::SpruceLeaves
                | Block::JungleLeaves
                | Block::AcaciaLeaves
                | Block::DarkOakLeaves
        )
    }

    pub fn holds_vine(self) -> bool {
        self.is_solid() && (self.is_opaque() || self.is_leaves())
    }

    pub fn double_upper(self) -> Option<Block> {
        Some(match self {
            Block::DoubleGrassLower => Block::DoubleGrassUpper,
            Block::DoubleFernLower => Block::DoubleFernUpper,
            Block::SunflowerLower => Block::SunflowerUpper,
            Block::LilacLower => Block::LilacUpper,
            Block::RoseBushLower => Block::RoseBushUpper,
            Block::PeonyLower => Block::PeonyUpper,
            _ => return None,
        })
    }

    pub fn double_lower(self) -> Option<Block> {
        Block::DOUBLE_PLANTS
            .into_iter()
            .find(|lower| lower.double_upper() == Some(self))
    }

    pub const DOUBLE_PLANTS: [Block; 6] = [
        Block::DoubleGrassLower,
        Block::DoubleFernLower,
        Block::SunflowerLower,
        Block::LilacLower,
        Block::RoseBushLower,
        Block::PeonyLower,
    ];

    pub fn is_mushroom(self) -> bool {
        matches!(self, Block::BrownMushroom | Block::RedMushroom)
    }

    pub fn is_stackable_plant(self) -> bool {
        matches!(self, Block::Cactus | Block::Reeds)
    }

    pub fn checks_sides(self) -> bool {
        self == Block::Cactus
    }

    pub fn stands_on(self, support: Block) -> bool {
        match self {
            Block::Cactus => support.is_sand() || support == Block::Cactus,
            Block::DeadBush => {
                support.is_sand()
                    || support.is_terracotta()
                    || matches!(support, Block::Dirt | Block::Podzol | Block::CoarseDirt)
            }
            Block::Reeds => {
                support.is_sand()
                    || support == Block::Reeds
                    || matches!(support, Block::Dirt | Block::Grass)
            }
            Block::Waterlily => support == Block::Water,
            Block::Cocoa => support == Block::JungleLog,
            Block::Carpet => support != Block::Air,
            Block::Snow => {
                support.is_leaves()
                    || (support.is_solid() && !matches!(support, Block::Ice | Block::IcePacked))
            }
            _ if self.double_lower().is_some() => self.double_lower() == Some(support),
            _ => support.is_solid(),
        }
    }

    pub fn needs_support(self) -> bool {
        self.def().needs_support
    }

    pub fn is_opaque(self) -> bool {
        self.def().opaque
    }

    pub fn lights_like(self, other: Block) -> bool {
        self.blocks_light() == other.blocks_light() && self.emission() == other.emission()
    }

    pub fn emission(self) -> u8 {
        self.def().emission
    }

    pub fn falls(self) -> bool {
        self.def().falls
    }

    pub fn random_ticks(self) -> bool {
        self.def().random_ticks
    }

    pub fn fluid(self) -> Option<FluidKind> {
        self.def().fluid
    }

    pub fn is_log(self) -> bool {
        self.data_kind() == DataKind::Axis
    }

    pub fn is_water(self) -> bool {
        self == Block::Water
    }

    pub fn render_layer(self) -> RenderLayer {
        self.def().layer
    }

    pub fn from_name(name: &str) -> Option<Block> {
        Block::ALL
            .into_iter()
            .find(|block| block.def().selectable && block.def().name == name)
    }

    pub fn icon_pixels(self, textures: &TextureSet, data: u8) -> Option<Vec<u8>> {
        self.tile_for(Facing::Side, data)
            .map(|tile| textures.icon_pixels(tile))
    }

    pub fn tile_for(self, facing: Facing, data: u8) -> Option<Tile> {
        if self.data_kind() == DataKind::Color {
            return Some(WOOL_TILES[usize::from(data % COLOR_COUNT)]);
        }
        self.tile(facing)
    }

    pub fn face_tile(self, facing: Facing, normal: IVec3, data: u8) -> Option<Tile> {
        if self == Block::CraftingTable
            && facing == Facing::Side
            && (normal == IVec3::NEG_Z || normal == IVec3::NEG_X)
        {
            return Some(Tile::CraftingTableFront);
        }
        self.tile_for(facing, data)
    }

    pub fn stacks() -> &'static [Stack] {
        static STACKS: LazyLock<Vec<Stack>> = LazyLock::new(|| {
            let mut out = Vec::new();
            for block in Block::ALL {
                let def = block.def();
                if def.data != DataKind::Color {
                    out.push(Stack {
                        name: def.name.to_owned(),
                        label: def.label.to_owned(),
                        block,
                        data: 0,
                        placeable: def.selectable,
                    });
                    continue;
                }
                let labels = if block == Block::Carpet {
                    &CARPET_LABELS
                } else {
                    &WOOL_LABELS
                };
                for color in 0..COLOR_COUNT {
                    out.push(Stack {
                        name: format!("{}_{}", COLOR_NAMES[usize::from(color)], def.name),
                        label: labels[usize::from(color)].to_owned(),
                        block,
                        data: color,
                        placeable: def.selectable,
                    });
                }
            }
            out
        });
        &STACKS
    }

    pub fn stack_from_name(name: &str) -> Option<&'static Stack> {
        Block::stacks()
            .iter()
            .find(|stack| stack.placeable && stack.name == name)
    }

    pub fn tile(self, facing: Facing) -> Option<Tile> {
        let def = self.def();
        match facing {
            Facing::Top => def.top,
            Facing::Bottom => def.bottom,
            Facing::Side => def.side,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StairShape {
    Straight,
    InnerLeft,
    InnerRight,
    OuterLeft,
    OuterRight,
}

/// Вариант блока в инвентаре: блок и его `data` (цвет шерсти и ковра).
#[derive(Clone, Debug)]
pub struct Stack {
    pub name: String,
    pub label: String,
    pub block: Block,
    pub data: u8,
    pub placeable: bool,
}

/// Контекст установки: клик по грани `normal`, `hit_y` — высота точки клика в клетке,
/// `look` — горизонтальное направление взгляда, `variant` — `data` предмета.
#[derive(Clone, Copy, Debug)]
pub struct Placement {
    pub normal: IVec3,
    pub hit_y: f32,
    pub look: IVec3,
    pub variant: u8,
}

pub fn facing_code(dir: IVec3) -> u8 {
    match dir {
        IVec3::X => FACING_POS_X,
        IVec3::Z => FACING_POS_Z,
        IVec3::NEG_X => FACING_NEG_X,
        _ => FACING_NEG_Z,
    }
}

impl TryFrom<u8> for Block {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, u8> {
        Block::ALL.get(usize::from(value)).copied().ok_or(value)
    }
}

#[cfg(test)]
mod tests;
