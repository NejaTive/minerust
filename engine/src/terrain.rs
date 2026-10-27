use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};
use glam::{IVec2, IVec3};

use crate::block::Block;
use crate::column::{Column, WORLD_HEIGHT};
use crate::section::SECTION_SIZE;
use crate::trees;

/// Слоёв земли под травой.
const DIRT_DEPTH: i32 = 3;
/// Средняя высота поверхности: запас по ~60 блоков до дна и до верха мира.
const SEA_LEVEL: f32 = 64.0;
/// Размах рельефа по биомам и добавка к нему в холмах, блоков. FBm почти не выходит
/// за ±0.7, так что поверхность остаётся примерно в 30..100.
const PLAINS_AMPLITUDE: f32 = 6.0;
const DESERT_AMPLITUDE: f32 = 4.0;
const FOREST_AMPLITUDE: f32 = 14.0;
const HILLS_AMPLITUDE: f32 = 34.0;

/// Биом столбца — тот, чей вес в нём наибольший.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Biome {
    Plains,
    Desert,
    Forest,
}

/// Доли биомов в столбце, в сумме 1. Рельеф смешивается по ним, поэтому на границах
/// биомов высота меняется плавно.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BiomeWeights {
    pub plains: f32,
    pub desert: f32,
    pub forest: f32,
}

impl BiomeWeights {
    pub fn dominant(self) -> Biome {
        if self.desert > self.plains.max(self.forest) {
            Biome::Desert
        } else if self.forest > self.plains {
            Biome::Forest
        } else {
            Biome::Plains
        }
    }
}

/// Генератор рельефа: высота поверхности — FBm-шум, его размах — второй, более
/// плавный шум и биом. Биом задают ещё два плавных шума: температура и влажность.
pub struct Terrain {
    seed: i32,
    height: FastNoiseLite,
    hilliness: FastNoiseLite,
    temperature: FastNoiseLite,
    humidity: FastNoiseLite,
}

impl Terrain {
    pub fn new(seed: i32) -> Self {
        let mut height = FastNoiseLite::with_seed(seed);
        height.set_noise_type(Some(NoiseType::OpenSimplex2));
        height.set_fractal_type(Some(FractalType::FBm));
        height.set_fractal_octaves(Some(5));
        // Период крупных холмов ~200 блоков; мелкие октавы дают неровности в пару блоков.
        height.set_frequency(Some(0.005));

        // Свои сиды у каждого шума, иначе они коррелировали бы друг с другом.
        let mut hilliness = FastNoiseLite::with_seed(seed.wrapping_add(1));
        hilliness.set_noise_type(Some(NoiseType::OpenSimplex2));
        // Зоны равнин и холмов шириной в несколько сотен блоков.
        hilliness.set_frequency(Some(0.0015));

        // Биомы шириной в пару сотен блоков: за несколько сотен встречаются все три.
        let climate = |offset| {
            let mut noise = FastNoiseLite::with_seed(seed.wrapping_add(offset));
            noise.set_noise_type(Some(NoiseType::OpenSimplex2));
            noise.set_frequency(Some(0.003));
            noise
        };

        Self {
            seed,
            height,
            hilliness,
            temperature: climate(2),
            humidity: climate(3),
        }
    }

    pub fn seed(&self) -> i32 {
        self.seed
    }

    /// Доли биомов: пустыня — жарко и сухо, лес — влажно, равнины — остальное.
    /// Пустыне нужна влажность ниже −0.1, лесу — выше 0.1, так что они не смешиваются
    /// между собой и всегда разделены полосой равнин.
    pub fn biome_weights(&self, x: i32, z: i32) -> BiomeWeights {
        let (x, z) = (x as f32, z as f32);
        let temperature = self.temperature.get_noise_2d(x, z);
        let humidity = self.humidity.get_noise_2d(x, z);
        let desert = smoothstep(0.05, 0.25, temperature) * smoothstep(0.1, 0.3, -humidity);
        let forest = smoothstep(0.1, 0.3, humidity);
        BiomeWeights {
            plains: 1.0 - desert - forest,
            desert,
            forest,
        }
    }

    pub fn biome_at(&self, x: i32, z: i32) -> Biome {
        self.biome_weights(x, z).dominant()
    }

    /// Y верхнего блока земли (без деревьев) в мировых координатах, всегда в
    /// 1..WORLD_HEIGHT-1.
    pub fn surface_height(&self, x: i32, z: i32) -> i32 {
        self.surface(x, z, self.biome_weights(x, z))
    }

    fn surface(&self, x: i32, z: i32, w: BiomeWeights) -> i32 {
        let (x, z) = (x as f32, z as f32);
        let hills = smoothstep(-0.2, 0.4, self.hilliness.get_noise_2d(x, z));
        let amplitude = w.plains * PLAINS_AMPLITUDE
            + w.desert * DESERT_AMPLITUDE
            + w.forest * FOREST_AMPLITUDE
            + HILLS_AMPLITUDE * hills;
        let y = SEA_LEVEL + amplitude * self.height.get_noise_2d(x, z);
        (y.floor() as i32).clamp(1, WORLD_HEIGHT - 2)
    }

    /// Колонка по её координатам (мировые x, z / 16): земля и деревья, в том числе
    /// кроны деревьев соседних колонок, нависающие над этой.
    pub fn generate(&self, key: IVec2) -> Column {
        let mut column = self.generate_ground(key);
        trees::plant(self, key, &mut column);
        column
    }

    /// Колонка без деревьев. В пустыне сверху 3–4 блока песка (глубже к центру
    /// пустыни), в остальных биомах трава и 3 слоя земли; ниже камень.
    pub(crate) fn generate_ground(&self, key: IVec2) -> Column {
        let mut column = Column::new();
        let origin = key * SECTION_SIZE;
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                let (wx, wz) = (origin.x + x, origin.y + z);
                let weights = self.biome_weights(wx, wz);
                let surface = self.surface(wx, wz, weights);
                let sand_depth = (3.0 + weights.desert).round() as i32;
                let desert = weights.dominant() == Biome::Desert;
                for y in 0..=surface {
                    let block = match (desert, surface - y) {
                        (true, d) if d < sand_depth => Block::Sand,
                        (false, 0) => Block::Grass,
                        (false, d) if d <= DIRT_DEPTH => Block::Dirt,
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

    /// Первый столбец биома `biome` при обходе от (0, 0) по диагонали с шагом 8.
    fn find_biome(terrain: &Terrain, biome: Biome) -> (i32, i32) {
        (0..4096)
            .map(|i| (i * 8, i * 3))
            .find(|&(x, z)| terrain.biome_weights(x, z).dominant() == biome)
            .unwrap_or_else(|| panic!("no {biome:?}"))
    }

    /// Блоки земли столбца (x, z) сверху вниз до камня, без деревьев.
    fn ground_layers(terrain: &Terrain, x: i32, z: i32) -> (i32, Vec<Block>) {
        let key = IVec2::new(x.div_euclid(16), z.div_euclid(16));
        let column = terrain.generate_ground(key);
        let local = |y| IVec3::new(x.rem_euclid(16), y, z.rem_euclid(16));
        let top = terrain.surface_height(x, z);
        for y in top + 1..WORLD_HEIGHT {
            assert_eq!(column.get(local(y)), Block::Air);
        }
        (top, (0..=top).rev().map(|y| column.get(local(y))).collect())
    }

    #[test]
    fn grass_dirt_stone_outside_desert() {
        let terrain = Terrain::new(1337);
        for biome in [Biome::Plains, Biome::Forest] {
            let (x, z) = find_biome(&terrain, biome);
            let (_, layers) = ground_layers(&terrain, x, z);
            assert_eq!(layers[0], Block::Grass);
            assert!(
                layers[1..=DIRT_DEPTH as usize]
                    .iter()
                    .all(|&b| b == Block::Dirt)
            );
            assert!(
                layers[DIRT_DEPTH as usize + 1..]
                    .iter()
                    .all(|&b| b == Block::Stone)
            );
        }
    }

    #[test]
    fn sand_over_stone_in_desert() {
        let terrain = Terrain::new(1337);
        let (x, z) = find_biome(&terrain, Biome::Desert);
        let (_, layers) = ground_layers(&terrain, x, z);
        let sand = layers.iter().take_while(|&&b| b == Block::Sand).count();
        assert!((3..=4).contains(&sand), "{sand} sand blocks");
        assert!(layers[sand..].iter().all(|&b| b == Block::Stone));
    }

    #[test]
    fn biomes_are_deterministic() {
        let (a, b) = (Terrain::new(42), Terrain::new(42));
        for (x, z) in [(0, 0), (-1000, 37), (123_456, -98_765)] {
            assert_eq!(a.biome_weights(x, z), b.biome_weights(x, z));
        }
    }

    #[test]
    fn weights_sum_to_one() {
        let terrain = Terrain::new(7);
        for i in 0..1000 {
            let w = terrain.biome_weights(i * 13, i * -7);
            assert!(
                [w.plains, w.desert, w.forest]
                    .iter()
                    .all(|&v| (0.0..=1.0).contains(&v))
            );
            assert!((w.plains + w.desert + w.forest - 1.0).abs() < 1e-5, "{w:?}");
        }
    }

    #[test]
    fn all_biomes_within_2048_blocks() {
        for seed in [1337, 42, 7] {
            let terrain = Terrain::new(seed);
            let mut counts = [0; 3];
            for z in (0..2048).step_by(16) {
                for x in (0..2048).step_by(16) {
                    counts[terrain.biome_at(x, z) as usize] += 1;
                }
            }
            // Каждый биом — не меньше 5% площади.
            let total: i32 = counts.iter().sum();
            assert!(
                counts.iter().all(|&c| c * 20 > total),
                "seed {seed}: {counts:?}"
            );
        }
    }

    #[test]
    fn biome_borders_have_no_cliffs() {
        // Перепад высоты между соседними столбцами: в переходной зоне (смешанные веса)
        // и внутри леса — самого холмистого биома.
        let terrain = Terrain::new(1337);
        let pure = |w: BiomeWeights| [w.plains, w.desert, w.forest].contains(&1.0);
        let (mut border, mut forest, mut border_pairs) = (0, 0, 0);
        for row in (-2048..2048).step_by(32) {
            for along_x in [true, false] {
                let mut prev: Option<(i32, BiomeWeights)> = None;
                for i in -2048..2048 {
                    let (x, z) = if along_x { (i, row) } else { (row, i) };
                    let w = terrain.biome_weights(x, z);
                    let h = terrain.surface_height(x, z);
                    if let Some((ph, pw)) = prev {
                        let step = (h - ph).abs();
                        if !pure(w) || !pure(pw) {
                            border = border.max(step);
                            border_pairs += 1;
                        } else if w.forest == 1.0 && pw.forest == 1.0 {
                            forest = forest.max(step);
                        }
                    }
                    prev = Some((h, w));
                }
            }
        }
        assert!(border_pairs > 1000, "{border_pairs} border pairs");
        assert!(
            border <= forest,
            "border step {border}, forest step {forest}"
        );
    }
}
