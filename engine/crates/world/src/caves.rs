use fastnoise_lite::{FastNoiseLite, NoiseType};
use glam::{IVec2, IVec3};

use crate::block::Block;
use crate::column::{Column, WORLD_HEIGHT};
use crate::section::SECTION_SIZE;
use crate::terrain::{WATER_LEVEL, smoothstep};

/// Шаг сетки узлов шума по каждой оси, блоков. Узлы лежат в мировых координатах,
/// кратных шагу, поэтому соседние колонки берут одни и те же узлы — швов на стыках нет.
const CELL: i32 = 4;
const NODES_XZ: usize = (SECTION_SIZE / CELL + 1) as usize;
const NODES_Y: usize = (WORLD_HEIGHT / CELL + 1) as usize;
/// y ≤ этого всегда камень: дно мира не протыкается.
const FLOOR: i32 = 3;
/// Верхние блоки под поверхностью (`surface - ROOF + 1..=surface`), которые
/// вырезаются только во входах: без них каждая пещера под самой травой проваливала бы
/// поверхность, а дерево могло бы остаться без опоры.
const ROOF: i32 = 3;
/// Под дном воды и пляжа нетронутыми остаются ещё столько блоков: вода статична, и над
/// воздухом она бы висела.
const WET_ROOF: i32 = 4;

/// «Сыр»: залы там, где шум выше порога. Период шума ~50 блоков, пятна выше 0.6 —
/// залы ~10–20 блоков в поперечнике.
const CHEESE_FREQUENCY: f32 = 0.02;
const CHEESE_THRESHOLD: f32 = 0.6;
/// Добавка к порогу залов у самой поверхности; при ней (выше 1) залов там нет.
const CHEESE_FADE: f32 = 0.6;
/// «Спагетти»: туннель там, где оба шума близки к нулю, — вдоль пересечения двух
/// поверхностей нулевого уровня.
const TUNNEL_FREQUENCY: f32 = 0.015;
/// Первый шум сжат по вертикали: его нулевые поверхности ложатся почти горизонтально,
/// и пересечение со вторым (изотропным) идёт скорее вдоль земли, чем вниз. Если
/// наоборот растянуть шумы по вертикали, нулевые поверхности встанут стенами, а их
/// пересечения станут вертикальными шахтами.
const TUNNEL_Y_SCALE: f32 = 2.0;
/// Полуширины туннеля в единицах шумов. Сжатый шум по y меняется вдвое быстрее, его
/// порог вдвое больше — сечение выходит примерно круглым.
const TUNNEL_A: f32 = 0.2;
const TUNNEL_B: f32 = 0.1;
/// На этой глубине под поверхностью (блоков) пещеры достигают полной плотности; выше
/// туннели тоньше, а залы реже.
const DEPTH_FADE: f32 = 24.0;
/// Над дном мира пещеры сходят на нет за столько блоков: иначе залы упирались бы в
/// нетронутые слои у дна и пол был бы плоским.
const BOTTOM_FADE: f32 = 12.0;
/// Входы — редкие пятна 2D-шума: в них пещеры доходят до поверхности в полную силу.
const ENTRANCE_FREQUENCY: f32 = 0.01;
const ENTRANCE_EDGES: (f32, f32) = (0.6, 0.8);
/// Добавка к плотности в центре входа. Туннели почти горизонтальны и поверхность лишь
/// задевают, оставляя канавки; с добавкой во входе открываются провалы в залы.
const ENTRANCE_BOOST: f32 = 0.4;

/// Значения шумов в точке: зал, первый и второй шумы туннелей.
type Sample = [f32; 3];

/// Пещеры: залы на «сырном» 3D-шуме и туннели на паре «спагетти»-шумов. Решение
/// зависит только от сида и мировых координат, так что колонки считаются независимо.
pub struct Caves {
    cheese: FastNoiseLite,
    tunnel_a: FastNoiseLite,
    tunnel_b: FastNoiseLite,
    entrance: FastNoiseLite,
}

/// Что разрешено вырезать в столбце.
#[derive(Clone, Copy, Default)]
struct Limits {
    surface: i32,
    /// Верхний y, который ещё можно вырезать; не выше `FLOOR` — ничего.
    top: i32,
    /// Сила входа 0..=1; больше нуля — столбец во входе.
    entrance: f32,
}

impl Caves {
    pub fn new(seed: i32) -> Self {
        // Смещения сидов не пересекаются с шумами рельефа и биомов (+1..+3).
        let noise = |offset, frequency| {
            let mut noise = FastNoiseLite::with_seed(seed.wrapping_add(offset));
            noise.set_noise_type(Some(NoiseType::OpenSimplex2));
            noise.set_frequency(Some(frequency));
            noise
        };
        Self {
            cheese: noise(10, CHEESE_FREQUENCY),
            tunnel_a: noise(11, TUNNEL_FREQUENCY),
            tunnel_b: noise(12, TUNNEL_FREQUENCY),
            entrance: noise(13, ENTRANCE_FREQUENCY),
        }
    }

    #[cfg(test)]
    /// Вырезать ли блок по мировым координатам, без учёта ограничений (дно, вода,
    /// входы) и ослабления у поверхности — как на большой глубине.
    pub fn is_cave(&self, pos: IVec3) -> bool {
        hollow(self.sample(pos), 1.0)
    }

    /// Применяет пещеры к колонке `key` со всеми ограничениями; `surface(x, z)` и
    /// `has_tree(x, z)` — по мировым координатам столбца. Вырезает только траву, землю,
    /// песок и камень: воду, брёвна и листву не трогает.
    pub fn carve(
        &self,
        key: IVec2,
        column: &mut Column,
        surface: impl Fn(i32, i32) -> i32,
        has_tree: impl Fn(i32, i32) -> bool,
    ) {
        let origin = key * SECTION_SIZE;
        // Поверхность с полосой в один столбец вокруг: вода соседа тоже ограничивает.
        const SIDE: i32 = SECTION_SIZE + 2;
        let mut heights = [0; (SIDE * SIDE) as usize];
        for z in 0..SIDE {
            for x in 0..SIDE {
                heights[(z * SIDE + x) as usize] = surface(origin.x + x - 1, origin.y + z - 1);
            }
        }
        let cached =
            |x: i32, z: i32| heights[((z - origin.y + 1) * SIDE + x - origin.x + 1) as usize];

        let mut limits = [[Limits::default(); SECTION_SIZE as usize]; SECTION_SIZE as usize];
        let mut top = FLOOR;
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                let l = self.limits(origin.x + x, origin.y + z, cached, &has_tree);
                top = top.max(l.top);
                limits[z as usize][x as usize] = l;
            }
        }
        if top <= FLOOR {
            return;
        }

        // Узлы считаются один раз на колонку и только в нужном диапазоне высот: ниже
        // первой вырезаемой ячейки и выше ячейки самой высокой разрешённой точки они не
        // нужны.
        let (low, high) = ((FLOOR + 1) / CELL, top / CELL + 1);
        let mut nodes = [[[0.0; 3]; NODES_XZ * NODES_XZ]; NODES_Y];
        for j in low..=high {
            for k in 0..NODES_XZ as i32 {
                for i in 0..NODES_XZ as i32 {
                    let pos = IVec3::new(origin.x, 0, origin.y) + IVec3::new(i, j, k) * CELL;
                    nodes[j as usize][k as usize * NODES_XZ + i as usize] = self.node(pos);
                }
            }
        }

        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                let l = limits[z as usize][x as usize];
                // Шумы столбца в каждом слое узлов, дальше по y — линейно между слоями:
                // так на блок приходится одна интерполяция вместо трилинейной.
                let (i, k) = ((x / CELL) as usize, (z / CELL) as usize);
                let (tx, tz) = (fraction(x % CELL), fraction(z % CELL));
                let mut layers = [[0.0; 3]; NODES_Y];
                for j in low..=l.top / CELL + 1 {
                    let layer = &nodes[j as usize];
                    let at = |di: usize, dk: usize| layer[(k + dk) * NODES_XZ + i + di];
                    layers[j as usize] = bilinear([at(0, 0), at(1, 0), at(0, 1), at(1, 1)], tx, tz);
                }
                for y in FLOOR + 1..=l.top {
                    let local = IVec3::new(x, y, z);
                    if !carvable(column.get(local)) {
                        continue;
                    }
                    let j = (y / CELL) as usize;
                    let sample = lerp(layers[j], layers[j + 1], fraction(y % CELL));
                    if hollow(sample, l.density(y)) {
                        column.set(local, Block::Air);
                    }
                }
            }
        }
    }

    /// Ограничения столбца (x, z).
    fn limits(
        &self,
        x: i32,
        z: i32,
        surface: impl Fn(i32, i32) -> i32,
        has_tree: impl Fn(i32, i32) -> bool,
    ) -> Limits {
        let h = surface(x, z);
        let entrance = self.entrance(x, z);
        let mut top = if entrance > 0.0 && !has_tree(x, z) {
            h
        } else {
            h - ROOF
        };
        // Сосед по горизонтали тоже: иначе вода стояла бы стеной у стенки пещеры.
        let wet_floor = (-1..=1)
            .flat_map(|dz| (-1..=1).map(move |dx| (x + dx, z + dz)))
            .map(|(x, z)| surface(x, z))
            .filter(|&s| s <= WATER_LEVEL + 1)
            .min();
        if let Some(floor) = wet_floor {
            top = top.min(floor - WET_ROOF - 1);
        }
        Limits {
            surface: h,
            top,
            entrance,
        }
    }

    fn entrance(&self, x: i32, z: i32) -> f32 {
        let n = self.entrance.get_noise_2d(x as f32, z as f32);
        smoothstep(ENTRANCE_EDGES.0, ENTRANCE_EDGES.1, n)
    }

    /// Шумы в узле сетки, без интерполяции.
    fn node(&self, pos: IVec3) -> Sample {
        let p = pos.as_vec3();
        [
            self.cheese.get_noise_3d(p.x, p.y, p.z),
            self.tunnel_a.get_noise_3d(p.x, p.y * TUNNEL_Y_SCALE, p.z),
            self.tunnel_b.get_noise_3d(p.x, p.y, p.z),
        ]
    }

    /// Шумы в любой точке: билинейно по x, z в двух слоях узлов вокруг неё, потом
    /// линейно по y. Те же узлы и тот же порядок арифметики, что в [`Caves::carve`],
    /// поэтому результат совпадает побитно.
    #[cfg(test)]
    fn sample(&self, pos: IVec3) -> Sample {
        let cell = pos.div_euclid(IVec3::splat(CELL));
        let t = pos - cell * CELL;
        let layer = |dy| {
            let at = |dx, dz| self.node((cell + IVec3::new(dx, dy, dz)) * CELL);
            bilinear(
                [at(0, 0), at(1, 0), at(0, 1), at(1, 1)],
                fraction(t.x),
                fraction(t.z),
            )
        };
        lerp(layer(0), layer(1), fraction(t.y))
    }
}

impl Limits {
    /// Плотность пещер на высоте `y`: растёт с глубиной до 1, во входах — полная и с
    /// добавкой, у дна мира снова падает.
    fn density(&self, y: i32) -> f32 {
        let depth = smoothstep(0.0, DEPTH_FADE, (self.surface - y) as f32);
        let bottom = smoothstep(FLOOR as f32, FLOOR as f32 + BOTTOM_FADE, y as f32);
        (depth.max(self.entrance) + self.entrance * ENTRANCE_BOOST) * bottom
    }
}

/// Пусто ли при таких шумах и плотности.
fn hollow([cheese, a, b]: Sample, density: f32) -> bool {
    let hall = cheese > CHEESE_THRESHOLD + (1.0 - density) * CHEESE_FADE;
    hall || (a.abs() < TUNNEL_A * density && b.abs() < TUNNEL_B * density)
}

fn carvable(block: Block) -> bool {
    // Траву тоже: иначе вход в пещеру был бы закрыт слоем дёрна.
    matches!(
        block,
        Block::Stone | Block::Dirt | Block::Sand | Block::Grass
    )
}

/// Положение внутри ячейки по оси, 0..1.
fn fraction(offset: i32) -> f32 {
    offset as f32 / CELL as f32
}

fn lerp(a: Sample, b: Sample, t: f32) -> Sample {
    std::array::from_fn(|n| a[n] + (b[n] - a[n]) * t)
}

/// Углы слоя по порядку: (x, z), (x + 1, z), (x, z + 1), (x + 1, z + 1).
fn bilinear([c00, c10, c01, c11]: [Sample; 4], tx: f32, tz: f32) -> Sample {
    lerp(lerp(c00, c10, tx), lerp(c01, c11, tx), tz)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::{Terrain, lake_key};
    use crate::trees::tree_at;

    fn local(pos: IVec3) -> IVec3 {
        IVec3::new(pos.x.rem_euclid(16), pos.y, pos.z.rem_euclid(16))
    }

    fn keys(center: IVec2, radius: i32) -> impl Iterator<Item = IVec2> {
        (-radius..=radius)
            .flat_map(move |z| (-radius..=radius).map(move |x| center + IVec2::new(x, z)))
    }

    /// Колонка без деревьев до и после пещер.
    fn ground_and_carved(terrain: &Terrain, key: IVec2) -> (Column, Column) {
        (terrain.generate_ground(key), terrain.generate_carved(key))
    }

    /// Мировые координаты блоков колонки `key`.
    fn blocks(key: IVec2) -> impl Iterator<Item = IVec3> {
        let origin = key * 16;
        (0..WORLD_HEIGHT).flat_map(move |y| {
            (0..16)
                .flat_map(move |z| (0..16).map(move |x| IVec3::new(origin.x + x, y, origin.y + z)))
        })
    }

    #[test]
    fn same_seed_same_caves_other_seed_other() {
        let (a, b, other) = (Caves::new(42), Caves::new(42), Caves::new(43));
        let points: Vec<IVec3> = (0..4000)
            .map(|i| IVec3::new(i * 7 - 9000, 4 + i % 120, i * -13 + 500))
            .collect();
        assert!(points.iter().all(|&p| a.is_cave(p) == b.is_cave(p)));
        let differs = points
            .iter()
            .filter(|&&p| a.is_cave(p) != other.is_cave(p))
            .count();
        let caves = points.iter().filter(|&&p| a.is_cave(p)).count();
        assert!(caves > 100, "{caves} caves");
        assert!(differs > caves / 2, "{differs} of {caves}");

        let terrain = Terrain::new(42);
        for key in [IVec2::new(-3, 7), IVec2::new(100, -40)] {
            let (_, x) = ground_and_carved(&terrain, key);
            let (_, y) = ground_and_carved(&Terrain::new(42), key);
            assert!(blocks(key).all(|p| x.get(local(p)) == y.get(local(p))));
        }
    }

    #[test]
    fn interpolation_hits_nodes_and_stays_between_them() {
        let caves = Caves::new(7);
        for i in -20..20 {
            let node = IVec3::new(i * 4, 8 + (i & 15) * 4, i * -12);
            assert_eq!(caves.sample(node), caves.node(node));
        }
        for i in 0..2000 {
            let pos = IVec3::new(i * 5 - 3000, i % 128, i * 3 - 77);
            let cell = pos.div_euclid(IVec3::splat(CELL));
            let corners: [Sample; 8] = std::array::from_fn(|c| {
                let offset = IVec3::new((c & 1) as i32, (c >> 1 & 1) as i32, (c >> 2) as i32);
                caves.node((cell + offset) * CELL)
            });
            let value = caves.sample(pos);
            for n in 0..3 {
                let (min, max) = corners.iter().fold((f32::MAX, f32::MIN), |(lo, hi), c| {
                    (lo.min(c[n]), hi.max(c[n]))
                });
                assert!(
                    (min - 1e-6..=max + 1e-6).contains(&value[n]),
                    "{pos}: {value:?}"
                );
            }
        }
    }

    #[test]
    fn carve_matches_direct_evaluation_across_borders() {
        // Сетка в колонке и прямое вычисление по мировым координатам должны совпасть
        // везде, в том числе в столбцах x = 15 и x = 0 соседних колонок.
        let terrain = Terrain::new(1337);
        let caves = &terrain.caves;
        let surface = |x, z| terrain.surface_height(x, z);
        let has_tree = |x, z| tree_at(&terrain, x, z).is_some();
        let mut carved_total = 0;
        for key in [
            IVec2::new(2, -5),
            IVec2::new(3, -5),
            IVec2::new(-1, 0),
            IVec2::new(0, 0),
        ] {
            let (ground, carved) = ground_and_carved(&terrain, key);
            for p in blocks(key) {
                let l = caves.limits(p.x, p.z, surface, has_tree);
                let expected = p.y > FLOOR
                    && p.y <= l.top
                    && carvable(ground.get(local(p)))
                    && hollow(caves.sample(p), l.density(p.y));
                let got = carved.get(local(p)) != ground.get(local(p));
                assert_eq!(got, expected, "{p}");
                carved_total += usize::from(got);
            }
        }
        assert!(carved_total > 1000, "{carved_total} carved");
    }

    #[test]
    fn carved_share_of_underground_is_moderate() {
        for seed in [1337, 42] {
            let terrain = Terrain::new(seed);
            let (mut solid, mut hollow) = (0, 0);
            for key in keys(IVec2::new(5, -3), 3) {
                let (ground, carved) = ground_and_carved(&terrain, key);
                for p in blocks(key).filter(|p| p.y > FLOOR) {
                    let p = local(p);
                    if carvable(ground.get(p)) {
                        solid += 1;
                        hollow += usize::from(carved.get(p) == Block::Air);
                    }
                }
            }
            let share = hollow as f32 / solid as f32;
            assert!((0.05..=0.20).contains(&share), "seed {seed}: {share}");
        }
    }

    #[test]
    fn bottom_layers_stay_stone() {
        let terrain = Terrain::new(1337);
        for key in keys(IVec2::new(-4, 9), 2) {
            let column = terrain.generate(key);
            for p in blocks(key).filter(|p| p.y <= FLOOR) {
                assert_eq!(column.get(local(p)), Block::Stone, "{p}");
            }
        }
    }

    #[test]
    fn no_air_under_water() {
        let terrain = Terrain::new(1337);
        let lake = lake_key(&terrain);
        let mut water = 0;
        for key in keys(lake, 2) {
            let column = terrain.generate(key);
            for p in blocks(key).map(local) {
                if column.get(p) != Block::Water {
                    continue;
                }
                water += 1;
                // Дно и ещё WET_ROOF блоков под ним целы.
                let floor = (0..p.y)
                    .rev()
                    .find(|&y| column.get(p.with_y(y)) != Block::Water);
                let floor = floor.expect("water down to the bottom");
                for y in floor - WET_ROOF..=floor {
                    assert!(column.get(p.with_y(y)).is_solid(), "{key} {p} y {y}");
                }
            }
        }
        assert!(water > 1000, "{water} water blocks");
    }

    #[test]
    fn trees_stand_on_ground() {
        let terrain = Terrain::new(1337);
        let mut trees = 0;
        for key in keys(IVec2::new(0, 0), 6) {
            let column = terrain.generate(key);
            let origin = key * 16;
            for z in origin.y..origin.y + 16 {
                for x in origin.x..origin.x + 16 {
                    let Some(tree) = tree_at(&terrain, x, z) else {
                        continue;
                    };
                    trees += 1;
                    for dy in 1..=ROOF {
                        let p = local(tree.base - IVec3::Y * dy);
                        assert!(column.get(p).is_solid(), "tree at {x}, {z}, dy {dy}");
                    }
                }
            }
        }
        assert!(trees > 10, "{trees} trees");
    }

    #[test]
    fn tree_closes_entrance_roof() {
        let terrain = Terrain::new(1337);
        let caves = &terrain.caves;
        let (x, z) = (0..100_000)
            .map(|i| (i * 7, i * -3))
            .find(|&(x, z)| {
                caves.entrance(x, z) > 0.5 && terrain.surface_height(x, z) > WATER_LEVEL + 8
            })
            .expect("no dry entrance");
        let surface = |x, z| terrain.surface_height(x, z);
        let h = surface(x, z);
        assert_eq!(caves.limits(x, z, surface, |_, _| false).top, h);
        assert_eq!(caves.limits(x, z, surface, |_, _| true).top, h - ROOF);
    }

    #[test]
    fn roof_is_opened_only_in_entrances() {
        let terrain = Terrain::new(1337);
        let caves = &terrain.caves;
        let mut openings = 0;
        for key in keys(IVec2::new(0, 0), 8) {
            let (ground, carved) = ground_and_carved(&terrain, key);
            let origin = key * 16;
            for z in origin.y..origin.y + 16 {
                for x in origin.x..origin.x + 16 {
                    let h = terrain.surface_height(x, z);
                    let opened = (h - ROOF + 1..=h).any(|y| {
                        let p = local(IVec3::new(x, y, z));
                        carved.get(p) != ground.get(p)
                    });
                    if opened {
                        assert!(caves.entrance(x, z) > 0.0, "{x}, {z}");
                        openings += 1;
                    }
                }
            }
        }
        // Входы редкие, но на 17×17 колонок встречаются.
        assert!(openings > 0, "no entrances");
        assert!(openings < 17 * 17 * 256 / 50, "{openings} opened columns");
    }
}
