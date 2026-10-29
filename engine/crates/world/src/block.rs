use glam::{IVec3, Vec3};

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

/// Что значит байт данных блока (`World::block_data`); сам байт хранится в секции и
/// сохраняется вместе с блоком. Нулевой байт — состояние по умолчанию у всех блоков.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DataKind {
    /// Блок данных не использует: байт всегда 0.
    None,
    /// Ось блока: [`AXIS_Y`] (0, стоячее), [`AXIS_X`] (1), [`AXIS_Z`] (2) — бревно.
    Axis,
    /// Куда прикреплён блок: [`ATTACH_FLOOR`] (0, держится за блок снизу) или
    /// [`ATTACH_POS_X`]..[`ATTACH_NEG_Z`] — за стену в этой стороне от блока (факел).
    Attachment,
}

/// Ось бревна в байте данных. Ноль — стоячее: так бревно из генерации и из
/// сохранений до версии 2 не меняется.
pub const AXIS_Y: u8 = 0;
pub const AXIS_X: u8 = 1;
pub const AXIS_Z: u8 = 2;

/// Крепление в байте данных: направление от блока к опоре. Ноль — опора снизу, как у
/// любого растения, поэтому сохранённые до версии 2 факелы остаются напольными.
pub const ATTACH_FLOOR: u8 = 0;
pub const ATTACH_POS_X: u8 = 1;
pub const ATTACH_NEG_X: u8 = 2;
pub const ATTACH_POS_Z: u8 = 3;
pub const ATTACH_NEG_Z: u8 = 4;

/// Настенный факел в шестнадцатых долях блока, для лежащей у стены в −X и наклонённой в
/// +X (остальные три стороны — поворотом вокруг центра клетки). Наклон 4/10 ≈ 22°:
/// шаги позиции целые, и другого угла рядом с 22° из них не собрать. Основание
/// прижато к стене, а поднят факел, чтобы его низ не висел в углу клетки.
pub const TORCH_WALL_SHIFT: u32 = 4;
pub const TORCH_WALL_RISE: u32 = 3;

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
    /// Нужна ли твёрдая опора (снизу, у факела — и сбоку, см. [`Block::support_dir`]):
    /// без неё блок исчезает и ставится только на неё.
    pub needs_support: bool,
    /// Смысл байта данных.
    pub data: DataKind,
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
        data: DataKind::None,
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
        data: DataKind::Axis,
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
        data: DataKind::Attachment,
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
    /// мерцал бы при наведении на соседние. Настенный факел наклонён, и бокс зависит
    /// от байта данных.
    pub fn bounds(self, data: u8) -> (Vec3, Vec3) {
        match self.shape() {
            Shape::Torch => {
                let unit = 1.0 / TILE_SIZE as f32;
                let (lo, hi) = (TORCH_PIXELS.0 as f32 * unit, TORCH_PIXELS.1 as f32 * unit);
                let height = (TILE_SIZE - TORCH_TOP_ROW) as f32 * unit;
                let Some(support) = self.support_dir(data).filter(|dir| dir.y == 0) else {
                    return (Vec3::new(lo, 0.0, lo), Vec3::new(hi, height, hi));
                };
                // Коробка для опоры в −X: основание у стены, верх отклонён в +X.
                let shift = TORCH_WALL_SHIFT as f32 * unit;
                let rise = TORCH_WALL_RISE as f32 * unit;
                let min = Vec3::new(0.0, rise, lo);
                let max = Vec3::new(hi - lo + shift, rise + height, hi);
                // Поворот вокруг центра клетки до нужной стены: (dx, dz) → (−dz, dx).
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

    /// Направление от блока к его опоре; `None` — опора не нужна. Растения держатся за
    /// блок снизу; факел — за блок снизу или за стену по байту данных. Неизвестный
    /// байт читается как напольный: сохранение с мусором не должно ронять игру.
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

    /// Байт данных при установке блока на грань с нормалью `normal` (от блока под
    /// прицелом к клетке установки). `None` — на эту грань блок не ставится: факел
    /// не висит на нижней грани.
    pub fn placement_data(self, normal: IVec3) -> Option<u8> {
        match self.data_kind() {
            DataKind::None => Some(0),
            DataKind::Axis => Some(match normal.abs().max_position() {
                0 => AXIS_X,
                1 => AXIS_Y,
                _ => AXIS_Z,
            }),
            // Факел: на пол — как раньше; на стену — опора там, откуда пришёл луч, то есть
            // против нормали.
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

    /// Ось блока с осью в данных: 0 — x, 1 — y, 2 — z; байт вне таблицы — стоячий.
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
mod tests;
