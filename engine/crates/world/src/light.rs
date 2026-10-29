use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};

use glam::{IVec2, IVec3};

use crate::block::{Block, Shape};
use crate::column::{Column, WORLD_HEIGHT};
use crate::pos::{ColumnPos, SectionPos};
use crate::section::SECTION_SIZE;
use crate::world::{World, affected_sections};

pub const MAX_LIGHT: u8 = 15;

const COLUMN_VOLUME: usize = (SECTION_SIZE * SECTION_SIZE * WORLD_HEIGHT) as usize;

const MIN_BRIGHTNESS: f32 = 0.05;

pub fn brightness(level: u8) -> f32 {
    level_brightness(f32::from(level))
}

pub fn level_brightness(level: f32) -> f32 {
    0.8f32
        .powf(f32::from(MAX_LIGHT) - level)
        .max(MIN_BRIGHTNESS)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Channel {
    Sky,
    Block,
}

pub fn pack_light(sky: u8, block: u8) -> u8 {
    debug_assert!(sky <= MAX_LIGHT && block <= MAX_LIGHT);
    sky | block << 4
}

pub fn sky_level(packed: u8) -> u8 {
    packed & MAX_LIGHT
}

pub fn block_level(packed: u8) -> u8 {
    packed >> 4
}

impl Channel {
    fn of(self, packed: u8) -> u8 {
        match self {
            Channel::Sky => sky_level(packed),
            Channel::Block => block_level(packed),
        }
    }

    fn with(self, packed: u8, level: u8) -> u8 {
        match self {
            Channel::Sky => pack_light(level, block_level(packed)),
            Channel::Block => pack_light(sky_level(packed), level),
        }
    }

    fn step(self, level: u8, down: bool, block: Block) -> u8 {
        if block.is_opaque() {
            return 0;
        }
        if self == Channel::Sky
            && down
            && level == MAX_LIGHT
            && (block == Block::Air || block.shape() != Shape::Cube)
        {
            return MAX_LIGHT;
        }
        level.saturating_sub(if block.is_water() { 2 } else { 1 })
    }
}

pub struct ColumnLight(Box<[u8; COLUMN_VOLUME]>);

impl ColumnLight {
    fn dark() -> Self {
        Self(
            vec![0; COLUMN_VOLUME]
                .into_boxed_slice()
                .try_into()
                .expect("length is COLUMN_VOLUME"),
        )
    }

    pub fn get(&self, local: IVec3) -> u8 {
        assert!(
            Column::contains(local),
            "local coords out of column: {local}"
        );
        self.0[column_index(local)]
    }
}

impl ColumnLight {
    pub fn row(&self, y: i32, z: i32) -> &[u8] {
        let start = column_index(IVec3::new(0, y, z));
        &self.0[start..start + SECTION_SIZE as usize]
    }
}

fn column_index(local: IVec3) -> usize {
    (local.x + SECTION_SIZE * local.z + SECTION_SIZE * SECTION_SIZE * local.y) as usize
}

pub struct LightMap {
    columns: HashMap<ColumnPos, ColumnLight>,
    block_lit: HashSet<ColumnPos>,
}

impl LightMap {
    pub fn new() -> Self {
        Self {
            columns: HashMap::new(),
            block_lit: HashSet::new(),
        }
    }

    pub fn sky(&self, pos: IVec3) -> u8 {
        sky_level(self.packed(pos))
    }

    pub fn block(&self, pos: IVec3) -> u8 {
        block_level(self.packed(pos))
    }

    pub fn packed(&self, pos: IVec3) -> u8 {
        if pos.y >= WORLD_HEIGHT {
            return MAX_LIGHT;
        }
        if pos.y < 0 {
            return 0;
        }
        let key = ColumnPos::of_block(pos);
        let local = IVec3::new(
            pos.x.rem_euclid(SECTION_SIZE),
            pos.y,
            pos.z.rem_euclid(SECTION_SIZE),
        );
        self.columns.get(&key).map_or(0, |light| light.get(local))
    }

    pub fn column(&self, key: ColumnPos) -> Option<&ColumnLight> {
        self.columns.get(&key)
    }

    pub fn neighborhood(&self, key: ColumnPos) -> [Option<&ColumnLight>; 9] {
        std::array::from_fn(|i| {
            self.columns
                .get(&(key + IVec2::new(i as i32 % 3 - 1, i as i32 / 3 - 1)))
        })
    }

    pub fn column_sampler(&self, key: ColumnPos) -> impl Fn(IVec3) -> u8 + '_ {
        let columns = self.neighborhood(key);
        move |local: IVec3| {
            if local.y >= WORLD_HEIGHT {
                return MAX_LIGHT;
            }
            let (dx, dz) = (
                local.x.div_euclid(SECTION_SIZE),
                local.z.div_euclid(SECTION_SIZE),
            );
            if local.y < 0 || !(-1..=1).contains(&dx) || !(-1..=1).contains(&dz) {
                return 0;
            }
            let inner = IVec3::new(
                local.x.rem_euclid(SECTION_SIZE),
                local.y,
                local.z.rem_euclid(SECTION_SIZE),
            );
            columns[(dx + 1 + 3 * (dz + 1)) as usize]
                .map_or(0, |light| light.0[column_index(inner)])
        }
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

    pub fn memory_bytes(&self) -> usize {
        self.columns.len() * COLUMN_VOLUME
    }

    pub fn add_column(&mut self, world: &World, key: ColumnPos) -> Vec<SectionPos> {
        let Some(column) = world.column(key) else {
            return Vec::new();
        };
        let mut light = ColumnLight::dark();
        let beams = sky_beams(column, &mut light);
        let mut changes = self.remove_column(world, key);
        self.columns.insert(key, light);

        let mut window = Window::new(&mut self.columns, world, key, false, Channel::Sky);
        window.seed_center(&beams);
        window.spread();
        changes.extend(window.changed_sections());

        let emits = (0..WORLD_HEIGHT / SECTION_SIZE).any(|sy| column.section(sy).emits_light());
        if emits || self.near_block_light(key) {
            let mut window = Window::new(&mut self.columns, world, key, false, Channel::Block);
            window.seed_emitters(column);
            window.spread();
            changes.extend(window.changed_sections());
            let touched = window.touched_columns();
            self.mark_block_lit(key, touched);
        }
        finish(changes, Some(key))
    }

    pub fn remove_column(&mut self, world: &World, key: ColumnPos) -> Vec<SectionPos> {
        let Some(old) = self.columns.remove(&key) else {
            return Vec::new();
        };
        let block_lit = self.block_lit.remove(&key);
        let mut changes = Vec::new();
        for channel in [Channel::Sky, Channel::Block] {
            if channel == Channel::Block && !block_lit {
                continue;
            }
            let mut window = Window::new(&mut self.columns, world, key, true, channel);
            let mut removal = Vec::new();
            for (dir, side) in SIDES {
                for y in 0..WORLD_HEIGHT {
                    for i in 0..SECTION_SIZE {
                        let edge = side.edge(i, y);
                        let level = channel.of(old.0[column_index(edge)]);
                        if level > 0 {
                            let cell = edge + IVec3::new(SECTION_SIZE, 0, SECTION_SIZE) + dir;
                            window.unlight_neighbor(cell, level, false, &mut removal);
                        }
                    }
                }
            }
            window.unspread(removal);
            window.spread();
            changes.extend(window.changed_sections());
            let touched = window.touched_columns();
            self.mark_block_lit(key, touched);
        }
        finish(changes, None)
    }

    pub fn block_changed(&mut self, world: &World, pos: IVec3) -> Vec<SectionPos> {
        let key = ColumnPos::of_block(pos);
        if !(0..WORLD_HEIGHT).contains(&pos.y) || !self.columns.contains_key(&key) {
            return Vec::new();
        }
        let mut changes = Vec::new();
        for channel in [Channel::Sky, Channel::Block] {
            if channel == Channel::Block
                && world.block(pos).emission() == 0
                && !self.near_block_light(key)
            {
                continue;
            }
            let mut window = Window::new(&mut self.columns, world, key, true, channel);
            let cell = pos - window.origin;
            let slot = window.slot(cell).expect("edited column is in the window");
            let old = window.level(slot);
            let mut removal = Vec::new();
            if old > 0 {
                window.set(cell, slot, 0);
                removal.push((cell, old));
            }
            window.unspread(removal);
            let emission = if channel == Channel::Block {
                window.block(slot).emission()
            } else {
                0
            };
            if emission > 0 {
                window.set(cell, slot, emission);
                window.enqueue(cell, emission);
            }
            for dir in DIRECTIONS {
                let neighbor = cell + dir;
                if let Some(slot) = window.slot(neighbor) {
                    window.enqueue(neighbor, window.level(slot));
                }
            }
            if channel == Channel::Sky && cell.y == WORLD_HEIGHT - 1 {
                window.relight_from_sky(cell);
            }
            window.spread();
            changes.extend(window.changed_sections());
            let touched = window.touched_columns();
            self.mark_block_lit(key, touched);
        }
        finish(changes, None)
    }

    fn near_block_light(&self, key: ColumnPos) -> bool {
        !self.block_lit.is_empty()
            && (-1..=1)
                .any(|dz| (-1..=1).any(|dx| self.block_lit.contains(&(key + IVec2::new(dx, dz)))))
    }

    fn mark_block_lit(&mut self, center: ColumnPos, touched: u16) {
        for i in 0..9 {
            if touched >> i & 1 != 0 {
                self.block_lit
                    .insert(center + IVec2::new(i % 3 - 1, i / 3 - 1));
            }
        }
    }
}

impl Default for LightMap {
    fn default() -> Self {
        Self::new()
    }
}

fn finish(keys: Vec<SectionPos>, skip: Option<ColumnPos>) -> Vec<SectionPos> {
    let mut keys: Vec<SectionPos> = keys
        .into_iter()
        .filter(|k| skip != Some(k.column()))
        .collect();
    keys.sort();
    keys.dedup();
    keys
}

struct Beams {
    bottom: [i32; 256],
    full: [i32; 256],
}

fn sky_beams(column: &Column, light: &mut ColumnLight) -> Beams {
    let mut beams = Beams {
        bottom: [WORLD_HEIGHT; 256],
        full: [WORLD_HEIGHT; 256],
    };
    for xz in 0..256 {
        let mut level = MAX_LIGHT;
        for y in (0..WORLD_HEIGHT).rev() {
            let index = xz + (y * SECTION_SIZE * SECTION_SIZE) as usize;
            level = Channel::Sky.step(level, true, column.get_index(index));
            if level == 0 {
                break;
            }
            light.0[index] = level;
            beams.bottom[xz] = y;
            if level == MAX_LIGHT {
                beams.full[xz] = y;
            }
        }
    }
    beams
}

const DIRECTIONS: [IVec3; 6] = [
    IVec3::X,
    IVec3::NEG_X,
    IVec3::Z,
    IVec3::NEG_Z,
    IVec3::Y,
    IVec3::NEG_Y,
];

#[derive(Clone, Copy)]
enum Side {
    MaxX,
    MinX,
    MaxZ,
    MinZ,
}

const SIDES: [(IVec3, Side); 4] = [
    (IVec3::X, Side::MaxX),
    (IVec3::NEG_X, Side::MinX),
    (IVec3::Z, Side::MaxZ),
    (IVec3::NEG_Z, Side::MinZ),
];

impl Side {
    fn edge(self, i: i32, y: i32) -> IVec3 {
        let last = SECTION_SIZE - 1;
        match self {
            Side::MaxX => IVec3::new(last, y, i),
            Side::MinX => IVec3::new(0, y, i),
            Side::MaxZ => IVec3::new(i, y, last),
            Side::MinZ => IVec3::new(i, y, 0),
        }
    }
}

const SPAN: i32 = 3 * SECTION_SIZE;
const CENTER: usize = 4;

#[derive(Clone, Copy)]
struct Slot {
    column: usize,
    index: usize,
}

struct Window<'a> {
    light: [Option<&'a mut [u8; COLUMN_VOLUME]>; 9],
    blocks: [Option<&'a Column>; 9],
    origin: IVec3,
    queue: [Vec<IVec3>; MAX_LIGHT as usize + 1],
    original: HashMap<u32, u8, BuildHasherDefault<CellHasher>>,
    track_center: bool,
    channel: Channel,
    block_touched: u16,
}

impl<'a> Window<'a> {
    fn new(
        columns: &'a mut HashMap<ColumnPos, ColumnLight>,
        world: &'a World,
        center: ColumnPos,
        track_center: bool,
        channel: Channel,
    ) -> Self {
        let keys: [ColumnPos; 9] =
            std::array::from_fn(|i| center + IVec2::new(i as i32 % 3 - 1, i as i32 / 3 - 1));
        let mut light = columns
            .get_disjoint_mut(keys.each_ref())
            .map(|l| l.map(|l| &mut *l.0));
        let blocks = std::array::from_fn(|i| {
            let column = world.column(keys[i]);
            if column.is_none() {
                light[i] = None;
            }
            column.filter(|_| light[i].is_some())
        });
        Self {
            light,
            blocks,
            origin: (center + IVec2::NEG_ONE).origin(),
            queue: Default::default(),
            original: HashMap::default(),
            track_center,
            channel,
            block_touched: 0,
        }
    }

    fn slot(&self, cell: IVec3) -> Option<Slot> {
        if !(0..SPAN).contains(&cell.x)
            || !(0..SPAN).contains(&cell.z)
            || !(0..WORLD_HEIGHT).contains(&cell.y)
        {
            return None;
        }
        let column = (cell.x / SECTION_SIZE + 3 * (cell.z / SECTION_SIZE)) as usize;
        self.light[column].as_ref()?;
        let local = IVec3::new(cell.x % SECTION_SIZE, cell.y, cell.z % SECTION_SIZE);
        Some(Slot {
            column,
            index: column_index(local),
        })
    }

    fn level(&self, slot: Slot) -> u8 {
        self.channel
            .of(self.light[slot.column].as_ref().expect("slot is lit")[slot.index])
    }

    fn block(&self, slot: Slot) -> Block {
        self.blocks[slot.column]
            .expect("slot has blocks")
            .get_index(slot.index)
    }

    fn set(&mut self, cell: IVec3, slot: Slot, level: u8) {
        let light = self.light[slot.column].as_mut().expect("slot is lit");
        let packed = light[slot.index];
        if self.track_center || slot.column != CENTER {
            self.original
                .entry(pack(cell))
                .or_insert(self.channel.of(packed));
        }
        light[slot.index] = self.channel.with(packed, level);
        if self.channel == Channel::Block && level > 0 {
            self.block_touched |= 1 << slot.column;
        }
    }

    fn enqueue(&mut self, cell: IVec3, level: u8) {
        if level > 1 {
            self.queue[usize::from(level)].push(cell);
        }
    }

    fn raise(&mut self, cell: IVec3, slot: Slot, level: u8) {
        if level > self.level(slot) {
            self.set(cell, slot, level);
            self.enqueue(cell, level);
        }
    }

    fn seed_center(&mut self, beams: &Beams) {
        let base = IVec3::new(SECTION_SIZE, 0, SECTION_SIZE);
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                let xz = (x + SECTION_SIZE * z) as usize;
                let column = base + IVec3::new(x, 0, z);
                for dir in &DIRECTIONS[..4] {
                    let beside = column + *dir;
                    let outside = !(0..SECTION_SIZE).contains(&(x + dir.x))
                        || !(0..SECTION_SIZE).contains(&(z + dir.z));
                    let beside_full = if outside {
                        let Some(full) = self.full_from(beside) else {
                            continue;
                        };
                        full
                    } else {
                        beams.full[(xz as i32 + dir.x + SECTION_SIZE * dir.z) as usize]
                    };
                    let top = beams.full[xz].max(beside_full);
                    for y in beams.bottom[xz]..top {
                        let cell = column.with_y(y);
                        let level = self.level(self.slot(cell).expect("center is lit"));
                        self.light_neighbor(beside.with_y(y), level, false);
                    }
                    if outside {
                        for y in 0..top {
                            let level = self.level(self.slot(beside.with_y(y)).expect("lit"));
                            self.light_neighbor(column.with_y(y), level, false);
                        }
                    }
                }
            }
        }
    }

    fn seed_emitters(&mut self, column: &Column) {
        let base = IVec3::new(SECTION_SIZE, 0, SECTION_SIZE);
        for sy in 0..WORLD_HEIGHT / SECTION_SIZE {
            if !column.section(sy).emits_light() {
                continue;
            }
            for y in sy * SECTION_SIZE..(sy + 1) * SECTION_SIZE {
                for z in 0..SECTION_SIZE {
                    for x in 0..SECTION_SIZE {
                        let local = IVec3::new(x, y, z);
                        let emission = column.get_index(column_index(local)).emission();
                        if emission > 0 {
                            let cell = base + local;
                            let slot = self.slot(cell).expect("center is lit");
                            self.set(cell, slot, emission);
                            self.enqueue(cell, emission);
                        }
                    }
                }
            }
        }
        for (dir, side) in SIDES {
            for y in 0..WORLD_HEIGHT {
                for i in 0..SECTION_SIZE {
                    let beside = side.edge(i, y) + base + dir;
                    if let Some(slot) = self.slot(beside) {
                        self.enqueue(beside, self.level(slot));
                    }
                }
            }
        }
    }

    fn full_from(&self, column: IVec3) -> Option<i32> {
        let mut full = WORLD_HEIGHT;
        for y in (0..WORLD_HEIGHT).rev() {
            if self.level(self.slot(column.with_y(y))?) != MAX_LIGHT {
                break;
            }
            full = y;
        }
        Some(full)
    }

    fn light_neighbor(&mut self, cell: IVec3, level: u8, down: bool) {
        if level <= 1 {
            return;
        }
        if let Some(slot) = self.slot(cell) {
            let lit = self.channel.step(level, down, self.block(slot));
            self.raise(cell, slot, lit);
        }
    }

    fn spread(&mut self) {
        for level in (2..=MAX_LIGHT).rev() {
            while let Some(cell) = self.queue[usize::from(level)].pop() {
                if self.slot(cell).is_none_or(|slot| self.level(slot) != level) {
                    continue;
                }
                for dir in DIRECTIONS {
                    self.light_neighbor(cell + dir, level, dir == IVec3::NEG_Y);
                }
            }
        }
    }

    fn unspread(&mut self, mut removal: Vec<(IVec3, u8)>) {
        let mut next = 0;
        while let Some(&(cell, old)) = removal.get(next) {
            next += 1;
            for dir in DIRECTIONS {
                self.unlight_neighbor(cell + dir, old, dir == IVec3::NEG_Y, &mut removal);
            }
        }
    }

    fn unlight_neighbor(
        &mut self,
        cell: IVec3,
        old: u8,
        down: bool,
        removal: &mut Vec<(IVec3, u8)>,
    ) {
        let Some(slot) = self.slot(cell) else {
            return;
        };
        let level = self.level(slot);
        if level == 0 {
            return;
        }
        let emitter = self.channel == Channel::Block && self.block(slot).emission() > 0;
        let sunbeam =
            self.channel == Channel::Sky && down && old == MAX_LIGHT && level == MAX_LIGHT;
        if !emitter && (level < old || sunbeam) {
            self.set(cell, slot, 0);
            removal.push((cell, level));
            if self.channel == Channel::Sky && cell.y == WORLD_HEIGHT - 1 {
                self.relight_from_sky(cell);
            }
        } else {
            self.enqueue(cell, level);
        }
    }

    fn relight_from_sky(&mut self, cell: IVec3) {
        self.light_neighbor(cell, MAX_LIGHT, true);
    }

    fn touched_columns(&self) -> u16 {
        self.block_touched
    }

    fn changed_sections(&self) -> Vec<SectionPos> {
        const SECTIONS: usize = 9 * (WORLD_HEIGHT / SECTION_SIZE) as usize;
        let mut seen = vec![false; SECTIONS * 27];
        let mut keys = HashSet::new();
        for (&packed, &original) in &self.original {
            let cell = unpack(packed);
            let slot = self.slot(cell).expect("changed cell is lit");
            if self.level(slot) == original {
                continue;
            }
            let section = cell / SECTION_SIZE;
            let edge = |c: i32| match c % SECTION_SIZE {
                0 => 0,
                l if l == SECTION_SIZE - 1 => 1,
                _ => 2,
            };
            let kind = edge(cell.x) + 3 * edge(cell.y) + 9 * edge(cell.z);
            let index = (section.x + 3 * section.z) as usize * SECTIONS / 9 + section.y as usize;
            if !std::mem::replace(&mut seen[index * 27 + kind as usize], true) {
                keys.extend(affected_sections(self.origin + cell));
            }
        }
        keys.into_iter().collect()
    }
}

fn pack(cell: IVec3) -> u32 {
    (cell.x + SPAN * cell.z + SPAN * SPAN * cell.y) as u32
}

fn unpack(packed: u32) -> IVec3 {
    let packed = packed as i32;
    IVec3::new(packed % SPAN, packed / (SPAN * SPAN), packed / SPAN % SPAN)
}

#[derive(Default)]
struct CellHasher(u64);

impl Hasher for CellHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = (self.0.rotate_left(5) ^ u64::from(byte)).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        }
    }

    fn write_u32(&mut self, value: u32) {
        self.0 = u64::from(value).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
}

#[cfg(test)]
mod bench;
#[cfg(test)]
mod tests;
