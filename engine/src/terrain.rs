use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};
use glam::{IVec2, IVec3};

use crate::block::Block;
use crate::column::{Column, WORLD_HEIGHT};
use crate::section::SECTION_SIZE;

/// Слоёв земли под травой.
const DIRT_DEPTH: i32 = 3;
/// Средняя высота поверхности: запас по ~60 блоков до дна и до верха мира.
const SEA_LEVEL: f32 = 64.0;
/// Размах рельефа на равнине и добавка к нему в холмах, блоков. FBm почти не выходит
/// за ±0.7, так что поверхность остаётся примерно в 40..90.
const PLAINS_AMPLITUDE: f32 = 6.0;
const HILLS_AMPLITUDE: f32 = 34.0;

/// Генератор рельефа: высота поверхности — FBm-шум, его размах — второй, более
/// плавный шум. Так на одной карте есть и равнины, и холмы.
pub struct Terrain {
    height: FastNoiseLite,
    hilliness: FastNoiseLite,
}

impl Terrain {
    pub fn new(seed: i32) -> Self {
        let mut height = FastNoiseLite::with_seed(seed);
        height.set_noise_type(Some(NoiseType::OpenSimplex2));
        height.set_fractal_type(Some(FractalType::FBm));
        height.set_fractal_octaves(Some(5));
        // Период крупных холмов ~200 блоков; мелкие октавы дают неровности в пару блоков.
        height.set_frequency(Some(0.005));

        // Свой сид, иначе размах коррелировал бы с самой высотой.
        let mut hilliness = FastNoiseLite::with_seed(seed.wrapping_add(1));
        hilliness.set_noise_type(Some(NoiseType::OpenSimplex2));
        // Зоны равнин и холмов шириной в несколько сотен блоков.
        hilliness.set_frequency(Some(0.0015));

        Self { height, hilliness }
    }

    /// Y верхнего твёрдого блока в мировых координатах, всегда в 1..WORLD_HEIGHT-1.
    pub fn surface_height(&self, x: i32, z: i32) -> i32 {
        let (x, z) = (x as f32, z as f32);
        let hills = smoothstep(-0.2, 0.4, self.hilliness.get_noise_2d(x, z));
        let amplitude = PLAINS_AMPLITUDE + HILLS_AMPLITUDE * hills;
        let y = SEA_LEVEL + amplitude * self.height.get_noise_2d(x, z);
        (y.floor() as i32).clamp(1, WORLD_HEIGHT - 2)
    }

    /// Колонка по её координатам (мировые x, z / 16): трава, 3 слоя земли, камень.
    pub fn generate(&self, key: IVec2) -> Column {
        let mut column = Column::new();
        let origin = key * SECTION_SIZE;
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                let surface = self.surface_height(origin.x + x, origin.y + z);
                for y in 0..=surface {
                    let block = match surface - y {
                        0 => Block::Grass,
                        d if d <= DIRT_DEPTH => Block::Dirt,
                        _ => Block::Stone,
                    };
                    column.set(IVec3::new(x, y, z), block);
                }
            }
        }
        column
    }
}

/// 0 до `edge0`, 1 после `edge1`, между ними — плавный переход без излома.
fn smoothstep(edge0: f32, edge1: f32, v: f32) -> f32 {
    let t = ((v - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn same_columns(a: &Column, b: &Column) -> bool {
        (0..WORLD_HEIGHT).all(|y| {
            (0..SECTION_SIZE).all(|z| {
                (0..SECTION_SIZE).all(|x| {
                    let p = IVec3::new(x, y, z);
                    a.get(p) == b.get(p)
                })
            })
        })
    }

    #[test]
    fn same_seed_same_world() {
        let (a, b) = (Terrain::new(42), Terrain::new(42));
        for (x, z) in [(0, 0), (-1000, 37), (123_456, -98_765)] {
            assert_eq!(a.surface_height(x, z), b.surface_height(x, z));
        }
        let key = IVec2::new(-3, 7);
        assert!(same_columns(&a.generate(key), &b.generate(key)));
    }

    #[test]
    fn other_seed_other_world() {
        let (a, b) = (Terrain::new(42), Terrain::new(43));
        let differs =
            (0..64).filter(|&i| a.surface_height(i * 7, i * 3) != b.surface_height(i * 7, i * 3));
        assert!(differs.count() > 32);
    }

    #[test]
    fn surface_stays_in_range() {
        let terrain = Terrain::new(1337);
        let (mut min, mut max) = (i32::MAX, i32::MIN);
        for z in (-2048..2048).step_by(8) {
            for x in (-2048..2048).step_by(8) {
                let h = terrain.surface_height(x, z);
                assert!((1..WORLD_HEIGHT - 1).contains(&h), "{h} at {x}, {z}");
                min = min.min(h);
                max = max.max(h);
            }
        }
        // Рельеф не плоский и держится около заданного коридора.
        assert!(max - min > 30, "{min}..{max}");
        assert!(min > 20 && max < 110, "{min}..{max}");
    }

    #[test]
    fn column_has_grass_dirt_stone_layers() {
        let terrain = Terrain::new(1337);
        let key = IVec2::new(-2, 5);
        let column = terrain.generate(key);
        let (x, z) = (4, 11);
        let top = terrain.surface_height(key.x * 16 + x, key.y * 16 + z);
        let at = |y| column.get(IVec3::new(x, y, z));
        for y in top + 1..WORLD_HEIGHT {
            assert_eq!(at(y), Block::Air);
        }
        assert_eq!(at(top), Block::Grass);
        for d in 1..=DIRT_DEPTH {
            assert_eq!(at(top - d), Block::Dirt);
        }
        for y in 0..top - DIRT_DEPTH {
            assert_eq!(at(y), Block::Stone);
        }
    }
}
