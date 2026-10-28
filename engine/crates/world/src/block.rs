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

impl Block {
    /// Для коллизий, прицела и проверки установки: всё, кроме воздуха и воды.
    pub fn is_solid(self) -> bool {
        !matches!(self, Block::Air | Block::Water) && self.shape() == Shape::Cube
    }

    /// Можно ли навести на блок прицелом: твёрдые и растения с факелом (они не
    /// мешают ходить, но их надо уметь сломать). Воздух и вода не выбираются.
    pub fn is_targetable(self) -> bool {
        self.is_solid() || self.shape() != Shape::Cube && self != Block::Air
    }

    pub fn shape(self) -> Shape {
        match self {
            Block::TallGrass | Block::Poppy | Block::Dandelion => Shape::Cross,
            Block::Torch => Shape::Torch,
            _ => Shape::Cube,
        }
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

    /// Можно ли поставить блок в клетку с этим содержимым: воздух, вода и трава (она
    /// заменяется).
    pub fn is_replaceable(self) -> bool {
        matches!(self, Block::Air | Block::Water | Block::TallGrass)
    }

    /// Нужна ли блоку твёрдая опора снизу: без неё он исчезает и ставится только на неё.
    pub fn needs_support(self) -> bool {
        self.shape() != Shape::Cube
    }

    /// Закрывает ли блок грань соседа и участвует ли в затенении углов: всё, кроме
    /// воздуха, листвы, воды и блоков не-кубов. Сквозь дырки листвы видно и грани за ней, и соседнюю
    /// листву; сквозь воду — дно и берег.
    pub fn is_opaque(self) -> bool {
        !matches!(self, Block::Air | Block::Leaves | Block::Water) && self.shape() == Shape::Cube
    }

    /// Уровень света блоков, который блок излучает (0 — не светится).
    pub fn emission(self) -> u8 {
        match self {
            Block::Glowstone => 15,
            // На единицу темнее светокамня: факел слабее блока-лампы.
            Block::Torch => 14,
            _ => 0,
        }
    }

    pub fn is_water(self) -> bool {
        self == Block::Water
    }

    /// Проход отрисовки. У воздуха граней нет, для него ответ не важен.
    pub fn render_layer(self) -> RenderLayer {
        match self {
            Block::Leaves | Block::TallGrass | Block::Poppy | Block::Dandelion | Block::Torch => {
                RenderLayer::Cutout
            }
            Block::Water => RenderLayer::Translucent,
            _ => RenderLayer::Opaque,
        }
    }

    /// Блок, который игрок может выбрать, по имени из интерфейса: `"grass"`, `"dirt"`,
    /// `"stone"`, `"sand"`, `"log"`, `"leaves"`, `"glowstone"`, `"tall_grass"`, `"poppy"`, `"dandelion"`, `"torch"`. Воздух не выбирается: для него есть ломание.
    pub fn from_name(name: &str) -> Option<Block> {
        match name {
            "grass" => Some(Block::Grass),
            "dirt" => Some(Block::Dirt),
            "stone" => Some(Block::Stone),
            "sand" => Some(Block::Sand),
            "log" => Some(Block::Log),
            "leaves" => Some(Block::Leaves),
            "glowstone" => Some(Block::Glowstone),
            "tall_grass" => Some(Block::TallGrass),
            "poppy" => Some(Block::Poppy),
            "dandelion" => Some(Block::Dandelion),
            "torch" => Some(Block::Torch),
            _ => None,
        }
    }

    /// Иконка для хотбара: пиксели боковой грани (16×16 RGBA, sRGB); `None` — у блока нет граней.
    pub fn icon_pixels(self) -> Option<Vec<u8>> {
        self.tile(Facing::Side).map(tile_pixels)
    }

    /// Тайл грани; `None` — у блока нет граней (воздух).
    pub fn tile(self, facing: Facing) -> Option<Tile> {
        Some(match (self, facing) {
            (Block::Air, _) => return None,
            (Block::Grass, Facing::Top) => Tile::GrassTop,
            (Block::Grass, Facing::Side) => Tile::GrassSide,
            (Block::Grass, Facing::Bottom) | (Block::Dirt, _) => Tile::Dirt,
            (Block::Stone, _) => Tile::Stone,
            (Block::Sand, _) => Tile::Sand,
            (Block::Log, Facing::Top | Facing::Bottom) => Tile::LogTop,
            (Block::Log, Facing::Side) => Tile::LogSide,
            (Block::Leaves, _) => Tile::Leaves,
            (Block::Water, _) => Tile::Water,
            (Block::Glowstone, _) => Tile::Glowstone,
            (Block::TallGrass, _) => Tile::TallGrass,
            (Block::Poppy, _) => Tile::Poppy,
            (Block::Dandelion, _) => Tile::Dandelion,
            (Block::Torch, _) => Tile::Torch,
        })
    }
}

/// Обратное к `block as u8`: так блоки читаются из сохранения. Ошибка — сам незнакомый
/// байт. Номера вариантов — часть формата сохранения: новые блоки добавлять в конец.
impl TryFrom<u8> for Block {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, u8> {
        Ok(match value {
            0 => Block::Air,
            1 => Block::Grass,
            2 => Block::Dirt,
            3 => Block::Stone,
            4 => Block::Sand,
            5 => Block::Log,
            6 => Block::Leaves,
            7 => Block::Water,
            8 => Block::Glowstone,
            9 => Block::TallGrass,
            10 => Block::Poppy,
            11 => Block::Dandelion,
            12 => Block::Torch,
            other => return Err(other),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Block; 13] = [
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
}
