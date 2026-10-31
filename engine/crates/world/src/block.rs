use glam::{IVec3, Vec3};

use crate::tile::{TILE_SIZE, TORCH_PIXELS, TORCH_TOP_ROW, Tile, tile_pixels};

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
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DataKind {
    None,
    Axis,
    Attachment,
    Fluid,
}

pub const AXIS_Y: u8 = 0;
pub const AXIS_X: u8 = 1;
pub const AXIS_Z: u8 = 2;

pub const ATTACH_FLOOR: u8 = 0;
pub const ATTACH_POS_X: u8 = 1;
pub const ATTACH_NEG_X: u8 = 2;
pub const ATTACH_POS_Z: u8 = 3;
pub const ATTACH_NEG_Z: u8 = 4;

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
    pub data: DataKind,
    pub fluid: Option<FluidKind>,
    pub emission: u8,
    pub layer: RenderLayer,
    pub shape: Shape,
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
        data: DataKind::None,
        fluid: None,
        emission: 0,
        layer: RenderLayer::Opaque,
        shape: Shape::Cube,
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
pub const BLOCKS: [BlockDef; 22] = [
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
];

impl Block {
    pub const ALL: [Block; 22] = [
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
    ];

    pub fn def(self) -> &'static BlockDef {
        &BLOCKS[self as usize]
    }

    pub fn is_unbreakable(self) -> bool {
        self.def().unbreakable
    }

    pub fn is_solid(self) -> bool {
        self.def().solid
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
                let unit = 1.0 / TILE_SIZE as f32;
                let (lo, hi) = (TORCH_PIXELS.0 as f32 * unit, TORCH_PIXELS.1 as f32 * unit);
                let height = (TILE_SIZE - TORCH_TOP_ROW) as f32 * unit;
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
            _ => (Vec3::ZERO, Vec3::ONE),
        }
    }

    pub fn data_kind(self) -> DataKind {
        self.def().data
    }

    pub fn support_dir(self, data: u8) -> Option<IVec3> {
        if !self.needs_support() {
            return None;
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

    pub fn placement_data(self, normal: IVec3) -> Option<u8> {
        match self.data_kind() {
            DataKind::None | DataKind::Fluid => Some(0),
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

    pub fn needs_support(self) -> bool {
        self.def().needs_support
    }

    pub fn is_opaque(self) -> bool {
        self.def().opaque
    }

    pub fn lights_like(self, other: Block) -> bool {
        self.is_opaque() == other.is_opaque() && self.emission() == other.emission()
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

    pub fn icon_pixels(self) -> Option<Vec<u8>> {
        self.tile(Facing::Side).map(tile_pixels)
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

impl TryFrom<u8> for Block {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, u8> {
        Block::ALL.get(usize::from(value)).copied().ok_or(value)
    }
}

#[cfg(test)]
mod tests;
