use super::*;
use crate::block::{SPAWNER_SKELETON, SPAWNER_SPIDER, SPAWNER_ZOMBIE};
use crate::chest;

const SALT_DUNGEONS: u64 = 410;
const DUNGEON_ATTEMPTS: u32 = 8;
const DUNGEON_MIN_OPENINGS: u32 = 1;
const DUNGEON_MAX_OPENINGS: u32 = 5;
const CHEST_ROOMS: u32 = 2;
const CHEST_TRIES: u32 = 3;
const MOSSY_FLOOR_CHANCE: (u32, u32) = (3, 4);
const CENTER_MIN: i32 = 4;
const CENTER_SPAN: u32 = 8;
const SPAWNER_MOBS: [u8; 4] = [
    SPAWNER_SKELETON,
    SPAWNER_ZOMBIE,
    SPAWNER_ZOMBIE,
    SPAWNER_SPIDER,
];
const SIDES: [IVec3; 4] = [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z];

impl Terrain {
    /// `WorldGenDungeons`: комната целиком в своей колонке, центр не ближе 4 к её краю.
    pub(crate) fn grow_dungeons(&self, key: ColumnPos, column: &mut Column) {
        if self.world_type != WorldType::Standard {
            return;
        }
        let state = u64::from(hash(self.seed, key.x(), key.z(), SALT_DUNGEONS))
            | u64::from(hash(self.seed, key.x(), key.z(), SALT_DUNGEONS + 1)) << 32;
        let mut rng = Stream::new(state);
        for _ in 0..DUNGEON_ATTEMPTS {
            let half = IVec3::new(rng.below(2) as i32 + 2, 0, rng.below(2) as i32 + 2);
            let center = IVec3::new(
                CENTER_MIN + rng.below(CENTER_SPAN) as i32,
                rng.below(WORLD_HEIGHT as u32) as i32,
                CENTER_MIN + rng.below(CENTER_SPAN) as i32,
            );
            if fits(column, center, half) {
                build(&mut rng, column, center, half);
            }
        }
    }
}

fn frame(center: IVec3, half: IVec3) -> impl Iterator<Item = IVec3> {
    (-half.x - 1..=half.x + 1).flat_map(move |dx| {
        (-1..=4).flat_map(move |dy| {
            (-half.z - 1..=half.z + 1).map(move |dz| center + IVec3::new(dx, dy, dz))
        })
    })
}

fn on_perimeter(center: IVec3, half: IVec3, at: IVec3) -> bool {
    let d = at - center;
    d.x.abs() == half.x + 1 || d.z.abs() == half.z + 1
}

fn fits(column: &Column, center: IVec3, half: IVec3) -> bool {
    if center.y < 1 || center.y + 4 >= WORLD_HEIGHT {
        return false;
    }
    let mut openings = 0;
    for at in frame(center, half) {
        let dy = at.y - center.y;
        if (dy == -1 || dy == 4) && !column.get(at).is_solid() {
            return false;
        }
        if dy == 0
            && on_perimeter(center, half, at)
            && column.get(at) == Block::Air
            && column.get(at + IVec3::Y) == Block::Air
        {
            openings += 1;
        }
    }
    (DUNGEON_MIN_OPENINGS..=DUNGEON_MAX_OPENINGS).contains(&openings)
}

fn build(rng: &mut Stream, column: &mut Column, center: IVec3, half: IVec3) {
    for at in frame(center, half) {
        let d = at - center;
        let inside = d.x.abs() <= half.x && d.z.abs() <= half.z && (0..=2).contains(&d.y);
        if inside {
            if column.get(at) != Block::Chest {
                column.set(at, Block::Air);
            }
        } else if column.get(at).is_solid() {
            let mossy = d.y == -1 && rng.chance(MOSSY_FLOOR_CHANCE.0, MOSSY_FLOOR_CHANCE.1);
            let block = if mossy {
                Block::MossyCobblestone
            } else {
                Block::Cobblestone
            };
            column.set(at, block);
        }
    }
    for _ in 0..CHEST_ROOMS {
        for _ in 0..CHEST_TRIES {
            let at = IVec3::new(
                center.x + rng.below(half.x as u32 * 2 + 1) as i32 - half.x,
                center.y,
                center.z + rng.below(half.z as u32 * 2 + 1) as i32 - half.z,
            );
            if column.get(at) != Block::Air {
                continue;
            }
            let walls: Vec<IVec3> = SIDES
                .into_iter()
                .filter(|&side| column.get(at + side).is_solid())
                .collect();
            if let [wall] = walls[..] {
                column.set_with_data(at, Block::Chest, chest::data(-wall, chest::HALF_NONE));
                break;
            }
        }
    }
    let mob = SPAWNER_MOBS[rng.below(SPAWNER_MOBS.len() as u32) as usize];
    column.set_with_data(center, Block::MobSpawner, mob);
}

#[cfg(test)]
mod tests;
