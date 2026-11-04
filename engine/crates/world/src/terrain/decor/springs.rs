use super::lakes::relief_block;
use super::*;
use crate::block::FluidKind;

const SALT_SPRINGS: u64 = 100;
const WATER_SPRINGS: u32 = 50;
const LAVA_SPRINGS: u32 = 20;
const WATER_HEIGHT_BASE: u32 = 248;
const LAVA_HEIGHT_BASE: u32 = 240;
const HEIGHT_MIN: u32 = 8;
const SHIFT: u32 = 8;
const SIDES: [IVec3; 4] = [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z];

impl Terrain {
    /// `WorldGenLiquids`: источник в камне с тремя каменными и одной открытой стороной; смещён на +8.
    pub(super) fn grow_springs(&self, key: ColumnPos, column: &mut Column) {
        let origin = key.origin();
        for sz in key.z() - 1..=key.z() {
            for sx in key.x() - 1..=key.x() {
                let state = u64::from(hash(self.seed, sx, sz, SALT_SPRINGS))
                    | u64::from(hash(self.seed, sx, sz, SALT_SPRINGS + 1)) << 32;
                let mut rng = Stream::new(state);
                for attempt in 0..WATER_SPRINGS + LAVA_SPRINGS {
                    let water = attempt < WATER_SPRINGS;
                    let rx = rng.below(16) + SHIFT;
                    let y = if water {
                        let reach = rng.below(WATER_HEIGHT_BASE) + HEIGHT_MIN;
                        rng.below(reach)
                    } else {
                        let outer = rng.below(LAVA_HEIGHT_BASE) + HEIGHT_MIN;
                        let reach = rng.below(outer) + HEIGHT_MIN;
                        rng.below(reach)
                    };
                    let rz = rng.below(16) + SHIFT;
                    let at = IVec3::new(
                        sx * SECTION_SIZE + rx as i32 - origin.x,
                        y as i32,
                        sz * SECTION_SIZE + rz as i32 - origin.z,
                    );
                    let kind = if water {
                        FluidKind::Water
                    } else {
                        FluidKind::Lava
                    };
                    if self.try_spring(key, column, at, kind) {
                        column.springs.push(at);
                    }
                }
            }
        }
    }

    pub(super) fn try_spring(
        &self,
        key: ColumnPos,
        column: &mut Column,
        at: IVec3,
        kind: FluidKind,
    ) -> bool {
        if !(0..SECTION_SIZE).contains(&at.x)
            || !(0..SECTION_SIZE).contains(&at.z)
            || at.y < 1
            || at.y + 1 >= WORLD_HEIGHT
            || column.get(at + IVec3::Y) != Block::Stone
            || column.get(at - IVec3::Y) != Block::Stone
            || !matches!(column.get(at), Block::Stone | Block::Air)
        {
            return false;
        }
        let origin = key.origin();
        let (mut stone, mut air) = (0, 0);
        for side in SIDES {
            let next = at + side;
            let block = if Column::contains(next) {
                column.get(next)
            } else {
                let (x, z) = (origin.x + next.x, origin.z + next.z);
                relief_block(&self.climate(x >> 4, z >> 4), x, next.y, z)
            };
            stone += i32::from(block == Block::Stone);
            air += i32::from(block == Block::Air);
        }
        if stone != 3 || air != 1 {
            return false;
        }
        column.set(at, kind.block());
        true
    }
}
