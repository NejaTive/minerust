use glam::Vec3;

use crate::tile::{TILE_SIZE, TORCH_PIXELS, TORCH_TOP_ROW, Tile, tile_pixels};

/// Тип блока. `repr(u8)`, чтобы секция занимала байт на блок.
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
}

/// Высота поверхности воды над низом блока, если над ней не вода: как в Minecraft,
/// поверхность чуть ниже берега, и кромка воды не сливается с землёй. По ней же
/// опущены верхние вершины боковых граней и решается, в воде ли глаза.
pub const WATER_SURFACE: f32 = 0.875;

/// В каком проходе отрисовки рисуются грани блока.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RenderLayer {
    /// Без прозрачности.
    Opaque,
    /// С дырками: пиксели либо целиком видны, либо отброшены (листва).
    Cutout,
    /// Полупрозрачные, смешиваются с тем, что за ними (вода).
    Translucent,
}

/// Геометрия блока в мешере.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shape {
    /// Полный куб (и вода): грани по соседям, AO.
    Cube,
    /// Две вертикальные плоскости по диагоналям клетки (трава, цветы).
    Cross,
    /// Столбик 2×10×2 (в 1/16) по центру клетки: факел.
    Torch,
}

/// Куда смотрит грань блока; от этого зависит её тайл.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Facing {
    Top,
    Bottom,
    Side,
}

/// Все свойства блока в одном месте; строка таблицы [`BLOCKS`].
#[derive(Clone, Copy, Debug)]
pub struct BlockDef {
    /// Имя из интерфейса и JS API (`"tall_grass"`).
    pub name: &'static str,
    /// Подпись для игрока.
    pub label: &'static str,
    /// Игрок может поставить блок: воздух и вода — нет, у воздуха есть ломание.
    pub selectable: bool,
    /// Для коллизий, прицела и проверки установки: всё, кроме воздуха, воды и не-кубов.
    pub solid: bool,
    /// Закрывает ли блок грань соседа и участвует ли в затенении углов. Сквозь дырки
    /// листвы видно и грани за ней, и соседнюю листву; сквозь воду — дно и берег.
    pub opaque: bool,
    /// Можно ли навести прицелом: твёрдые, растения и факел (они не мешают ходить, но
    /// их надо уметь сломать).
    pub targetable: bool,
    /// Можно ли поставить другой блок в клетку с этим: воздух, вода и трава.
    pub replaceable: bool,
    /// Нужна ли твёрдая опора снизу: без неё блок исчезает и ставится только на неё.
    pub needs_support: bool,
    /// Уровень света блоков, который блок излучает (0 — не светится).
    pub emission: u8,
    /// Проход отрисовки. У воздуха граней нет, для него значение не важно.
    pub layer: RenderLayer,
    pub shape: Shape,
    /// Тайлы граней; у воздуха их нет.
    pub top: Option<Tile>,
    pub side: Option<Tile>,
    pub bottom: Option<Tile>,
}

impl BlockDef {
    /// Основа для строк таблицы: целый непрозрачный куб; строка меняет только отличия.
    const CUBE: BlockDef = BlockDef {
        name: "",
        label: "",
        selectable: true,
        solid: true,
        opaque: true,
        targetable: true,
        replaceable: false,
        needs_support: false,
        emission: 0,
        layer: RenderLayer::Opaque,
        shape: Shape::Cube,
        top: None,
        side: None,
        bottom: None,
    };

    /// Куб с одним тайлом на все грани.
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

    /// Растение-крест (факел переопределяет форму): не твёрдое, но выбирается прицелом и требует опоры.
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

/// Таблица свойств; индекс — номер блока (`block as u8`), то есть номера в ней часть
/// формата сохранения: новые блоки только в конец, вместе с вариантом [`Block`] и
/// записью в [`Block::ALL`].
pub const BLOCKS: [BlockDef; 13] = [
    // Air
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
    // Grass
    BlockDef {
        top: Some(Tile::GrassTop),
        side: Some(Tile::GrassSide),
        bottom: Some(Tile::Dirt),
        ..BlockDef::uniform("grass", "трава", Tile::Dirt)
    },
    // Dirt
    BlockDef::uniform("dirt", "земля", Tile::Dirt),
    // Stone
    BlockDef::uniform("stone", "камень", Tile::Stone),
    // Sand
    BlockDef::uniform("sand", "песок", Tile::Sand),
    // Log
    BlockDef {
        top: Some(Tile::LogTop),
        side: Some(Tile::LogSide),
        bottom: Some(Tile::LogTop),
        ..BlockDef::uniform("log", "бревно", Tile::LogSide)
    },
    // Leaves
    BlockDef {
        opaque: false,
        layer: RenderLayer::Cutout,
        ..BlockDef::uniform("leaves", "листва", Tile::Leaves)
    },
    // Water
    BlockDef {
        selectable: false,
        solid: false,
        opaque: false,
        targetable: false,
        replaceable: true,
        layer: RenderLayer::Translucent,
        ..BlockDef::uniform("water", "вода", Tile::Water)
    },
    // Glowstone
    BlockDef {
        emission: 15,
        ..BlockDef::uniform("glowstone", "светокамень", Tile::Glowstone)
    },
    // TallGrass: единственное растение, которое заменяется при установке.
    BlockDef {
        replaceable: true,
        ..BlockDef::plant("tall_grass", "высокая трава", Tile::TallGrass)
    },
    // Poppy
    BlockDef::plant("poppy", "мак", Tile::Poppy),
    // Dandelion
    BlockDef::plant("dandelion", "одуванчик", Tile::Dandelion),
    // Torch: на единицу темнее светокамня.
    BlockDef {
        shape: Shape::Torch,
        emission: 14,
        ..BlockDef::plant("torch", "факел", Tile::Torch)
    },
];

impl Block {
    /// Все блоки в порядке номеров. Нужен для `TryFrom<u8>` и обхода таблицы: `as`
    /// в обратную сторону без `unsafe` не сделать.
    pub const ALL: [Block; 13] = [
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
    ];

    pub fn def(self) -> &'static BlockDef {
        &BLOCKS[self as usize]
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

    /// Бокс блока в долях клетки (min, max) — по нему рисуется рамка выделения. У креста
    /// клетка целиком: бокс по высоте травы вышел бы разным для цветка и травы и
    /// мерцал бы при наведении на соседние.
    pub fn bounds(self) -> (Vec3, Vec3) {
        match self.shape() {
            Shape::Torch => {
                let unit = 1.0 / TILE_SIZE as f32;
                let (lo, hi) = (TORCH_PIXELS.0 as f32 * unit, TORCH_PIXELS.1 as f32 * unit);
                (
                    Vec3::new(lo, 0.0, lo),
                    Vec3::new(hi, (TILE_SIZE - TORCH_TOP_ROW) as f32 * unit, hi),
                )
            }
            _ => (Vec3::ZERO, Vec3::ONE),
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

    pub fn emission(self) -> u8 {
        self.def().emission
    }

    pub fn is_water(self) -> bool {
        self == Block::Water
    }

    pub fn render_layer(self) -> RenderLayer {
        self.def().layer
    }

    /// Блок, который игрок может выбрать, по имени из интерфейса (`"grass"`,
    /// `"tall_grass"`, …). Воздух и вода не выбираются: для воздуха есть ломание.
    pub fn from_name(name: &str) -> Option<Block> {
        Block::ALL
            .into_iter()
            .find(|block| block.def().selectable && block.def().name == name)
    }

    /// Иконка для хотбара: пиксели боковой грани (16×16 RGBA, sRGB); `None` — у блока нет граней.
    pub fn icon_pixels(self) -> Option<Vec<u8>> {
        self.tile(Facing::Side).map(tile_pixels)
    }

    /// Тайл грани; `None` — у блока нет граней (воздух).
    pub fn tile(self, facing: Facing) -> Option<Tile> {
        let def = self.def();
        match facing {
            Facing::Top => def.top,
            Facing::Bottom => def.bottom,
            Facing::Side => def.side,
        }
    }
}

/// Обратное к `block as u8`: так блоки читаются из сохранения. Ошибка — сам незнакомый
/// байт.
impl TryFrom<u8> for Block {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, u8> {
        Block::ALL.get(usize::from(value)).copied().ok_or(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Block; 13] = Block::ALL;

    #[test]
    fn byte_round_trip() {
        for (i, block) in ALL.into_iter().enumerate() {
            assert_eq!(block as usize, i, "save numbers must not move");
            assert_eq!(Block::try_from(block as u8), Ok(block));
        }
        assert_eq!(Block::try_from(13), Err(13));
        assert_eq!(Block::try_from(255), Err(255));
    }

    #[test]
    fn grass_has_distinct_top_side_bottom() {
        assert_eq!(Block::Grass.tile(Facing::Top), Some(Tile::GrassTop));
        assert_eq!(Block::Grass.tile(Facing::Side), Some(Tile::GrassSide));
        assert_eq!(Block::Grass.tile(Facing::Bottom), Some(Tile::Dirt));
        assert_eq!(Block::Air.tile(Facing::Top), None);
    }

    #[test]
    fn selectable_by_name() {
        assert_eq!(Block::from_name("grass"), Some(Block::Grass));
        assert_eq!(Block::from_name("dirt"), Some(Block::Dirt));
        assert_eq!(Block::from_name("stone"), Some(Block::Stone));
        assert_eq!(Block::from_name("sand"), Some(Block::Sand));
        assert_eq!(Block::from_name("log"), Some(Block::Log));
        assert_eq!(Block::from_name("leaves"), Some(Block::Leaves));
        assert_eq!(Block::from_name("glowstone"), Some(Block::Glowstone));
        assert_eq!(Block::from_name("air"), None);
        assert_eq!(Block::from_name("water"), None);
        assert_eq!(Block::from_name("Stone"), None);
    }

    #[test]
    fn icon_is_side_face() {
        let pixels = |tile| Some(tile_pixels(tile));
        assert_eq!(Block::Grass.icon_pixels(), pixels(Tile::GrassSide));
        assert_eq!(Block::Log.icon_pixels(), pixels(Tile::LogSide));
        assert_eq!(Block::Stone.icon_pixels(), pixels(Tile::Stone));
        assert_eq!(Block::Air.icon_pixels(), None);
        // Зелёная кайма сверху и земля ниже: иконка травы не сплошная.
        let grass = Block::Grass.icon_pixels().unwrap();
        let (top, bottom) = (&grass[..3], &grass[grass.len() - 4..][..3]);
        assert_ne!(top, bottom);
    }

    #[test]
    fn solid_and_opaque() {
        for block in ALL {
            let plant = matches!(
                block,
                Block::TallGrass | Block::Poppy | Block::Dandelion | Block::Torch
            );
            let solid = !matches!(block, Block::Air | Block::Water) && !plant;
            assert_eq!(block.is_solid(), solid, "{block:?}");
            let opaque = !matches!(block, Block::Air | Block::Leaves | Block::Water) && !plant;
            assert_eq!(block.is_targetable(), solid || plant, "{block:?}");
            assert_eq!(block.needs_support(), plant, "{block:?}");
            assert_eq!(block.is_opaque(), opaque, "{block:?}");
            assert_eq!(block.is_water(), block == Block::Water, "{block:?}");
        }
    }

    #[test]
    fn render_layers() {
        for block in ALL {
            let layer = match block {
                Block::Leaves
                | Block::TallGrass
                | Block::Poppy
                | Block::Dandelion
                | Block::Torch => RenderLayer::Cutout,
                Block::Water => RenderLayer::Translucent,
                _ => RenderLayer::Opaque,
            };
            assert_eq!(block.render_layer(), layer, "{block:?}");
        }
        assert_eq!(Block::Water.tile(Facing::Top), Some(Tile::Water));
    }

    #[test]
    fn log_has_rings_on_ends_and_bark_on_sides() {
        assert_eq!(Block::Log.tile(Facing::Top), Some(Tile::LogTop));
        assert_eq!(Block::Log.tile(Facing::Bottom), Some(Tile::LogTop));
        assert_eq!(Block::Log.tile(Facing::Side), Some(Tile::LogSide));
        assert_eq!(Block::Sand.tile(Facing::Top), Some(Tile::Sand));
        assert_eq!(Block::Leaves.tile(Facing::Side), Some(Tile::Leaves));
    }

    #[test]
    fn bounds_follow_shape() {
        assert_eq!(Block::Stone.bounds(), (Vec3::ZERO, Vec3::ONE));
        assert_eq!(Block::Poppy.bounds(), (Vec3::ZERO, Vec3::ONE));
        let (lo, hi) = Block::Torch.bounds();
        assert_eq!((lo.x, hi.x, hi.y), (7.0 / 16.0, 9.0 / 16.0, 10.0 / 16.0));
    }

    #[test]
    fn replaceable_cells() {
        for block in ALL {
            let expected = matches!(block, Block::Air | Block::Water | Block::TallGrass);
            assert_eq!(block.is_replaceable(), expected, "{block:?}");
        }
    }

    #[test]
    fn shapes_and_emission() {
        for block in [Block::TallGrass, Block::Poppy, Block::Dandelion] {
            assert_eq!(block.shape(), Shape::Cross, "{block:?}");
            assert_eq!(block.emission(), 0);
        }
        assert_eq!(Block::Torch.shape(), Shape::Torch);
        assert_eq!(Block::Torch.emission(), 14);
        assert_eq!(Block::Stone.shape(), Shape::Cube);
        assert_eq!(Block::Water.shape(), Shape::Cube);
        assert!(!Block::Air.is_targetable() && !Block::Water.is_targetable());
    }

    #[test]
    fn new_blocks_selectable_by_name() {
        assert_eq!(Block::from_name("tall_grass"), Some(Block::TallGrass));
        assert_eq!(Block::from_name("poppy"), Some(Block::Poppy));
        assert_eq!(Block::from_name("dandelion"), Some(Block::Dandelion));
        assert_eq!(Block::from_name("torch"), Some(Block::Torch));
        for block in [
            Block::TallGrass,
            Block::Poppy,
            Block::Dandelion,
            Block::Torch,
        ] {
            assert!(block.icon_pixels().is_some());
        }
    }

    #[test]
    fn table_rows_follow_save_numbers() {
        assert_eq!(BLOCKS.len(), Block::ALL.len());
        for (i, block) in Block::ALL.into_iter().enumerate() {
            assert_eq!(block as usize, i, "{block:?}");
            assert_eq!(block.def().name, BLOCKS[i].name);
        }
    }

    #[test]
    fn names_are_unique_and_labeled() {
        let mut names: Vec<_> = BLOCKS.iter().map(|d| d.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), BLOCKS.len());
        assert!(
            BLOCKS
                .iter()
                .all(|d| !d.name.is_empty() && !d.label.is_empty())
        );
        assert_eq!(Block::TallGrass.def().label, "высокая трава");
    }

    #[test]
    fn air_has_no_tiles_and_others_have_all() {
        let air = Block::Air.def();
        assert!(air.top.is_none() && air.side.is_none() && air.bottom.is_none());
        for block in Block::ALL.into_iter().skip(1) {
            let def = block.def();
            assert!(def.top.is_some() && def.side.is_some() && def.bottom.is_some());
        }
    }

    #[test]
    fn support_is_for_non_solid_blocks() {
        for def in &BLOCKS {
            assert!(!(def.needs_support && def.solid), "{}", def.name);
            // Непрозрачный блок обязан быть твёрдым кубом.
            assert!(
                !def.opaque || def.solid && def.shape == Shape::Cube,
                "{}",
                def.name
            );
        }
    }
}
