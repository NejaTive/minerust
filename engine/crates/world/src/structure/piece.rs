use glam::IVec3;

use super::cell::{Cell, Selector};
use super::template::{Entry, Template};
use crate::block::{Block, VINE_SIDES, facing_code};
use crate::column::{Column, WORLD_HEIGHT};
use crate::pos::ColumnPos;
use crate::section::SECTION_SIZE;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Facing {
    North,
    East,
    South,
    West,
}

impl Facing {
    pub const ALL: [Facing; 4] = [Facing::North, Facing::East, Facing::South, Facing::West];

    pub fn rotated_right(self) -> Facing {
        match self {
            Facing::North => Facing::East,
            Facing::East => Facing::South,
            Facing::South => Facing::West,
            Facing::West => Facing::North,
        }
    }

    pub fn rotated_left(self) -> Facing {
        self.rotated_right().rotated_right().rotated_right()
    }

    pub fn opposite(self) -> Facing {
        self.rotated_right().rotated_right()
    }

    /// Единичный сдвиг в мире: север — −Z, восток — +X.
    pub fn step(self) -> IVec3 {
        match self {
            Facing::North => IVec3::new(0, 0, -1),
            Facing::East => IVec3::new(1, 0, 0),
            Facing::South => IVec3::new(0, 0, 1),
            Facing::West => IVec3::new(-1, 0, 0),
        }
    }
}

/// Замкнутый параллелепипед блоков: `min` и `max` входят.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct BlockBox {
    pub min: IVec3,
    pub max: IVec3,
}

impl BlockBox {
    pub fn new(a: IVec3, b: IVec3) -> Self {
        Self {
            min: a.min(b),
            max: a.max(b),
        }
    }

    /// `StructureComponent.getComponentToAddBoundingBox`: рамка размера `size` (ширина, высота, глубина) от `origin`,
    /// где `offset` и `size` заданы в локальных осях части, повёрнутых по `facing`.
    pub fn oriented(origin: IVec3, offset: IVec3, size: IVec3, facing: Facing) -> Self {
        let (x, y, z) = (origin.x, origin.y, origin.z);
        let (w, h, d) = (size.x, size.y, size.z);
        let (ox, oy, oz) = (offset.x, offset.y, offset.z);
        let (low, high) = match facing {
            Facing::North => (
                IVec3::new(x + ox, y + oy, z - d + 1 + oz),
                IVec3::new(x + w - 1 + ox, y + h - 1 + oy, z + oz),
            ),
            Facing::South => (
                IVec3::new(x + ox, y + oy, z + oz),
                IVec3::new(x + w - 1 + ox, y + h - 1 + oy, z + d - 1 + oz),
            ),
            Facing::West => (
                IVec3::new(x - d + 1 + oz, y + oy, z + ox),
                IVec3::new(x + oz, y + h - 1 + oy, z + w - 1 + ox),
            ),
            Facing::East => (
                IVec3::new(x + oz, y + oy, z + ox),
                IVec3::new(x + d - 1 + oz, y + h - 1 + oy, z + w - 1 + ox),
            ),
        };
        Self::new(low, high)
    }

    /// Рамка размера `size` (ширина, высота, глубина) в осях части, у которой локальная клетка (0, 0, 0)
    /// лежит в мировой `origin`; обратна `PieceWriter::world`.
    pub fn from_local(origin: IVec3, size: IVec3, facing: Facing) -> Self {
        let (w, h, d) = (size.x, size.y, size.z);
        let top = h - 1;
        let (low, high) = match facing {
            Facing::South => (IVec3::new(0, 0, 0), IVec3::new(w - 1, top, d - 1)),
            Facing::North => (IVec3::new(0, 0, 1 - d), IVec3::new(w - 1, top, 0)),
            Facing::West => (IVec3::new(1 - d, 0, 0), IVec3::new(0, top, w - 1)),
            Facing::East => (IVec3::new(0, 0, 0), IVec3::new(d - 1, top, w - 1)),
        };
        Self::new(origin + low, origin + high)
    }

    pub fn of_column(key: ColumnPos) -> Self {
        let origin = key.origin();
        Self {
            min: origin,
            max: origin + IVec3::new(SECTION_SIZE - 1, WORLD_HEIGHT - 1, SECTION_SIZE - 1),
        }
    }

    pub fn contains(&self, at: IVec3) -> bool {
        at.cmpge(self.min).all() && at.cmple(self.max).all()
    }

    pub fn contains_xz(&self, x: i32, z: i32) -> bool {
        (self.min.x..=self.max.x).contains(&x) && (self.min.z..=self.max.z).contains(&z)
    }

    pub fn intersects(&self, other: &BlockBox) -> bool {
        self.intersects_xz(other) && self.max.y >= other.min.y && self.min.y <= other.max.y
    }

    pub fn intersects_xz(&self, other: &BlockBox) -> bool {
        self.max.x >= other.min.x
            && self.min.x <= other.max.x
            && self.max.z >= other.min.z
            && self.min.z <= other.max.z
    }

    pub fn union(&self, other: &BlockBox) -> BlockBox {
        BlockBox {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    pub fn size(&self) -> IVec3 {
        self.max - self.min + IVec3::ONE
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PieceKind {
    SwampHut,
    Igloo,
    DesertTemple,
    JungleTemple,
    /// Шахта и комната под иглу: число сегментов шахты.
    IglooBasement(u8),
    #[cfg(test)]
    Probe,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Piece {
    pub bounds: BlockBox,
    pub facing: Facing,
    pub kind: PieceKind,
    /// Соль случайности внутри `place`: из `layout`, чтобы колонки строились в любом порядке.
    pub salt: u32,
}

impl Piece {
    pub fn place(&self, writer: &mut PieceWriter) {
        match self.kind {
            PieceKind::SwampHut => super::swamp_hut::place(writer),
            PieceKind::Igloo => super::igloo::place_top(writer),
            PieceKind::DesertTemple => super::desert_temple::place(writer),
            PieceKind::JungleTemple => super::jungle_temple::place(writer),
            PieceKind::IglooBasement(segments) => super::igloo::place_basement(writer, segments),
            #[cfg(test)]
            PieceKind::Probe => super::tests::draw_probe(writer),
        }
    }
}

fn vine_side(toward_wall: IVec3) -> u8 {
    VINE_SIDES
        .iter()
        .find_map(|&(bit, dir)| (dir == toward_wall).then_some(bit))
        .unwrap_or(0)
}

/// Коробка в локальных координатах части, обе границы входят, как в 1.12.
pub fn bx(x0: i32, y0: i32, z0: i32, x1: i32, y1: i32, z1: i32) -> BlockBox {
    BlockBox::new(IVec3::new(x0, y0, z0), IVec3::new(x1, y1, z1))
}

/// Хэш клетки части: не зависит от порядка генерации колонок.
fn mix(salt: u32, x: i32, y: i32, z: i32) -> u32 {
    let mut h = salt
        ^ (x as u32).wrapping_mul(0x9E37_79B1)
        ^ (y as u32).wrapping_mul(0x85EB_CA77)
        ^ (z as u32).wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^ h >> 15
}

/// Запись блоков части в одну колонку: всё вне колонки молча отбрасывается, поэтому колонки не зависят друг от друга.
pub struct PieceWriter<'a> {
    column: &'a mut Column,
    key: ColumnPos,
    frame: BlockBox,
    bounds: BlockBox,
    facing: Facing,
    salt: u32,
}

impl<'a> PieceWriter<'a> {
    pub fn new(column: &'a mut Column, key: ColumnPos, piece: &Piece) -> Self {
        Self {
            column,
            key,
            frame: BlockBox::of_column(key),
            bounds: piece.bounds,
            facing: piece.facing,
            salt: piece.salt,
        }
    }

    pub fn key(&self) -> ColumnPos {
        self.key
    }

    pub fn bounds(&self) -> BlockBox {
        self.bounds
    }

    pub fn facing(&self) -> Facing {
        self.facing
    }

    /// Локальные координаты части (x вдоль ширины, z вдоль глубины) в мировые, как `getXWithOffset`/`getZWithOffset`.
    pub fn world(&self, x: i32, y: i32, z: i32) -> IVec3 {
        let b = &self.bounds;
        let (wx, wz) = match self.facing {
            Facing::North => (b.min.x + x, b.max.z - z),
            Facing::South => (b.min.x + x, b.min.z + z),
            Facing::West => (b.max.x - z, b.min.z + x),
            Facing::East => (b.min.x + z, b.min.z + x),
        };
        IVec3::new(wx, b.min.y + y, wz)
    }

    fn local(&self, at: IVec3) -> Option<IVec3> {
        self.frame.contains(at).then(|| at - self.frame.min)
    }

    /// `setBlockState`.
    pub fn put(&mut self, x: i32, y: i32, z: i32, cell: impl Into<Cell>) {
        let cell = cell.into();
        let data = match cell.front {
            Some(front) if cell.block == Block::Vine => vine_side(self.world_dir(front)),
            Some(front) => cell.data | facing_code(self.world_dir(front)),
            None => cell.data,
        };
        let at = self.world(x, y, z);
        self.set_world(at, cell.block, data);
    }

    /// Бросок «один из `one_in`» по хэшу клетки: не зависит от порядка колонок.
    pub fn chance(&self, x: i32, y: i32, z: i32, one_in: u32) -> bool {
        mix(!self.salt, x, y, z).is_multiple_of(one_in)
    }

    /// Число от 0 до `n` по хэшу клетки.
    pub fn pick(&self, x: i32, y: i32, z: i32, n: u32) -> u32 {
        mix(self.salt, x, y, z) % n
    }

    /// Направление части (x вдоль ширины, z вдоль глубины) в мировое.
    pub fn world_dir(&self, local: IVec3) -> IVec3 {
        self.world(local.x, local.y, local.z) - self.world(0, 0, 0)
    }

    fn set_world(&mut self, at: IVec3, block: Block, data: u8) {
        if let Some(local) = self.local(at) {
            self.column.set_with_data(local, block, data);
        }
    }

    /// Блок под координатой части; вне колонки — воздух, как у `getBlockStateFromPos` вне рамки.
    pub fn get(&self, x: i32, y: i32, z: i32) -> Block {
        self.local(self.world(x, y, z))
            .map_or(Block::Air, |local| self.column.get(local))
    }

    /// `fillWithBlocks` с воздухом внутри: `fill(bx, AIR)` — `fillWithAir`.
    pub fn fill(&mut self, area: BlockBox, cell: impl Into<Cell>) {
        let cell = cell.into();
        self.each(area, |w, x, y, z| w.put(x, y, z, cell));
    }

    /// Рамка `wall` вокруг `inside`.
    pub fn hollow(&mut self, area: BlockBox, wall: impl Into<Cell>, inside: impl Into<Cell>) {
        let (wall, inside) = (wall.into(), inside.into());
        let (min, max) = (area.min, area.max);
        self.each(area, |w, x, y, z| {
            let on_edge =
                x == min.x || x == max.x || y == min.y || y == max.y || z == min.z || z == max.z;
            w.put(x, y, z, if on_edge { wall } else { inside });
        });
    }

    /// `fillWithBlocks(.., existingOnly)`: только клетки, где уже что-то есть.
    pub fn fill_existing(&mut self, area: BlockBox, cell: impl Into<Cell>) {
        let cell = cell.into();
        self.each(area, |w, x, y, z| {
            if w.get(x, y, z) != Block::Air {
                w.put(x, y, z, cell);
            }
        });
    }

    /// `fillWithRandomizedBlocks`.
    pub fn fill_random(&mut self, area: BlockBox, selector: &Selector) {
        self.each(area, |w, x, y, z| {
            let cell = selector.pick(w, x, y, z);
            w.put(x, y, z, cell);
        });
    }

    /// `generateMaybeBox`: каждая клетка с шансом «один из `one_in`».
    pub fn fill_chance(&mut self, area: BlockBox, one_in: u32, cell: impl Into<Cell>) {
        let cell = cell.into();
        self.each(area, |w, x, y, z| w.put_chance(x, y, z, one_in, cell));
    }

    /// `randomlyPlaceBlock`.
    pub fn put_chance(&mut self, x: i32, y: i32, z: i32, one_in: u32, cell: impl Into<Cell>) {
        if self.chance(x, y, z, one_in) {
            self.put(x, y, z, cell);
        }
    }

    /// Ставит шаблон, сдвинув его начало в `(ox, oy, oz)` части.
    pub fn template(&mut self, template: &Template, offset: IVec3) {
        let size = template.size();
        for y in 0..size.y {
            for z in 0..size.z {
                for x in 0..size.x {
                    let (px, py, pz) = (x + offset.x, y + offset.y, z + offset.z);
                    match template.entry(x, y, z) {
                        Entry::Keep => {}
                        Entry::Cell(cell) => self.put(px, py, pz, *cell),
                        Entry::Random(selector) => {
                            let cell = selector.pick(self, px, py, pz);
                            self.put(px, py, pz, cell);
                        }
                        Entry::Chance(one_in, cell) => {
                            self.put_chance(px, py, pz, *one_in, *cell);
                        }
                    }
                }
            }
        }
    }

    fn each(&mut self, area: BlockBox, mut visit: impl FnMut(&mut Self, i32, i32, i32)) {
        for y in area.min.y..=area.max.y {
            for z in area.min.z..=area.max.z {
                for x in area.min.x..=area.max.x {
                    visit(self, x, y, z);
                }
            }
        }
    }

    /// `replaceAirAndLiquidDownwards`: ставит блок и тянет его вниз сквозь воздух и воду.
    pub fn fill_down(&mut self, x: i32, y: i32, z: i32, block: Block) {
        let top = self.world(x, y, z);
        let Some(mut local) = self.local(top) else {
            return;
        };
        self.column.set(local, block);
        local.y -= 1;
        while local.y > 0
            && (self.column.get(local) == Block::Air || self.column.get(local).is_water())
        {
            self.column.set(local, block);
            local.y -= 1;
        }
    }

    /// `clearCurrentPositionBlocksUpwards`: убирает всё непустое от точки вверх до первого воздуха.
    pub fn clear_up(&mut self, x: i32, y: i32, z: i32) {
        let Some(mut local) = self.local(self.world(x, y, z)) else {
            return;
        };
        while local.y < WORLD_HEIGHT - 1 && self.column.get(local) != Block::Air {
            self.column.set(local, Block::Air);
            local.y += 1;
        }
    }
}
