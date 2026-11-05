use glam::IVec3;

use super::MineshaftKind;
use super::cell::{AIR, Cell, NEG_X, NEG_Z, X, Z};
use super::piece::{BlockBox, Facing, Piece, PieceKind, PieceWriter, bx};
use crate::block::{Block, RAIL_EAST_WEST, RAIL_NORTH_SOUTH, SPAWNER_SPIDER};
use crate::stream::Stream;
use crate::terrain::{Terrain, WATER_LEVEL};

const SEA_LEVEL: i32 = WATER_LEVEL + 1;
const MAX_DEPTH: i32 = 8;
const MAX_DISTANCE: i32 = 80;
const START_OFFSET: i32 = 6;
const ROOM_FLOOR: i32 = 50;
const ROOM_MIN_WIDTH: i32 = 7;
const ROOM_MIN_TOP: i32 = 54;
const ROOM_SPREAD: u32 = 6;
const SECTION_LENGTH: i32 = 5;
const CORRIDOR_HEIGHT: i32 = 3;
const CROSS_EXTRA_HEIGHT: i32 = 4;
const STAIRS_DROP: i32 = 5;
const STAIRS_LENGTH: i32 = 8;
const GROUND_MARGIN: i32 = 10;
const MESA_LIFT: i32 = 5;
const NO_DOME: f32 = 1.05;
const SALT_LIMIT: u32 = 1 << 30;

const TAG_ROOF: u32 = 1;
const TAG_WEB_FILL: u32 = 2;
const TAG_WEB_SPOT: u32 = 3;
const TAG_TORCH: u32 = 4;
const TAG_RAIL: u32 = 5;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CorridorSpec {
    pub kind: MineshaftKind,
    pub sections: u8,
    pub rails: bool,
    pub spiders: bool,
    pub supports: u8,
    pub spawner: Option<u8>,
}

#[derive(Clone, Copy)]
enum Shape {
    Room,
    Corridor {
        rails: bool,
        spiders: bool,
        sections: u8,
    },
    Cross {
        entry: Facing,
    },
    Stairs,
}

#[derive(Clone, Copy)]
struct Part {
    bounds: BlockBox,
    facing: Facing,
    shape: Shape,
    depth: i32,
}

struct Shaft<'a> {
    parts: Vec<Part>,
    openings: Vec<BlockBox>,
    start: IVec3,
    rng: &'a mut Stream,
}

fn at(x: i32, y: i32, z: i32) -> IVec3 {
    IVec3::new(x, y, z)
}

impl Shaft<'_> {
    fn roll(&mut self, bound: i32) -> i32 {
        self.rng.below(bound as u32) as i32
    }

    fn blocked(&self, area: &BlockBox) -> bool {
        self.parts.iter().any(|part| part.bounds.intersects(area))
    }

    fn add(&mut self, x: i32, y: i32, z: i32, facing: Facing, depth: i32) -> Option<usize> {
        if depth > MAX_DEPTH
            || (x - self.start.x).abs() > MAX_DISTANCE
            || (z - self.start.z).abs() > MAX_DISTANCE
        {
            return None;
        }
        let part = self.random_part(x, y, z, facing, depth + 1)?;
        let index = self.parts.len();
        self.parts.push(part);
        self.build(index);
        Some(index)
    }

    fn random_part(&mut self, x: i32, y: i32, z: i32, facing: Facing, depth: i32) -> Option<Part> {
        let roll = self.roll(100);
        if roll >= 80 {
            self.cross(x, y, z, facing, depth)
        } else if roll >= 70 {
            self.stairs(x, y, z, facing, depth)
        } else {
            self.corridor(x, y, z, facing, depth)
        }
    }

    fn corridor(&mut self, x: i32, y: i32, z: i32, facing: Facing, depth: i32) -> Option<Part> {
        let mut sections = self.roll(3) + 2;
        let mut area = BlockBox::new(at(x, y, z), at(x, y + CORRIDOR_HEIGHT - 1, z));
        while sections > 0 {
            let reach = sections * SECTION_LENGTH - 1;
            match facing {
                Facing::North => {
                    area.max.x = x + 2;
                    area.min.z = z - reach;
                }
                Facing::South => {
                    area.max.x = x + 2;
                    area.max.z = z + reach;
                }
                Facing::West => {
                    area.min.x = x - reach;
                    area.max.z = z + 2;
                }
                Facing::East => {
                    area.max.x = x + reach;
                    area.max.z = z + 2;
                }
            }
            if !self.blocked(&area) {
                break;
            }
            sections -= 1;
        }
        if sections == 0 {
            return None;
        }
        let rails = self.roll(3) == 0;
        let spiders = !rails && self.roll(23) == 0;
        Some(Part {
            bounds: area,
            facing,
            shape: Shape::Corridor {
                rails,
                spiders,
                sections: sections as u8,
            },
            depth,
        })
    }

    fn cross(&mut self, x: i32, y: i32, z: i32, facing: Facing, depth: i32) -> Option<Part> {
        let mut area = BlockBox::new(at(x, y, z), at(x, y + CORRIDOR_HEIGHT - 1, z));
        if self.roll(4) == 0 {
            area.max.y += CROSS_EXTRA_HEIGHT;
        }
        match facing {
            Facing::North => {
                area.min.x = x - 1;
                area.max.x = x + 3;
                area.min.z = z - 4;
            }
            Facing::South => {
                area.min.x = x - 1;
                area.max.x = x + 3;
                area.max.z = z + 4;
            }
            Facing::West => {
                area.min.x = x - 4;
                area.min.z = z - 1;
                area.max.z = z + 3;
            }
            Facing::East => {
                area.max.x = x + 4;
                area.min.z = z - 1;
                area.max.z = z + 3;
            }
        }
        (!self.blocked(&area)).then_some(Part {
            bounds: area,
            facing: Facing::South,
            shape: Shape::Cross { entry: facing },
            depth,
        })
    }

    fn stairs(&mut self, x: i32, y: i32, z: i32, facing: Facing, depth: i32) -> Option<Part> {
        let mut area = BlockBox::new(at(x, y - STAIRS_DROP, z), at(x, y + 2, z));
        match facing {
            Facing::North => {
                area.max.x = x + 2;
                area.min.z = z - STAIRS_LENGTH;
            }
            Facing::South => {
                area.max.x = x + 2;
                area.max.z = z + STAIRS_LENGTH;
            }
            Facing::West => {
                area.min.x = x - STAIRS_LENGTH;
                area.max.z = z + 2;
            }
            Facing::East => {
                area.max.x = x + STAIRS_LENGTH;
                area.max.z = z + 2;
            }
        }
        (!self.blocked(&area)).then_some(Part {
            bounds: area,
            facing,
            shape: Shape::Stairs,
            depth,
        })
    }

    fn build(&mut self, index: usize) {
        let part = self.parts[index];
        let b = part.bounds;
        let depth = part.depth;
        match part.shape {
            Shape::Room => self.build_room(b, depth),
            Shape::Corridor { .. } => self.build_corridor(b, part.facing, depth),
            Shape::Cross { entry } => self.build_cross(b, entry, depth),
            Shape::Stairs => {
                let (x, z, facing) = match part.facing {
                    Facing::North => (b.min.x, b.min.z - 1, Facing::North),
                    Facing::South => (b.min.x, b.max.z + 1, Facing::South),
                    Facing::West => (b.min.x - 1, b.min.z, Facing::West),
                    Facing::East => (b.max.x + 1, b.min.z, Facing::East),
                };
                self.add(x, b.min.y, z, facing, depth);
            }
        }
    }

    fn build_room(&mut self, b: BlockBox, depth: i32) {
        let size = b.size();
        let floors = (size.y - 3 - 1).max(1);
        let mut j = 0;
        while j < size.x {
            j += self.roll(size.x);
            if j + 3 > size.x {
                break;
            }
            let y = b.min.y + self.roll(floors) + 1;
            if let Some(i) = self.add(b.min.x + j, y, b.min.z - 1, Facing::North, depth) {
                let c = self.parts[i].bounds;
                self.opening(c.min.x, c.max.y, b.min.z, c.max.x, b.min.z + 1);
            }
            j += 4;
        }
        j = 0;
        while j < size.x {
            j += self.roll(size.x);
            if j + 3 > size.x {
                break;
            }
            let y = b.min.y + self.roll(floors) + 1;
            if let Some(i) = self.add(b.min.x + j, y, b.max.z + 1, Facing::South, depth) {
                let c = self.parts[i].bounds;
                self.opening(c.min.x, c.max.y, b.max.z - 1, c.max.x, b.max.z);
            }
            j += 4;
        }
        j = 0;
        while j < size.z {
            j += self.roll(size.z);
            if j + 3 > size.z {
                break;
            }
            let y = b.min.y + self.roll(floors) + 1;
            if let Some(i) = self.add(b.min.x - 1, y, b.min.z + j, Facing::West, depth) {
                let c = self.parts[i].bounds;
                self.opening(b.min.x, c.max.y, c.min.z, b.min.x + 1, c.max.z);
            }
            j += 4;
        }
        j = 0;
        while j < size.z {
            j += self.roll(size.z);
            if j + 3 > size.z {
                break;
            }
            let y = b.min.y + self.roll(floors) + 1;
            if let Some(i) = self.add(b.max.x + 1, y, b.min.z + j, Facing::East, depth) {
                let c = self.parts[i].bounds;
                self.opening(b.max.x - 1, c.max.y, c.min.z, b.max.x, c.max.z);
            }
            j += 4;
        }
    }

    fn opening(&mut self, x0: i32, top: i32, z0: i32, x1: i32, z1: i32) {
        self.openings
            .push(BlockBox::new(at(x0, top - 2, z0), at(x1, top, z1)));
    }

    fn build_corridor(&mut self, b: BlockBox, facing: Facing, depth: i32) {
        let side = self.roll(4);
        let lift = self.roll(3);
        let y = b.min.y - 1 + lift;
        match facing {
            Facing::North => match side {
                0 | 1 => self.add(b.min.x, y, b.min.z - 1, facing, depth),
                2 => self.add(b.min.x - 1, y, b.min.z, Facing::West, depth),
                _ => self.add(b.max.x + 1, y, b.min.z, Facing::East, depth),
            },
            Facing::South => match side {
                0 | 1 => self.add(b.min.x, y, b.max.z + 1, facing, depth),
                2 => self.add(b.min.x - 1, y, b.max.z - 3, Facing::West, depth),
                _ => self.add(b.max.x + 1, y, b.max.z - 3, Facing::East, depth),
            },
            Facing::West => match side {
                0 | 1 => self.add(b.min.x - 1, y, b.min.z, facing, depth),
                2 => self.add(b.min.x, y, b.min.z - 1, Facing::North, depth),
                _ => self.add(b.min.x, y, b.max.z + 1, Facing::South, depth),
            },
            Facing::East => match side {
                0 | 1 => self.add(b.max.x + 1, y, b.min.z, facing, depth),
                2 => self.add(b.max.x - 3, y, b.min.z - 1, Facing::North, depth),
                _ => self.add(b.max.x - 3, y, b.max.z + 1, Facing::South, depth),
            },
        };
        if depth >= MAX_DEPTH {
            return;
        }
        if matches!(facing, Facing::North | Facing::South) {
            let mut z = b.min.z + 3;
            while z + 3 <= b.max.z {
                match self.roll(5) {
                    0 => self.add(b.min.x - 1, b.min.y, z, Facing::West, depth + 1),
                    1 => self.add(b.max.x + 1, b.min.y, z, Facing::East, depth + 1),
                    _ => None,
                };
                z += SECTION_LENGTH;
            }
        } else {
            let mut x = b.min.x + 3;
            while x + 3 <= b.max.x {
                match self.roll(5) {
                    0 => self.add(x, b.min.y, b.min.z - 1, Facing::North, depth + 1),
                    1 => self.add(x, b.min.y, b.max.z + 1, Facing::South, depth + 1),
                    _ => None,
                };
                x += SECTION_LENGTH;
            }
        }
    }

    fn build_cross(&mut self, b: BlockBox, entry: Facing, depth: i32) {
        let y = b.min.y;
        match entry {
            Facing::North => {
                self.add(b.min.x + 1, y, b.min.z - 1, Facing::North, depth);
                self.add(b.min.x - 1, y, b.min.z + 1, Facing::West, depth);
                self.add(b.max.x + 1, y, b.min.z + 1, Facing::East, depth);
            }
            Facing::South => {
                self.add(b.min.x + 1, y, b.max.z + 1, Facing::South, depth);
                self.add(b.min.x - 1, y, b.min.z + 1, Facing::West, depth);
                self.add(b.max.x + 1, y, b.min.z + 1, Facing::East, depth);
            }
            Facing::West => {
                self.add(b.min.x + 1, y, b.min.z - 1, Facing::North, depth);
                self.add(b.min.x + 1, y, b.max.z + 1, Facing::South, depth);
                self.add(b.min.x - 1, y, b.min.z + 1, Facing::West, depth);
            }
            Facing::East => {
                self.add(b.min.x + 1, y, b.min.z - 1, Facing::North, depth);
                self.add(b.min.x + 1, y, b.max.z + 1, Facing::South, depth);
                self.add(b.max.x + 1, y, b.min.z + 1, Facing::East, depth);
            }
        }
        if b.size().y > CORRIDOR_HEIGHT {
            let upper = y + CORRIDOR_HEIGHT + 1;
            if self.rng.flip() {
                self.add(b.min.x + 1, upper, b.min.z - 1, Facing::North, depth);
            }
            if self.rng.flip() {
                self.add(b.min.x - 1, upper, b.min.z + 1, Facing::West, depth);
            }
            if self.rng.flip() {
                self.add(b.max.x + 1, upper, b.min.z + 1, Facing::East, depth);
            }
            if self.rng.flip() {
                self.add(b.min.x + 1, upper, b.max.z + 1, Facing::South, depth);
            }
        }
    }
}

pub(super) fn layout(
    terrain: &Terrain,
    kind: MineshaftKind,
    anchor: (i32, i32),
    rng: &mut Stream,
) -> Vec<Piece> {
    let (x, z) = (anchor.0 - START_OFFSET, anchor.1 - START_OFFSET);
    let wide = |rng: &mut Stream| ROOM_MIN_WIDTH + rng.below(ROOM_SPREAD) as i32;
    let reach_x = wide(rng);
    let top = ROOM_MIN_TOP + rng.below(ROOM_SPREAD) as i32;
    let reach_z = wide(rng);
    let room = BlockBox::new(at(x, ROOM_FLOOR, z), at(x + reach_x, top, z + reach_z));
    let mut shaft = Shaft {
        parts: vec![Part {
            bounds: room,
            facing: Facing::South,
            shape: Shape::Room,
            depth: 0,
        }],
        openings: Vec::new(),
        start: room.min,
        rng,
    };
    shaft.build(0);
    let Shaft {
        mut parts,
        openings,
        rng,
        ..
    } = shaft;
    let span = parts
        .iter()
        .map(|part| part.bounds)
        .reduce(|a, b| a.union(&b))
        .unwrap_or(room);
    let lift = match kind {
        MineshaftKind::Mesa => SEA_LEVEL - span.max.y + span.size().y / 2 + MESA_LIFT,
        MineshaftKind::Normal => {
            let ceiling = SEA_LEVEL - GROUND_MARGIN;
            let mut top = span.size().y + 1;
            if top < ceiling {
                top += rng.below((ceiling - top) as u32) as i32;
            }
            top - span.max.y
        }
    };
    for part in &mut parts {
        part.bounds = BlockBox::new(
            part.bounds.min + at(0, lift, 0),
            part.bounds.max + at(0, lift, 0),
        );
    }
    let openings: Vec<BlockBox> = openings
        .into_iter()
        .map(|b| BlockBox::new(b.min + at(0, lift, 0), b.max + at(0, lift, 0)))
        .collect();
    if in_liquid(terrain, &parts[0].bounds) {
        return Vec::new();
    }
    let mut pieces = vec![Piece {
        bounds: parts[0].bounds,
        facing: Facing::South,
        kind: PieceKind::MineshaftRoom,
        salt: rng.below(SALT_LIMIT),
    }];
    for bounds in openings {
        pieces.push(Piece {
            bounds,
            facing: Facing::South,
            kind: PieceKind::MineshaftOpening,
            salt: 0,
        });
    }
    for part in &parts[1..] {
        if in_liquid(terrain, &part.bounds) {
            continue;
        }
        let salt = rng.below(SALT_LIMIT);
        let mut piece = Piece {
            bounds: part.bounds,
            facing: part.facing,
            kind: PieceKind::MineshaftStairs,
            salt,
        };
        piece.kind = match part.shape {
            Shape::Corridor {
                rails,
                spiders,
                sections,
            } => PieceKind::MineshaftCorridor(corridor_spec(
                terrain, &piece, kind, rails, spiders, sections, rng,
            )),
            Shape::Cross { .. } => PieceKind::MineshaftCross(kind),
            Shape::Stairs | Shape::Room => PieceKind::MineshaftStairs,
        };
        pieces.push(piece);
    }
    pieces
}

/// `isLiquidInStructureBoundingBox` по рельефу: вода над дном океана или реки задевает рамку, расширенную на блок.
fn in_liquid(terrain: &Terrain, b: &BlockBox) -> bool {
    let (low, high) = (b.min.y - 1, b.max.y + 1);
    (b.min.z - 1..=b.max.z + 1).any(|z| {
        (b.min.x - 1..=b.max.x + 1).any(|x| {
            let ground = terrain.surface_height(x, z) + 1;
            low.max(ground) <= high.min(WATER_LEVEL)
        })
    })
}

fn corridor_spec(
    terrain: &Terrain,
    piece: &Piece,
    kind: MineshaftKind,
    rails: bool,
    spiders: bool,
    sections: u8,
    rng: &mut Stream,
) -> CorridorSpec {
    let buried = |x: i32, y: i32, z: i32| {
        let cell = piece.world(x, y, z);
        cell.y <= terrain.surface_height(cell.x, cell.z)
    };
    let mut supports = 0;
    let mut spawner = None;
    for section in 0..sections {
        let z = section_center(section);
        if (0..=2).all(|x| buried(x, CORRIDOR_HEIGHT, z)) {
            supports |= 1 << section;
        }
        if spiders && spawner.is_none() {
            let spot = z - 1 + rng.below(3) as i32;
            if buried(1, 0, spot) {
                spawner = Some(spot as u8);
            }
        }
    }
    CorridorSpec {
        kind,
        sections,
        rails,
        spiders,
        supports,
        spawner,
    }
}

fn section_center(section: u8) -> i32 {
    2 + i32::from(section) * SECTION_LENGTH
}

fn planks(kind: MineshaftKind) -> Block {
    match kind {
        MineshaftKind::Normal => Block::OakPlanks,
        MineshaftKind::Mesa => Block::DarkOakPlanks,
    }
}

fn fence(kind: MineshaftKind) -> Block {
    match kind {
        MineshaftKind::Normal => Block::OakFence,
        MineshaftKind::Mesa => Block::DarkOakFence,
    }
}

pub(super) fn place_opening(w: &mut PieceWriter) {
    let size = w.bounds().size();
    w.fill(bx(0, 0, 0, size.x - 1, size.y - 1, size.z - 1), AIR);
}

pub(super) fn place_room(w: &mut PieceWriter) {
    let size = w.bounds().size();
    let (last_x, last_y, last_z) = (size.x - 1, size.y - 1, size.z - 1);
    w.fill_existing(bx(0, 0, 0, last_x, 0, last_z), Block::Dirt);
    w.fill(bx(0, 1, 0, last_x, 3.min(last_y), last_z), AIR);
    let (width, height, depth) = (size.x as f32, (last_y - 3) as f32, size.z as f32);
    for y in 4..=last_y {
        let rise = (y - 4) as f32 / height;
        for x in 0..=last_x {
            let across = (x as f32 - width / 2.0) / (width * 0.5);
            for z in 0..=last_z {
                let along = (z as f32 - depth / 2.0) / (depth * 0.5);
                if across * across + rise * rise + along * along <= NO_DOME {
                    w.put(x, y, z, AIR);
                }
            }
        }
    }
}

pub(super) fn place_stairs(w: &mut PieceWriter) {
    w.fill(bx(0, 5, 0, 2, 7, 1), AIR);
    w.fill(bx(0, 0, 7, 2, 2, 8), AIR);
    for i in 0..5 {
        let low = 5 - i - i32::from(i < 4);
        w.fill(bx(0, low, 2 + i, 2, 7 - i, 2 + i), AIR);
    }
}

pub(super) fn place_cross(w: &mut PieceWriter, kind: MineshaftKind) {
    let size = w.bounds().size();
    let (last_x, last_y, last_z) = (size.x - 1, size.y - 1, size.z - 1);
    let planks = planks(kind);
    if last_y + 1 > CORRIDOR_HEIGHT {
        let top = CORRIDOR_HEIGHT - 1;
        w.fill(bx(1, 0, 0, last_x - 1, top, last_z), AIR);
        w.fill(bx(0, 0, 1, last_x, top, last_z - 1), AIR);
        w.fill(bx(1, last_y - 2, 0, last_x - 1, last_y, last_z), AIR);
        w.fill(bx(0, last_y - 2, 1, last_x, last_y, last_z - 1), AIR);
        w.fill(bx(1, 3, 1, last_x - 1, 3, last_z - 1), planks);
    } else {
        w.fill(bx(1, 0, 0, last_x - 1, last_y, last_z), AIR);
        w.fill(bx(0, 0, 1, last_x, last_y, last_z - 1), AIR);
    }
    for (x, z) in [
        (1, 1),
        (1, last_z - 1),
        (last_x - 1, 1),
        (last_x - 1, last_z - 1),
    ] {
        if w.get(x, last_y + 1, z) != Block::Air {
            w.fill(bx(x, 0, z, x, last_y, z), planks);
        }
    }
    for x in 0..=last_x {
        for z in 0..=last_z {
            if w.get(x, -1, z) == Block::Air && w.sky_blocked(x, -1, z) {
                w.put(x, -1, z, planks);
            }
        }
    }
}

fn place_support(w: &mut PieceWriter, z: i32, kind: MineshaftKind) {
    let (planks, fence) = (planks(kind), fence(kind));
    for x in [0, 2] {
        w.fill(bx(x, 0, z, x, 1, z), fence);
    }
    if w.chance(0, 0, z, 4) {
        for x in [0, 2] {
            w.put(x, 2, z, planks);
        }
    } else {
        w.fill(bx(0, 2, z, 2, 2, z), planks);
        if w.chance_pct(1, 2, z - 1, TAG_TORCH, 5) {
            w.put(1, 2, z - 1, Cell::torch(Z));
        }
        if w.chance_pct(1, 2, z + 1, TAG_TORCH, 5) {
            w.put(1, 2, z + 1, Cell::torch(NEG_Z));
        }
    }
}

fn place_web(w: &mut PieceWriter, x: i32, y: i32, z: i32, percent: u32) {
    if w.sky_blocked(x, y, z) && w.chance_pct(x, y, z, TAG_WEB_SPOT, percent) {
        w.put(x, y, z, Block::Web);
    }
}

pub(super) fn place_corridor(w: &mut PieceWriter, spec: CorridorSpec) {
    let last = i32::from(spec.sections) * SECTION_LENGTH - 1;
    w.fill(bx(0, 0, 0, 2, 1, last), AIR);
    for z in 0..=last {
        for x in 0..=2 {
            if w.chance_pct(x, 2, z, TAG_ROOF, 80) {
                w.put(x, 2, z, AIR);
            }
        }
    }
    if spec.spiders {
        for y in 0..=1 {
            for z in 0..=last {
                for x in 0..=2 {
                    if w.sky_blocked(x, y, z) && w.chance_pct(x, y, z, TAG_WEB_FILL, 60) {
                        w.put(x, y, z, Block::Web);
                    }
                }
            }
        }
    }
    for section in 0..spec.sections {
        let z = section_center(section);
        if spec.supports & 1 << section != 0 {
            place_support(w, z, spec.kind);
        }
        for x in [0, 2] {
            place_web(w, x, 2, z - 1, 10);
            place_web(w, x, 2, z + 1, 10);
            place_web(w, x, 2, z - 2, 5);
            place_web(w, x, 2, z + 2, 5);
        }
        if w.chance(2, 0, z - 1, 100) {
            w.put(2, 0, z - 1, Cell::chest(NEG_X));
        }
        if w.chance(0, 0, z + 1, 100) {
            w.put(0, 0, z + 1, Cell::chest(X));
        }
    }
    if let Some(z) = spec.spawner {
        w.put(
            1,
            0,
            i32::from(z),
            Cell::with_data(Block::MobSpawner, SPAWNER_SPIDER),
        );
    }
    for z in 0..=last {
        for x in 0..=2 {
            if w.get(x, -1, z) == Block::Air && w.sky_blocked(x, -1, z) {
                w.put(x, -1, z, planks(spec.kind));
            }
        }
    }
    if spec.rails {
        let shape = match w.facing() {
            Facing::North | Facing::South => RAIL_NORTH_SOUTH,
            Facing::East | Facing::West => RAIL_EAST_WEST,
        };
        for z in 0..=last {
            let floor = w.get(1, -1, z);
            if floor.is_solid() && floor.def().opaque {
                let percent = if w.sky_blocked(1, 0, z) { 70 } else { 90 };
                if w.chance_pct(1, 0, z, TAG_RAIL, percent) {
                    w.put(1, 0, z, Cell::with_data(Block::Rail, shape));
                }
            }
        }
    }
}
