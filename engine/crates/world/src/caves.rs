use fastnoise_lite::{FastNoiseLite, NoiseType};
use glam::IVec3;

use crate::block::Block;
use crate::column::{Column, WORLD_HEIGHT};
use crate::pos::ColumnPos;
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

    /// Применяет пещеры к колонке `key` со всеми ограничениями; `surface(x, z)` и
    /// `has_tree(x, z)` — по мировым координатам столбца. Вырезает только траву, землю,
    /// песок и камень: воду, брёвна и листву не трогает.
    pub fn carve(
        &self,
        key: ColumnPos,
        column: &mut Column,
        surface: impl Fn(i32, i32) -> i32,
        has_tree: impl Fn(i32, i32) -> bool,
    ) {
        let origin = key.origin();
        // Поверхность с полосой в один столбец вокруг: вода соседа тоже ограничивает.
        const SIDE: i32 = SECTION_SIZE + 2;
        let mut heights = [0; (SIDE * SIDE) as usize];
        for z in 0..SIDE {
            for x in 0..SIDE {
                heights[(z * SIDE + x) as usize] = surface(origin.x + x - 1, origin.z + z - 1);
            }
        }
        let cached =
            |x: i32, z: i32| heights[((z - origin.z + 1) * SIDE + x - origin.x + 1) as usize];

        let mut limits = [[Limits::default(); SECTION_SIZE as usize]; SECTION_SIZE as usize];
        let mut top = FLOOR;
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                let l = self.limits(origin.x + x, origin.z + z, cached, &has_tree);
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
                    let pos = origin + IVec3::new(i, j, k) * CELL;
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
mod tests;
