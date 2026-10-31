use glam::IVec3;
use world::column::WORLD_HEIGHT;
use world::pos::ColumnPos;
use world::section::SECTION_SIZE;
use world::world::World;

pub const COVER_SIZE: i32 = 32;
const CELLS: usize = (COVER_SIZE * COVER_SIZE) as usize;
const HALF: i32 = COVER_SIZE / 2;

fn index(x: i32, z: i32) -> usize {
    ((z & (COVER_SIZE - 1)) * COVER_SIZE + (x & (COVER_SIZE - 1))) as usize
}

pub struct RainCover {
    heights: [u8; CELLS],
    resolved: [Option<(i32, i32)>; CELLS],
    dirty: bool,
}

impl RainCover {
    pub fn new() -> Self {
        Self {
            heights: [0; CELLS],
            resolved: [None; CELLS],
            dirty: true,
        }
    }

    pub fn height(&self, x: i32, z: i32) -> u8 {
        self.heights[index(x, z)]
    }

    pub fn update(&mut self, world: &World, center_x: i32, center_z: i32) {
        for z in center_z - HALF..center_z + HALF {
            for x in center_x - HALF..center_x + HALF {
                let at = index(x, z);
                if self.resolved[at] == Some((x, z)) {
                    continue;
                }
                let (height, resolved) = match column_cover(world, x, z) {
                    Some(height) => (height, Some((x, z))),
                    None => (0, None),
                };
                if self.heights[at] != height {
                    self.heights[at] = height;
                    self.dirty = true;
                }
                self.resolved[at] = resolved;
            }
        }
    }

    pub fn invalidate(&mut self, pos: IVec3) {
        let at = index(pos.x, pos.z);
        if self.resolved[at] == Some((pos.x, pos.z)) {
            self.resolved[at] = None;
        }
    }

    pub fn invalidate_column(&mut self, key: ColumnPos) {
        let origin = key.origin();
        for dz in 0..SECTION_SIZE {
            for dx in 0..SECTION_SIZE {
                self.invalidate(origin + IVec3::new(dx, 0, dz));
            }
        }
    }

    pub fn take_dirty(&mut self) -> Option<&[u8]> {
        std::mem::take(&mut self.dirty).then_some(&self.heights[..])
    }
}

impl Default for RainCover {
    fn default() -> Self {
        Self::new()
    }
}

fn column_cover(world: &World, x: i32, z: i32) -> Option<u8> {
    let column = world.column(ColumnPos::of_block(IVec3::new(x, 0, z)))?;
    let (lx, lz) = (x.rem_euclid(SECTION_SIZE), z.rem_euclid(SECTION_SIZE));
    let top = (0..WORLD_HEIGHT)
        .rev()
        .find(|&y| column.get(IVec3::new(lx, y, lz)).is_opaque());
    Some(top.map_or(0, |y| (y + 1) as u8))
}

#[cfg(test)]
mod tests;
