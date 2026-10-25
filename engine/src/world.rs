use std::collections::HashMap;

use glam::IVec3;

use crate::block::Block;
use crate::section::{SECTION_SIZE, Section};

/// Размер тестового мира в секциях: 64×32×64 блоков.
const WORLD_SECTIONS: IVec3 = IVec3::new(4, 2, 4);
/// Слоёв земли под травой.
const DIRT_DEPTH: i32 = 3;

/// Конечный мир из секций; ключ карты — координаты секции (мировые блоки / 16).
pub struct World {
    sections: HashMap<IVec3, Section>,
}

impl World {
    /// Холмистый рельеф 4×2×4 секций: трава, под ней 3 слоя земли, ниже камень.
    pub fn generate() -> Self {
        let mut sections = HashMap::new();
        for sy in 0..WORLD_SECTIONS.y {
            for sz in 0..WORLD_SECTIONS.z {
                for sx in 0..WORLD_SECTIONS.x {
                    sections.insert(IVec3::new(sx, sy, sz), Section::new());
                }
            }
        }
        let mut world = Self { sections };

        let size = WORLD_SECTIONS * SECTION_SIZE;
        for z in 0..size.z {
            for x in 0..size.x {
                let surface = surface_height(x, z);
                for y in 0..=surface.min(size.y - 1) {
                    let block = match surface - y {
                        0 => Block::Grass,
                        d if d <= DIRT_DEPTH => Block::Dirt,
                        _ => Block::Stone,
                    };
                    world.set(IVec3::new(x, y, z), block);
                }
            }
        }
        world
    }

    /// Блок по мировым координатам; за пределами мира — воздух.
    pub fn block(&self, pos: IVec3) -> Block {
        let (key, local) = split(pos);
        self.sections
            .get(&key)
            .map_or(Block::Air, |section| section.get(local))
    }

    /// Секции вместе с их координатами (в секциях).
    pub fn sections(&self) -> impl Iterator<Item = (IVec3, &Section)> {
        self.sections.iter().map(|(&key, section)| (key, section))
    }

    fn set(&mut self, pos: IVec3, block: Block) {
        let (key, local) = split(pos);
        if let Some(section) = self.sections.get_mut(&key) {
            section.set(local, block);
        }
    }
}

/// Мировые координаты → (координаты секции, локальные в ней). `div_euclid`, а не `/`,
/// чтобы отрицательные координаты попадали в секцию -1, а не 0.
fn split(pos: IVec3) -> (IVec3, IVec3) {
    let size = IVec3::splat(SECTION_SIZE);
    (pos.div_euclid(size), pos.rem_euclid(size))
}

/// Y верхнего твёрдого блока колонки. Синусы с некратными периодами вместо шума,
/// который появится на шаге 4; значения лежат в 6..=26.
fn surface_height(x: i32, z: i32) -> i32 {
    let (x, z) = (x as f32, z as f32);
    let hills = 6.0 * (x * 0.17).sin() * (z * 0.13).cos();
    let ridges = 4.0 * (x * 0.05 + z * 0.09).sin();
    (16.0 + hills + ridges).floor() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_has_grass_dirt_stone_layers() {
        let world = World::generate();
        let (x, z) = (10, 20);
        let top = surface_height(x, z);
        let at = |y| world.block(IVec3::new(x, y, z));
        assert_eq!(at(top + 1), Block::Air);
        assert_eq!(at(top), Block::Grass);
        for d in 1..=DIRT_DEPTH {
            assert_eq!(at(top - d), Block::Dirt);
        }
        assert_eq!(at(top - DIRT_DEPTH - 1), Block::Stone);
        assert_eq!(at(0), Block::Stone);
    }

    #[test]
    fn surface_stays_in_range() {
        for z in 0..64 {
            for x in 0..64 {
                assert!((6..=26).contains(&surface_height(x, z)));
            }
        }
    }

    #[test]
    fn no_faces_between_sections() {
        use crate::mesher::mesh_section;

        let world = World::generate();
        let meshed: usize = world
            .sections()
            .map(|(key, section)| {
                let origin = key * SECTION_SIZE;
                mesh_section(section, origin, |local| world.block(origin + local))
                    .indices
                    .len()
                    / 6
            })
            .sum();

        // Те же грани, посчитанные по миру целиком, без деления на секции.
        let dirs = [
            IVec3::X,
            IVec3::NEG_X,
            IVec3::Y,
            IVec3::NEG_Y,
            IVec3::Z,
            IVec3::NEG_Z,
        ];
        let mut expected = 0;
        for y in 0..32 {
            for z in 0..64 {
                for x in 0..64 {
                    let pos = IVec3::new(x, y, z);
                    if world.block(pos).is_solid() {
                        expected += dirs
                            .iter()
                            .filter(|&&d| !world.block(pos + d).is_solid())
                            .count();
                    }
                }
            }
        }
        assert_eq!(meshed, expected);
    }

    #[test]
    fn outside_world_is_air() {
        let world = World::generate();
        assert_eq!(world.block(IVec3::new(-1, 0, 0)), Block::Air);
        assert_eq!(world.block(IVec3::new(0, 0, 64)), Block::Air);
        assert_eq!(world.block(IVec3::new(0, -1, 0)), Block::Air);
    }
}
