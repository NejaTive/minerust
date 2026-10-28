use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};
use glam::{IVec2, IVec3};

use crate::block::Block;
use crate::caves::Caves;
use crate::column::{Column, WORLD_HEIGHT};
use crate::section::SECTION_SIZE;
use crate::{plants, trees};

/// Слоёв земли под травой.
const DIRT_DEPTH: i32 = 3;
/// Средняя высота поверхности: запас по ~60 блоков до дна и до верха мира. Это не
/// уровень воды: он чуть ниже, чтобы вода стояла только в низинах.
const SEA_LEVEL: f32 = 64.0;
/// Y верхнего блока воды: всё, что ниже поверхности земли и не выше него, — вода.
pub const WATER_LEVEL: i32 = 62;
/// Поверхность в этих пределах вне пустыни — пляж: сверху песок. Нижняя граница —
/// мелководье у берега, верхняя — полоса в пару блоков над водой.
const BEACH: std::ops::RangeInclusive<i32> = WATER_LEVEL - 2..=WATER_LEVEL + 1;
/// Слоёв песка на пляже; ниже них земля до той же глубины, что под травой.
const BEACH_SAND_DEPTH: i32 = 3;
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
    pub(crate) caves: Caves,
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
            caves: Caves::new(seed),
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

    pub(crate) fn surface(&self, x: i32, z: i32, w: BiomeWeights) -> i32 {
        let (x, z) = (x as f32, z as f32);
        let hills = smoothstep(-0.2, 0.4, self.hilliness.get_noise_2d(x, z));
        let amplitude = w.plains * PLAINS_AMPLITUDE
            + w.desert * DESERT_AMPLITUDE
            + w.forest * FOREST_AMPLITUDE
            + HILLS_AMPLITUDE * hills;
        let y = SEA_LEVEL + amplitude * self.height.get_noise_2d(x, z);
        (y.floor() as i32).clamp(1, WORLD_HEIGHT - 2)
    }

    /// Колонка по её координатам (мировые x, z / 16): земля, пещеры и деревья, в том
    /// числе кроны деревьев соседних колонок, нависающие над этой.
    pub fn generate(&self, key: IVec2) -> Column {
        let mut column = self.generate_trees(key);
        plants::plant(self, key, &mut column);
        column
    }

    /// Колонка с пещерами и деревьями, без растений: тесты мешера и деревьев сверяют кубы.
    pub(crate) fn generate_trees(&self, key: IVec2) -> Column {
        let mut column = self.generate_carved(key);
        trees::plant(self, key, &mut column);
        column
    }

    /// Колонка без деревьев, но с пещерами.
    pub(crate) fn generate_carved(&self, key: IVec2) -> Column {
        let mut column = self.generate_ground(key);
        self.caves.carve(
            key,
            &mut column,
            |x, z| self.surface_height(x, z),
            |x, z| trees::tree_at(self, x, z).is_some(),
        );
        column
    }

    /// Колонка без деревьев и пещер. В пустыне сверху 3–4 блока песка (глубже к центру
    /// пустыни), в остальных биомах трава и 3 слоя земли; ниже камень. У кромки воды
    /// ([`BEACH`]) вместо травы песок, глубже под водой — земля. Над поверхностью ниже
    /// [`WATER_LEVEL`] — вода до него включительно.
    pub(crate) fn generate_ground(&self, key: IVec2) -> Column {
        let mut column = Column::new();
        let origin = key * SECTION_SIZE;
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                let (wx, wz) = (origin.x + x, origin.y + z);
                let weights = self.biome_weights(wx, wz);
                let surface = self.surface(wx, wz, weights);
                let desert = weights.dominant() == Biome::Desert;
                let sand_depth = if desert {
                    (3.0 + weights.desert).round() as i32
                } else if BEACH.contains(&surface) {
                    BEACH_SAND_DEPTH
                } else {
                    0
                };
                // Трава под водой не растёт.
                let top = if surface < WATER_LEVEL {
                    Block::Dirt
                } else {
                    Block::Grass
                };
                for y in 0..=surface {
                    let block = match (desert, surface - y) {
                        (_, d) if d < sand_depth => Block::Sand,
                        (false, 0) => top,
                        (false, d) if d <= DIRT_DEPTH => Block::Dirt,
                        _ => Block::Stone,
                    };
                    column.set(IVec3::new(x, y, z), block);
                }
                for y in surface + 1..=WATER_LEVEL {
                    column.set(IVec3::new(x, y, z), Block::Water);
                }
            }
        }
        column
    }
}

/// 0 до `edge0`, 1 после `edge1`, между ними — плавный переход без излома.
pub(crate) fn smoothstep(edge0: f32, edge1: f32, v: f32) -> f32 {
    let t = ((v - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Колонка, у которой вода стоит и на западном краю, и за ним: для тестов стыков.
#[cfg(test)]
pub(crate) fn lake_key(terrain: &Terrain) -> IVec2 {
    let deep = |x, z| terrain.surface_height(x, z) < WATER_LEVEL - 2;
    (0..4096)
        .flat_map(|z| (-64..64).map(move |i| (i * SECTION_SIZE, z * 3)))
        .find(|&(x, z)| deep(x, z) && deep(x - 1, z))
        .map(|(x, z)| IVec2::new(x / SECTION_SIZE, z.div_euclid(SECTION_SIZE)))
        .expect("no lake on a column border")
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
        for key in [IVec2::new(-3, 7), lake_key(&a)] {
            assert!(same_columns(&a.generate(key), &b.generate(key)));
        }
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

    /// Первый столбец биома `biome` выше пляжей при обходе от (0, 0) по диагонали с шагом 8.
    fn find_biome(terrain: &Terrain, biome: Biome) -> (i32, i32) {
        find_column(terrain, |x, z, w| {
            w.dominant() == biome && terrain.surface(x, z, w) > *BEACH.end()
        })
        .unwrap_or_else(|| panic!("no {biome:?}"))
    }

    /// Первый столбец при обходе по диагонали с шагом 8, для которого `pred` истинен.
    fn find_column(
        terrain: &Terrain,
        pred: impl Fn(i32, i32, BiomeWeights) -> bool,
    ) -> Option<(i32, i32)> {
        (0..8192)
            .map(|i| (i * 8, i * 3))
            .find(|&(x, z)| pred(x, z, terrain.biome_weights(x, z)))
    }

    #[test]
    fn low_ground_is_under_water() {
        let terrain = Terrain::new(1337);
        let (x, z) = find_column(&terrain, |x, z, w| {
            w.dominant() != Biome::Desert && terrain.surface(x, z, w) < *BEACH.start()
        })
        .expect("no deep water");
        // Проверка воды до WATER_LEVEL и воздуха выше — внутри ground_layers.
        let (top, layers) = ground_layers(&terrain, x, z);
        assert!(top < WATER_LEVEL);
        assert_eq!(layers[0], Block::Dirt, "no grass under water");
        assert!(layers.iter().all(|&b| b != Block::Water));
    }

    #[test]
    fn water_fills_column_up_to_level() {
        let terrain = Terrain::new(1);
        let (x, z) = find_column(&terrain, |x, z, w| terrain.surface(x, z, w) == 55)
            .expect("no column with surface 55");
        let column = terrain.generate(IVec2::new(x.div_euclid(16), z.div_euclid(16)));
        let at = |y| column.get(IVec3::new(x.rem_euclid(16), y, z.rem_euclid(16)));
        assert!(at(55).is_solid());
        assert!((56..=WATER_LEVEL).all(|y| at(y) == Block::Water));
        assert!((WATER_LEVEL + 1..WORLD_HEIGHT).all(|y| at(y) == Block::Air));
    }

    #[test]
    fn beaches_are_sand() {
        let terrain = Terrain::new(1337);
        for surface in [WATER_LEVEL - 1, WATER_LEVEL + 1] {
            let (x, z) = find_column(&terrain, |x, z, w| {
                w.dominant() != Biome::Desert && terrain.surface(x, z, w) == surface
            })
            .unwrap_or_else(|| panic!("no beach at {surface}"));
            let (_, layers) = ground_layers(&terrain, x, z);
            let sand = layers.iter().take_while(|&&b| b == Block::Sand).count();
            assert_eq!(sand, BEACH_SAND_DEPTH as usize, "surface {surface}");
            assert_eq!(layers[sand], Block::Dirt);
        }
    }

    /// Блоки земли столбца (x, z) сверху вниз до камня, без деревьев. Над землёй
    /// проверяет воду до `WATER_LEVEL` и воздух выше.
    fn ground_layers(terrain: &Terrain, x: i32, z: i32) -> (i32, Vec<Block>) {
        let key = IVec2::new(x.div_euclid(16), z.div_euclid(16));
        let column = terrain.generate_ground(key);
        let local = |y| IVec3::new(x.rem_euclid(16), y, z.rem_euclid(16));
        let top = terrain.surface_height(x, z);
        for y in top + 1..WORLD_HEIGHT {
            let expected = if y <= WATER_LEVEL {
                Block::Water
            } else {
                Block::Air
            };
            assert_eq!(column.get(local(y)), expected, "y {y}");
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

#[cfg(test)]
mod bench {
    use super::*;

    /// Среднее время `generate` на квадрате колонок 16×16; запуск:
    /// `cargo test --release generate_timing -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn generate_timing() {
        let terrain = Terrain::new(1337);
        let keys: Vec<IVec2> = (-8..8)
            .flat_map(|z| (-8..8).map(move |x| IVec2::new(x, z)))
            .collect();
        let start = std::time::Instant::now();
        for &key in &keys {
            std::hint::black_box(terrain.generate(key));
        }
        let per_column = start.elapsed() / keys.len() as u32;
        println!("generate: {per_column:?} per column");
    }

    /// Подбор точек для режима замера (`?bench`): равнина и лес у воды на сиде 1337
    /// (`BENCH_SEED` в `GameCanvas.vue`). Оценивает круг радиуса 192 блока вокруг каждого узла
    /// сетки и печатает лучшие; запуск:
    /// `cargo test --release bench_points -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn bench_points() {
        let terrain = Terrain::new(1337);
        let (mut plains, mut forest) = (Vec::new(), Vec::new());
        for cz in (-3000..3000).step_by(64) {
            for cx in (-3000..3000).step_by(64) {
                let (mut n, mut pl, mut fo, mut wet) = (0, 0, 0, 0);
                let (mut lo, mut hi) = (i32::MAX, i32::MIN);
                for dz in (-192..=192).step_by(16) {
                    for dx in (-192..=192).step_by(16) {
                        if dx * dx + dz * dz > 192 * 192 {
                            continue;
                        }
                        let (x, z) = (cx + dx, cz + dz);
                        let h = terrain.surface_height(x, z);
                        n += 1;
                        lo = lo.min(h);
                        hi = hi.max(h);
                        wet += i32::from(h < WATER_LEVEL);
                        match terrain.biome_at(x, z) {
                            Biome::Plains => pl += 1,
                            Biome::Forest => fo += 1,
                            Biome::Desert => {}
                        }
                    }
                }
                let f = |k: i32| k as f32 / n as f32;
                if f(wet) < 0.2 && f(pl) > 0.6 {
                    plains.push((hi - lo, cx, cz, f(pl), f(wet)));
                }
                if f(fo) > 0.6 && (0.08..0.25).contains(&f(wet)) {
                    forest.push(((f(wet) - 0.15).abs(), cx, cz, f(fo)));
                }
            }
        }
        plains.sort_by_key(|p| p.0);
        forest.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (x, z) in [(-248, 264), (-1016, 1416)] {
            println!("surface {x} {z}: {}", terrain.surface_height(x, z));
        }
        println!(
            "plains (range, x, z, share, wet): {:?}",
            &plains[..plains.len().min(5)]
        );
        println!(
            "forest (|wet-0.15|, x, z, share): {:?}",
            &forest[..forest.len().min(5)]
        );
    }
}
