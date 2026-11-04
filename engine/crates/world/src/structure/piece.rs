use glam::IVec3;

use crate::block::{Block, CAULDRON_LEVELS, facing_code};
use crate::chest;
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
    #[cfg(test)]
    Probe,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Piece {
    pub bounds: BlockBox,
    pub facing: Facing,
    pub kind: PieceKind,
}

impl Piece {
    #[cfg(test)]
    pub fn place(&self, writer: &mut PieceWriter) {
        match self.kind {
            PieceKind::SwampHut => super::swamp_hut::place(writer),
            PieceKind::Probe => super::tests::draw_probe(writer),
        }
    }

    #[cfg(not(test))]
    pub fn place(&self, writer: &mut PieceWriter) {
        match self.kind {
            PieceKind::SwampHut => super::swamp_hut::place(writer),
        }
    }
}

/// Запись блоков части в одну колонку: всё вне колонки молча отбрасывается, поэтому колонки не зависят друг от друга.
pub struct PieceWriter<'a> {
    column: &'a mut Column,
    key: ColumnPos,
    frame: BlockBox,
    bounds: BlockBox,
    facing: Facing,
}

impl<'a> PieceWriter<'a> {
    pub fn new(column: &'a mut Column, key: ColumnPos, piece: &Piece) -> Self {
        Self {
            column,
            key,
            frame: BlockBox::of_column(key),
            bounds: piece.bounds,
            facing: piece.facing,
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

    pub fn set(&mut self, x: i32, y: i32, z: i32, block: Block) {
        self.set_with_data(x, y, z, block, 0);
    }

    pub fn set_with_data(&mut self, x: i32, y: i32, z: i32, block: Block, data: u8) {
        if let Some(local) = self.local(self.world(x, y, z)) {
            self.column.set_with_data(local, block, data);
        }
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

    /// Пустой одинарный сундук лицом в `front` (направление части); содержимое появится при первом открытии.
    pub fn chest(&mut self, x: i32, y: i32, z: i32, front: IVec3) {
        let data = chest::data(self.world_dir(front), chest::HALF_NONE);
        self.set_with_data(x, y, z, Block::Chest, data);
    }

    /// Двойной сундук: левая половина в `(x, y, z)`, правая — справа от неё, если смотреть на лицо.
    pub fn double_chest(&mut self, x: i32, y: i32, z: i32, front: IVec3) {
        let front = self.world_dir(front);
        let left = self.world(x, y, z);
        let right = left + chest::right_of(front);
        self.set_world(left, Block::Chest, chest::data(front, chest::HALF_LEFT));
        self.set_world(right, Block::Chest, chest::data(front, chest::HALF_RIGHT));
    }

    pub fn furnace(&mut self, x: i32, y: i32, z: i32, front: IVec3, lit: bool) {
        let block = if lit {
            Block::LitFurnace
        } else {
            Block::Furnace
        };
        let data = facing_code(self.world_dir(front));
        self.set_with_data(x, y, z, block, data);
    }

    /// Котёл с водой `level` от 0 до 3.
    pub fn cauldron(&mut self, x: i32, y: i32, z: i32, level: u8) {
        self.set_with_data(x, y, z, Block::Cauldron, level.min(CAULDRON_LEVELS));
    }

    /// Горшок с растением; растение вне списка горшка оставляет горшок пустым.
    pub fn flower_pot(&mut self, x: i32, y: i32, z: i32, plant: Option<Block>) {
        let data = plant.and_then(Block::pot_code).unwrap_or(0);
        self.set_with_data(x, y, z, Block::FlowerPot, data);
    }

    /// Блок под координатой части; вне колонки — воздух, как у `getBlockStateFromPos` вне рамки.
    pub fn get(&self, x: i32, y: i32, z: i32) -> Block {
        self.local(self.world(x, y, z))
            .map_or(Block::Air, |local| self.column.get(local))
    }

    pub fn fill(&mut self, from: IVec3, to: IVec3, block: Block) {
        for y in from.y..=to.y {
            for z in from.z..=to.z {
                for x in from.x..=to.x {
                    self.set(x, y, z, block);
                }
            }
        }
    }

    pub fn fill_hollow(&mut self, from: IVec3, to: IVec3, wall: Block, inside: Block) {
        for y in from.y..=to.y {
            for z in from.z..=to.z {
                for x in from.x..=to.x {
                    let on_edge = x == from.x
                        || x == to.x
                        || y == from.y
                        || y == to.y
                        || z == from.z
                        || z == to.z;
                    self.set(x, y, z, if on_edge { wall } else { inside });
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
