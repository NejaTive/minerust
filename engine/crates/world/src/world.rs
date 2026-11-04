use std::collections::{HashMap, HashSet};

use glam::{IVec2, IVec3};

use crate::pos::{ColumnPos, SectionPos};

use crate::block::{Block, FLUID_SOURCE, FluidKind, VINE_SIDES, VINE_TOP, fluid_height};
use crate::boxes::BoxList;
use crate::column::{COLUMN_SECTIONS, Column, WORLD_HEIGHT};
use crate::mob::MobRecord;
use crate::section::SECTION_SIZE;

pub struct World {
    columns: HashMap<ColumnPos, Column>,
    edited: HashSet<ColumnPos>,
    stashed: HashMap<ColumnPos, Column>,
    dirty: HashSet<ColumnPos>,
    fluid_wakes: Vec<IVec3>,
}

const HORIZONTAL: [IVec3; 4] = [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z];

impl World {
    pub fn new() -> Self {
        Self {
            columns: HashMap::new(),
            edited: HashSet::new(),
            stashed: HashMap::new(),
            dirty: HashSet::new(),
            fluid_wakes: Vec::new(),
        }
    }

    pub fn block(&self, pos: IVec3) -> Block {
        if pos.y < 0 {
            return Block::Stone;
        }
        if pos.y >= WORLD_HEIGHT {
            return Block::Air;
        }
        let (key, local) = split(pos);
        self.columns
            .get(&key)
            .map_or(Block::Air, |column| column.get(local))
    }

    pub fn is_loaded(&self, pos: IVec3) -> bool {
        self.columns.contains_key(&ColumnPos::of_block(pos))
    }

    pub fn fluid_height(&self, pos: IVec3) -> Option<(FluidKind, f32)> {
        let kind = self.block(pos).fluid()?;
        let height = if self.block(pos + IVec3::Y).fluid() == Some(kind) {
            1.0
        } else {
            fluid_height(self.block_data(pos))
        };
        Some((kind, height))
    }

    pub fn neighborhood(&self, key: ColumnPos) -> [Option<&Column>; 9] {
        std::array::from_fn(|i| self.column(key + IVec2::new(i as i32 % 3 - 1, i as i32 / 3 - 1)))
    }

    pub fn boxes_at(&self, pos: IVec3) -> BoxList {
        self.block(pos).boxes(self.block_data(pos), |offset| {
            (self.block(pos + offset), self.block_data(pos + offset))
        })
    }

    pub fn block_data(&self, pos: IVec3) -> u8 {
        if !(0..WORLD_HEIGHT).contains(&pos.y) {
            return 0;
        }
        let (key, local) = split(pos);
        self.columns
            .get(&key)
            .map_or(0, |column| column.get_data(local))
    }

    pub fn set_block(&mut self, pos: IVec3, block: Block) -> bool {
        self.set_block_with_data(pos, block, 0)
    }

    pub fn set_block_with_data(&mut self, pos: IVec3, block: Block, data: u8) -> bool {
        if !(0..WORLD_HEIGHT).contains(&pos.y) {
            return false;
        }
        let (key, local) = split(pos);
        let Some(column) = self.columns.get_mut(&key) else {
            return false;
        };
        column.set_with_data(local, block, data);
        self.edited.insert(key);
        self.dirty.insert(key);
        true
    }

    pub fn vine_sides(&self, pos: IVec3, data: u8) -> u8 {
        let above = self.block(pos + IVec3::Y);
        let above_data = self.block_data(pos + IVec3::Y);
        VINE_SIDES
            .iter()
            .filter(|&&(bit, dir)| {
                data & bit != 0
                    && (self.block(pos + dir).holds_vine()
                        || (bit != VINE_TOP && above == Block::Vine && above_data & bit != 0))
            })
            .fold(0, |sides, &(bit, _)| sides | bit)
    }

    pub fn is_supported(&self, pos: IVec3, block: Block, data: u8) -> bool {
        if block == Block::Vine {
            return self.vine_sides(pos, data) != 0;
        }
        if let Some(upper) = block.double_upper()
            && self.block(pos + IVec3::Y) != upper
        {
            return false;
        }
        let Some(dir) = block.support_dir(data) else {
            return true;
        };
        let support = self.block(pos + dir);
        if !(block.stands_on(support) && (dir.y != 0 || support.is_opaque())) {
            return false;
        }
        if block.checks_sides() && self.has_solid_side(pos) {
            return false;
        }
        match block {
            Block::Reeds => support == Block::Reeds || self.water_beside(pos + dir),
            Block::Waterlily => self.block_data(pos + dir) == FLUID_SOURCE,
            _ => true,
        }
    }

    fn has_solid_side(&self, pos: IVec3) -> bool {
        HORIZONTAL
            .iter()
            .any(|&dir| self.block(pos + dir).is_solid())
    }

    fn water_beside(&self, pos: IVec3) -> bool {
        HORIZONTAL
            .iter()
            .any(|&dir| self.block(pos + dir).is_water())
    }

    pub fn can_place(&self, pos: IVec3, block: Block) -> bool {
        self.can_place_with_data(pos, block, 0)
    }

    pub fn can_place_with_data(&self, pos: IVec3, block: Block, data: u8) -> bool {
        self.block(pos).is_replaceable() && self.is_supported(pos, block, data)
    }

    pub fn set_block_supported(&mut self, pos: IVec3, block: Block) -> Vec<IVec3> {
        self.set_block_supported_with_data(pos, block, 0)
    }

    pub fn set_block_supported_with_data(
        &mut self,
        pos: IVec3,
        block: Block,
        data: u8,
    ) -> Vec<IVec3> {
        if !self.set_block_with_data(pos, block, data) {
            return Vec::new();
        }
        let mut changed = vec![pos];
        let mut next = 0;
        while next < changed.len() {
            let cell = changed[next];
            next += 1;
            for dir in [
                IVec3::Y,
                IVec3::NEG_Y,
                IVec3::X,
                IVec3::NEG_X,
                IVec3::Z,
                IVec3::NEG_Z,
            ] {
                let around = cell + dir;
                let (neighbor, data) = (self.block(around), self.block_data(around));
                if neighbor == Block::Vine {
                    let kept = self.vine_sides(around, data);
                    let moved = if kept == 0 {
                        self.set_block(around, Block::Air)
                    } else {
                        kept != data && self.set_block_with_data(around, neighbor, kept)
                    };
                    if moved {
                        changed.push(around);
                    }
                    continue;
                }
                let depends = neighbor.support_dir(data) == Some(-dir)
                    || (neighbor.checks_sides() && dir.y == 0)
                    || (neighbor.double_upper().is_some() && dir == IVec3::NEG_Y);
                if depends
                    && !self.is_supported(around, neighbor, data)
                    && self.set_block(around, Block::Air)
                {
                    changed.push(around);
                }
            }
        }
        changed
    }

    pub fn column(&self, key: ColumnPos) -> Option<&Column> {
        self.columns.get(&key)
    }

    pub fn insert(&mut self, key: ColumnPos, mut column: Column) {
        let origin = key.origin();
        self.fluid_wakes
            .extend(column.springs.drain(..).map(|spring| origin + spring));
        self.columns.insert(key, column);
    }

    pub fn drain_fluid_wakes(&mut self) -> Vec<IVec3> {
        std::mem::take(&mut self.fluid_wakes)
    }

    pub fn remove(&mut self, key: ColumnPos) -> Option<Column> {
        let column = self.columns.remove(&key)?;
        if self.edited.remove(&key) {
            self.stashed.insert(key, column.clone());
        }
        Some(column)
    }

    pub fn take_edited(&mut self, key: ColumnPos) -> Option<Column> {
        let column = self.stashed.remove(&key)?;
        self.edited.insert(key);
        Some(column)
    }

    pub fn is_edited(&self, key: ColumnPos) -> bool {
        self.edited.contains(&key) || self.stashed.contains_key(&key)
    }

    pub fn set_mobs(&mut self, key: ColumnPos, mobs: Vec<MobRecord>) {
        let Some(column) = self.columns.get_mut(&key) else {
            return;
        };
        if column.mobs == mobs {
            return;
        }
        column.mobs = mobs;
        self.edited.insert(key);
        self.dirty.insert(key);
    }

    pub fn populate(&mut self, key: ColumnPos, mobs: Vec<MobRecord>) {
        let Some(column) = self.columns.get_mut(&key) else {
            return;
        };
        column.populated = true;
        if mobs.is_empty() {
            return;
        }
        column.mobs = mobs;
        self.edited.insert(key);
        self.dirty.insert(key);
    }

    pub fn drain_dirty(&mut self) -> Vec<ColumnPos> {
        self.dirty.drain().collect()
    }

    pub fn edited_column(&self, key: ColumnPos) -> Option<&Column> {
        if self.edited.contains(&key) {
            self.columns.get(&key)
        } else {
            self.stashed.get(&key)
        }
    }

    pub fn import(&mut self, key: ColumnPos, column: Column) {
        if let Some(loaded) = self.columns.get_mut(&key) {
            *loaded = column;
            self.edited.insert(key);
        } else {
            self.stashed.insert(key, column);
        }
        self.dirty.remove(&key);
    }

    pub fn keys(&self) -> impl Iterator<Item = ColumnPos> + '_ {
        self.columns.keys().copied()
    }

    pub fn len(&self) -> usize {
        self.columns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

pub fn affected_sections(pos: IVec3) -> Vec<SectionPos> {
    let own = SectionPos::of_block(pos);
    let local = pos.rem_euclid(IVec3::splat(SECTION_SIZE));
    let span = |axis: usize| match local[axis] {
        0 => -1..=0,
        l if l == SECTION_SIZE - 1 => 0..=1,
        _ => 0..=0,
    };
    let mut keys = Vec::new();
    for dy in span(1) {
        for dz in span(2) {
            for dx in span(0) {
                keys.push(own + IVec3::new(dx, dy, dz));
            }
        }
    }
    keys.retain(|key| (0..COLUMN_SECTIONS).contains(&key.y()));
    keys
}

fn split(pos: IVec3) -> (ColumnPos, IVec3) {
    let key = ColumnPos::of_block(pos);
    let local = IVec3::new(
        pos.x.rem_euclid(SECTION_SIZE),
        pos.y,
        pos.z.rem_euclid(SECTION_SIZE),
    );
    (key, local)
}

#[cfg(test)]
mod tests;
