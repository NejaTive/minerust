use glam::IVec3;

use super::*;

const FENCE_REACH_UNITS: u8 = 24;
const GATE_REACH_UNITS: u8 = 24;

pub const CAULDRON_LEVELS: u8 = 3;
const CAULDRON_WATER_BASE: u8 = 6;
const CAULDRON_WATER_STEP: u8 = 3;

pub const POT_PLANTS: [Block; 15] = [
    Block::Poppy,
    Block::BlueOrchid,
    Block::Allium,
    Block::Houstonia,
    Block::RedTulip,
    Block::OrangeTulip,
    Block::WhiteTulip,
    Block::PinkTulip,
    Block::OxeyeDaisy,
    Block::Dandelion,
    Block::RedMushroom,
    Block::BrownMushroom,
    Block::DeadBush,
    Block::Fern,
    Block::Cactus,
];

const HORIZONTAL: [IVec3; 4] = [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z];

fn is_full_cube(block: Block) -> bool {
    block.is_opaque() && block.shape() == Shape::Cube && block.is_solid()
}

impl Block {
    pub fn is_fence(self) -> bool {
        self.box_kind() == BoxKind::Fence
    }

    pub fn is_gate(self) -> bool {
        self.box_kind() == BoxKind::Gate
    }

    pub fn is_pane(self) -> bool {
        self.box_kind() == BoxKind::Pane
    }

    pub fn is_door(self) -> bool {
        self.box_kind() == BoxKind::Door
    }

    pub fn is_rail(self) -> bool {
        self.box_kind() == BoxKind::Rail
    }

    pub fn is_ladder(self) -> bool {
        self.box_kind() == BoxKind::Ladder
    }

    pub fn is_trapdoor(self) -> bool {
        self.box_kind() == BoxKind::Trapdoor
    }

    /// Правая кнопка открывает и закрывает; железная дверь ждёт механизмов.
    pub fn opens_by_hand(self) -> bool {
        matches!(self, Block::Chest | Block::FlowerPot | Block::Bed)
            || self.is_trapdoor()
            || self.is_gate()
            || (self.is_door() && !matches!(self, Block::IronDoor | Block::IronDoorUpper))
    }

    pub fn door_upper(self) -> Option<Block> {
        DOORS
            .iter()
            .find_map(|&(lower, upper)| (lower == self).then_some(upper))
    }

    pub fn door_lower(self) -> Option<Block> {
        DOORS
            .iter()
            .find_map(|&(lower, upper)| (upper == self).then_some(lower))
    }

    /// Нижняя половина двери того же вида, что и `self`, в любой из двух половин.
    pub fn same_door(self, other: Block) -> bool {
        self.is_door()
            && (self == other
                || self.door_upper() == Some(other)
                || self.door_lower() == Some(other))
    }

    /// Соединение забора с соседом в направлении `toward`, как `BlockFence.canConnectTo`.
    pub fn fence_joins(self, other: Block, other_data: u8, toward: IVec3) -> bool {
        if other.is_fence() {
            return true;
        }
        if other.is_gate() {
            let facing = other.facing_dir(other_data & GATE_FACING_MASK);
            return facing.abs() != toward.abs();
        }
        is_full_cube(other) && !matches!(other, Block::Barrier | Block::Pumpkin | Block::Melon)
    }

    pub fn pane_joins(self, other: Block) -> bool {
        other.is_pane() || other == Block::Glass || (is_full_cube(other) && other != Block::Barrier)
    }

    pub fn is_open(self, data: u8) -> bool {
        self.is_gate() && data & GATE_OPEN != 0
            || self.data_kind() == DataKind::Door && data & DOOR_OPEN != 0
            || self.is_trapdoor() && data & TRAPDOOR_OPEN != 0
    }

    /// Поворот, открытость и петля двери по обеим половинам, как `getActualState`.
    pub fn door_parts(self, data: u8, around: impl Fn(IVec3) -> (Block, u8)) -> (u8, bool, bool) {
        let (lower_data, upper_data) = if let Some(lower) = self.door_lower() {
            let (block, below) = around(IVec3::NEG_Y);
            (if block == lower { below } else { 0 }, data)
        } else {
            let (block, above) = around(IVec3::Y);
            (
                data,
                if Some(block) == self.door_upper() {
                    above
                } else {
                    0
                },
            )
        };
        (
            lower_data & GATE_FACING_MASK,
            lower_data & DOOR_OPEN != 0,
            upper_data & DOOR_HINGE_RIGHT != 0,
        )
    }

    pub(super) fn fence_boxes(
        self,
        around: &impl Fn(IVec3) -> (Block, u8),
        collide: bool,
    ) -> BoxList {
        let height = if collide { FENCE_REACH_UNITS } else { 16 };
        let mut list = BoxList::of(&[VoxelBox::new([6, 0, 6], [10, height, 10])]);
        for dir in HORIZONTAL {
            let (other, other_data) = around(dir);
            if !self.fence_joins(other, other_data, dir) {
                continue;
            }
            let turns = (facing_code(dir) + 3) % 4;
            let arms: &[VoxelBox] = if collide {
                &[VoxelBox::new([10, 0, 6], [16, FENCE_REACH_UNITS, 10])]
            } else {
                &[
                    VoxelBox::new([10, 6, 7], [16, 9, 9]),
                    VoxelBox::new([10, 12, 7], [16, 15, 9]),
                ]
            };
            for arm in arms {
                list.push(arm.turned(turns));
            }
        }
        list
    }

    pub(super) fn pane_boxes(self, around: &impl Fn(IVec3) -> (Block, u8)) -> BoxList {
        let mut list = BoxList::of(&[VoxelBox::new([7, 0, 7], [9, 16, 9])]);
        let joined: Vec<IVec3> = HORIZONTAL
            .into_iter()
            .filter(|&dir| self.pane_joins(around(dir).0))
            .collect();
        let arms = if joined.is_empty() {
            HORIZONTAL.to_vec()
        } else {
            joined
        };
        for dir in arms {
            let turns = (facing_code(dir) + 3) % 4;
            list.push(VoxelBox::new([9, 0, 7], [16, 16, 9]).turned(turns));
        }
        list
    }

    pub(super) fn gate_boxes(self, data: u8, collide: bool) -> BoxList {
        let open = data & GATE_OPEN != 0;
        let turns = ((data & GATE_FACING_MASK) + 2) % 4;
        let base: &[VoxelBox] = match (collide, open) {
            (true, true) => &[],
            (true, false) => &[VoxelBox::new([0, 0, 6], [16, GATE_REACH_UNITS, 10])],
            (false, true) => &[
                VoxelBox::new([0, 5, 7], [2, 16, 9]),
                VoxelBox::new([14, 5, 7], [16, 16, 9]),
                VoxelBox::new([0, 6, 9], [2, 9, 13]),
                VoxelBox::new([0, 12, 9], [2, 15, 13]),
                VoxelBox::new([0, 6, 13], [2, 15, 15]),
                VoxelBox::new([14, 6, 9], [16, 9, 13]),
                VoxelBox::new([14, 12, 9], [16, 15, 13]),
                VoxelBox::new([14, 6, 13], [16, 15, 15]),
            ],
            (false, false) => &[
                VoxelBox::new([0, 5, 7], [2, 16, 9]),
                VoxelBox::new([14, 5, 7], [16, 16, 9]),
                VoxelBox::new([2, 6, 7], [14, 9, 9]),
                VoxelBox::new([2, 12, 7], [14, 15, 9]),
                VoxelBox::new([6, 9, 7], [10, 12, 9]),
            ],
        };
        let mut list = BoxList::EMPTY;
        for b in base {
            list.push(b.turned(turns));
        }
        list
    }

    pub(super) fn door_boxes(self, data: u8, around: &impl Fn(IVec3) -> (Block, u8)) -> BoxList {
        let (facing, open, right) = self.door_parts(data, around);
        let slab = match (open, right) {
            (false, _) => VoxelBox::new([0, 0, 0], [3, 16, 16]),
            (true, true) => VoxelBox::new([0, 0, 13], [16, 16, 16]),
            (true, false) => VoxelBox::new([0, 0, 0], [16, 16, 3]),
        };
        BoxList::of(&[slab.turned((facing + 3) % 4)])
    }

    pub(super) fn rail_boxes(data: u8) -> BoxList {
        let height = if (RAIL_ASCENDING_EAST..=RAIL_ASCENDING_SOUTH).contains(&data) {
            RAIL_SLOPE_UNITS
        } else {
            RAIL_UNITS
        };
        BoxList::of(&[VoxelBox::new([0, 0, 0], [16, height, 16])])
    }

    /// Коробки сундука: в системе взгляда спереди `a` растёт вправо, `c` — вглубь от лица.
    pub(super) fn chest_boxes(data: u8) -> BoxList {
        let half = data >> chest::HALF_SHIFT & chest::HALF_MASK;
        let pieces: [([u8; 2], [u8; 2], [u8; 2]); 3] = if half == chest::HALF_NONE {
            [
                ([1, 15], [0, 10], [1, 15]),
                ([1, 15], [9, 14], [1, 15]),
                ([7, 9], [7, 11], [0, 1]),
            ]
        } else {
            let (lo, hi) = if half == chest::HALF_LEFT {
                (0, 16)
            } else {
                (16, 32)
            };
            let clip = |a: [u8; 2]| [a[0].max(lo) - lo, a[1].min(hi).saturating_sub(lo)];
            [
                (clip([1, 31]), [0, 10], [1, 15]),
                (clip([1, 31]), [9, 14], [1, 15]),
                (clip([15, 17]), [7, 11], [0, 1]),
            ]
        };
        let mut list = BoxList::EMPTY;
        for (a, y, c) in pieces {
            if a[0] >= a[1] {
                continue;
            }
            let b = VoxelBox::new([16 - a[1], y[0], c[0]], [16 - a[0], y[1], c[1]]);
            list.push(b.turned(data & chest::FACING_MASK));
        }
        list
    }

    /// Кровать: в системе, где изголовье смотрит на север; ножки — у дальнего от стыка края.
    pub(super) fn bed_boxes(data: u8) -> BoxList {
        let b = VoxelBox::new;
        let leg = BED_LEG_UNITS;
        let outer = if bed::is_head(data) { 0 } else { 16 - leg };
        let mut list = BoxList::of(&[b([0, leg, 0], [16, BED_UNITS, 16])]);
        for x in [0, 16 - leg] {
            list.push(b([x, 0, outer], [x + leg, leg, outer + leg]));
        }
        let turns = data & bed::FACING_MASK;
        list.map(|b| b.turned(turns))
    }

    pub(super) fn trapdoor_boxes(data: u8) -> BoxList {
        let panel = if data & TRAPDOOR_OPEN != 0 {
            VoxelBox::new([0, 0, 16 - TRAPDOOR_UNITS], [16, 16, 16])
                .turned(data & TRAPDOOR_FACING_MASK)
        } else if data & TRAPDOOR_TOP != 0 {
            VoxelBox::new([0, 16 - TRAPDOOR_UNITS, 0], [16, 16, 16])
        } else {
            VoxelBox::new([0, 0, 0], [16, TRAPDOOR_UNITS, 16])
        };
        BoxList::of(&[panel])
    }

    pub(super) fn brewing_stand_boxes() -> BoxList {
        let b = VoxelBox::new;
        BoxList::of(&[
            b([7, 0, 7], [9, 14, 9]),
            b([9, 0, 5], [15, 2, 11]),
            b([2, 0, 1], [8, 2, 7]),
            b([2, 0, 9], [8, 2, 15]),
        ])
    }

    pub(super) fn cauldron_boxes() -> BoxList {
        let b = VoxelBox::new;
        BoxList::of(&[
            b([0, 3, 0], [16, 4, 16]),
            b([0, 4, 0], [2, 16, 16]),
            b([14, 4, 0], [16, 16, 16]),
            b([2, 4, 0], [14, 16, 2]),
            b([2, 4, 14], [14, 16, 16]),
            b([0, 0, 0], [4, 3, 4]),
            b([12, 0, 0], [16, 3, 4]),
            b([0, 0, 12], [4, 3, 16]),
            b([12, 0, 12], [16, 3, 16]),
        ])
    }

    /// Высота воды в котле в 1/16 блока; `None` — пусто.
    pub fn cauldron_water_units(data: u8) -> Option<u8> {
        let level = data.min(CAULDRON_LEVELS);
        (level > 0).then_some(CAULDRON_WATER_BASE + CAULDRON_WATER_STEP * level)
    }

    /// Плитка грани `box_index`-й коробки: у котла стенки, дно и ножки смотрят разными плитками.
    pub fn box_tile(
        self,
        box_index: usize,
        facing: Facing,
        normal: IVec3,
        data: u8,
    ) -> Option<Tile> {
        match self {
            Block::Cauldron => Some(match (box_index, facing) {
                (_, Facing::Bottom) => Tile::CauldronBottom,
                (0, Facing::Top) => Tile::CauldronInner,
                (1..=4, Facing::Top) => Tile::CauldronTop,
                (1..=4, Facing::Side) => {
                    let outward = [IVec3::NEG_X, IVec3::X, IVec3::NEG_Z, IVec3::Z][box_index - 1];
                    if normal == -outward {
                        Tile::CauldronInner
                    } else {
                        Tile::CauldronSide
                    }
                }
                _ => Tile::CauldronSide,
            }),
            Block::FlowerPot if facing == Facing::Top => Some(Tile::Dirt),
            Block::Bed => Some(match (box_index, facing) {
                (0, Facing::Top) => Tile::bed(bed::color_of(data), bed::is_head(data), true),
                (0, Facing::Side) => Tile::bed(bed::color_of(data), bed::is_head(data), false),
                _ => Tile::PlanksOak,
            }),
            Block::BrewingStand => Some(if box_index == 0 {
                Tile::BrewingStand
            } else {
                Tile::BrewingStandBase
            }),
            _ => self.face_tile(facing, normal, data),
        }
    }

    /// Растение в горшке по `data`; 0 — пусто.
    pub fn pot_plant(data: u8) -> Option<Block> {
        POT_PLANTS.get(usize::from(data).checked_sub(1)?).copied()
    }

    pub fn pot_code(plant: Block) -> Option<u8> {
        POT_PLANTS
            .iter()
            .position(|&p| p == plant)
            .map(|i| i as u8 + 1)
    }

    /// Коробки столкновений: у заборов и калиток они выше блока (1.5), у открытой калитки пусты.
    pub fn collision_boxes(self, data: u8, around: impl Fn(IVec3) -> (Block, u8)) -> BoxList {
        match self.box_kind() {
            BoxKind::Fence => self.fence_boxes(&around, true),
            BoxKind::Gate => self.gate_boxes(data, true),
            BoxKind::Bed => BoxList::of(&[VoxelBox::new([0, 0, 0], [16, BED_UNITS, 16])]),
            BoxKind::Plate => BoxList::EMPTY,
            _ => self.boxes(data, around),
        }
    }
}
