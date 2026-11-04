use super::*;

const SALT_PUMPKINS: u64 = 90;
const PUMPKIN_ONE_IN: u32 = 32;
const PUMPKIN_TRIES: u32 = 64;
const PUMPKIN_SPREAD: u32 = 8;
const PUMPKIN_VERTICAL_SPREAD: u32 = 4;
const PUMPKIN_SKY_REACH: u32 = 32;
const SHIFT: i32 = 8;

impl Terrain {
    pub(super) fn pumpkin_patch(&self, sx: i32, sz: i32) -> Option<Stream> {
        let state = u64::from(hash(self.seed, sx, sz, SALT_PUMPKINS))
            | u64::from(hash(self.seed, sx, sz, SALT_PUMPKINS + 1)) << 32;
        let mut rng = Stream::new(state);
        (rng.below(PUMPKIN_ONE_IN) == 0).then_some(rng)
    }

    /// `WorldGenPumpkin`: смещён на +8, поэтому колонку задевают грядки четырёх колонок.
    pub(super) fn grow_pumpkins(&self, key: ColumnPos, column: &mut Column) {
        let origin = key.origin();
        for sz in key.z() - 1..=key.z() {
            for sx in key.x() - 1..=key.x() {
                let Some(mut rng) = self.pumpkin_patch(sx, sz) else {
                    continue;
                };
                let x = sx * SECTION_SIZE + SHIFT + rng.below(16) as i32;
                let z = sz * SECTION_SIZE + SHIFT + rng.below(16) as i32;
                let surface = self.climate(x >> 4, z >> 4).surface[ColumnClimate::index(x, z)];
                let y = rng.below(surface as u32 + PUMPKIN_SKY_REACH) as i32;
                for _ in 0..PUMPKIN_TRIES {
                    let at = IVec3::new(
                        x + rng.spread(PUMPKIN_SPREAD) - origin.x,
                        y + rng.spread(PUMPKIN_VERTICAL_SPREAD),
                        z + rng.spread(PUMPKIN_SPREAD) - origin.z,
                    );
                    let facing = rng.below(4) as u8;
                    if at.y > 0
                        && Column::contains(at)
                        && column.get(at) == Block::Air
                        && column.get(at - IVec3::Y) == Block::Grass
                    {
                        column.set_with_data(at, Block::Pumpkin, facing);
                    }
                }
            }
        }
    }
}
