use glam::IVec3;

use crate::mesh::{POSITION_UNITS, Vertex, day_shade, quantize_shade};
use world::block::{Block, Facing, Shape, WATER_SURFACE};
use world::column::{COLUMN_SECTIONS, Column, WORLD_HEIGHT};
use world::light::{ColumnLight, LightMap, MAX_LIGHT, block_level, sky_level};
use world::pos::{ColumnPos, SectionPos};
use world::section::{SECTION_SIZE, Section};
use world::tile::{LAYER_COUNT, TILE_SIZE, TORCH_PIXELS, TORCH_TOP_ROW, Tile};
use world::world::World;

// Слой хранится в вершине одним байтом.
const _: () = assert!(LAYER_COUNT <= 256);

pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
}

struct Face {
    /// Направление на соседа, которого грань касается.
    normal: IVec3,
    facing: Facing,
    /// Множитель яркости, как в Minecraft без сглаженного света.
    shade: f32,
    /// Углы в единичном кубе блока, против часовой стрелки при взгляде снаружи
    /// (этого ждут `FrontFace::Ccw` и `cull_mode: Back`). У боковых граней порядок
    /// всегда низ-лево, низ-право, верх-право, верх-лево — на этом держится `CORNER_UVS`.
    corners: [[i32; 3]; 4],
}

/// UV углов грани в порядке `Face::corners`: у боковых граней v = 0 оказывается
/// сверху, и кайма травы смотрит вверх.
const CORNER_UVS: [[u8; 2]; 4] = [[0, 16], [16, 16], [16, 0], [0, 0]];

const FACES: [Face; 6] = [
    Face {
        normal: IVec3::Y,
        facing: Facing::Top,
        shade: 1.0,
        corners: [[0, 1, 1], [1, 1, 1], [1, 1, 0], [0, 1, 0]],
    },
    Face {
        normal: IVec3::NEG_Y,
        facing: Facing::Bottom,
        shade: 0.5,
        corners: [[0, 0, 0], [1, 0, 0], [1, 0, 1], [0, 0, 1]],
    },
    Face {
        normal: IVec3::X,
        facing: Facing::Side,
        shade: 0.6,
        corners: [[1, 0, 1], [1, 0, 0], [1, 1, 0], [1, 1, 1]],
    },
    Face {
        normal: IVec3::NEG_X,
        facing: Facing::Side,
        shade: 0.6,
        corners: [[0, 0, 0], [0, 0, 1], [0, 1, 1], [0, 1, 0]],
    },
    Face {
        normal: IVec3::Z,
        facing: Facing::Side,
        shade: 0.8,
        corners: [[0, 0, 1], [1, 0, 1], [1, 1, 1], [0, 1, 1]],
    },
    Face {
        normal: IVec3::NEG_Z,
        facing: Facing::Side,
        shade: 0.8,
        corners: [[1, 0, 0], [0, 0, 0], [0, 1, 0], [1, 1, 0]],
    },
];

/// Верх воды в шагах позиции вершины ([`WATER_SURFACE`] × 16; точно, потому что
/// 0.875 = 14/16).
const WATER_SURFACE_UNITS: i32 = 14;
const _: () = assert!(WATER_SURFACE_UNITS as f32 == WATER_SURFACE * POSITION_UNITS as f32);

/// Настройки построения меша, общие для всего мира.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshOptions {
    /// Затенять углы и стыки блоков (ambient occlusion по вершинам).
    pub ambient_occlusion: bool,
}

impl Default for MeshOptions {
    fn default() -> Self {
        Self {
            ambient_occlusion: true,
        }
    }
}

/// Яркость вершины по уровню [`vertex_ao`]. Не 0.25-шаг по 0fps: вместе с `shade`
/// нижней грани (0.5) угол стал бы почти чёрным и съел текстуру. Нижняя ступень
/// темнее остальных (0.2 против 0.15), чтобы внутренний угол из двух стен читался
/// отдельно от основания одной стены.
const AO_BRIGHTNESS: [f32; 4] = [0.5, 0.7, 0.85, 1.0];

/// Уровень затенения вершины по схеме 0fps, 0 — темнее всего, 3 — без затенения.
/// `side1`, `side2` — непрозрачные ли блоки, примыкающие к вершине по рёбрам грани,
/// `corner` — блок по диагонали; все три — в слое перед гранью. Если обе стороны
/// непрозрачные, угловой блок не виден и уровень 0 независимо от него.
pub fn vertex_ao(side1: bool, side2: bool, corner: bool) -> u8 {
    if side1 && side2 {
        0
    } else {
        3 - (u8::from(side1) + u8::from(side2) + u8::from(corner))
    }
}

/// Сторона куба блоков секции с полем в один блок со всех сторон.
const PADDED: i32 = SECTION_SIZE + 2;

/// Блоки секции и всех её соседей в пределах −1..=16 по каждой оси. AO смотрит на
/// 12 блоков на грань, и брать их из массива намного дешевле, чем из мира (поиск
/// колонки в `HashMap`).
struct Neighborhood([Block; (PADDED * PADDED * PADDED) as usize]);

impl Neighborhood {
    /// Индекс локальных координат секции, −1..=16 по каждой оси.
    fn index(local: IVec3) -> usize {
        let p = local + IVec3::ONE;
        (p.x + PADDED * p.z + PADDED * PADDED * p.y) as usize
    }

    /// Сдвиг индекса при шаге на `offset`: так соседи блока берутся без пересчёта координат.
    fn stride(offset: IVec3) -> isize {
        (offset.x + PADDED * offset.z + PADDED * PADDED * offset.y) as isize
    }

    fn at(&self, index: usize, offset: isize) -> Block {
        self.0[index.wrapping_add_signed(offset)]
    }

    fn opaque(&self, index: usize, offset: isize) -> bool {
        self.at(index, offset).is_opaque()
    }
}

/// Байты света секции и соседей ([`world::light::pack_light`]), как [`Neighborhood`]
/// для блоков.
struct Lights([u8; (PADDED * PADDED * PADDED) as usize]);

impl Lights {
    /// Солнечный уровень и уровень света блоков клетки.
    fn at(&self, index: usize, offset: isize) -> (u32, u32) {
        let packed = self.0[index.wrapping_add_signed(offset)];
        (u32::from(sky_level(packed)), u32::from(block_level(packed)))
    }
}

/// Для каждой вершины грани — сдвиги индекса в [`Neighborhood`] к трём блокам перед
/// ней: два боковых и угловой. Те же клетки дают и плавный свет вершины.
type CornerStrides = [[isize; 3]; 4];

fn corner_strides(face: &Face) -> CornerStrides {
    let normal_axis = face.normal.abs().max_position();
    let [a, b] = [1, 2].map(|i| (normal_axis + i) % 3);
    face.corners.map(|corner| {
        // От центра блока к вершине: ±1 по каждой оси.
        let toward = IVec3::from(corner) * 2 - IVec3::ONE;
        let along = |axis: usize| {
            let mut step = IVec3::ZERO;
            step[axis] = toward[axis];
            step
        };
        let (side1, side2) = (along(a), along(b));
        [side1, side2, side1 + side2].map(|step| Neighborhood::stride(face.normal + step))
    })
}

/// Колонка, её восемь соседей и их свет: всё, из чего строится кэш секции. Берётся
/// один раз на колонку, а не хэш-запросом на клетку.
pub struct Surroundings<'a> {
    /// Индекс `dx + 1 + 3·(dz + 1)`, центр — 4; незагруженные — `None`
    /// (блоки там воздух, свет 0, как раньше у `World::block` и `LightMap::packed`).
    columns: [Option<&'a Column>; 9],
    lights: [Option<&'a ColumnLight>; 9],
}

impl<'a> Surroundings<'a> {
    /// `None`, если колонка `key` не загружена.
    pub fn new(world: &'a World, light: &'a LightMap, key: ColumnPos) -> Option<Self> {
        let columns = world.neighborhood(key);
        columns[4]?;
        Some(Self {
            columns,
            lights: light.neighborhood(key),
        })
    }

    /// Заполняет кэши для секции `sy` центральной колонки (0..8): строки по x копируются
    /// срезами. Ниже мира блоки — камень, свет 0; выше — воздух и полный свет.
    fn fill(&self, sy: i32, blocks: &mut Neighborhood, lights: &mut Lights) {
        for y in -1..=SECTION_SIZE {
            let world_y = sy * SECTION_SIZE + y;
            for z in -1..=SECTION_SIZE {
                let row = Neighborhood::index(IVec3::new(-1, y, z));
                let (block_row, light_row) = (
                    &mut blocks.0[row..row + PADDED as usize],
                    &mut lights.0[row..row + PADDED as usize],
                );
                if world_y < 0 {
                    block_row.fill(Block::Stone);
                    light_row.fill(0);
                    continue;
                }
                if world_y >= WORLD_HEIGHT {
                    block_row.fill(Block::Air);
                    light_row.fill(MAX_LIGHT);
                    continue;
                }
                let (section, in_y) = (world_y / SECTION_SIZE, world_y % SECTION_SIZE);
                let in_z = z.rem_euclid(SECTION_SIZE);
                let dz = (z + SECTION_SIZE) / SECTION_SIZE;
                // Строка из трёх кусков: последний блок западного соседа, строка
                // самой колонки, первый блок восточного.
                let last = SECTION_SIZE as usize - 1;
                for (dx, src, dst) in [
                    (0, last..last + 1, 0..1),
                    (1, 0..last + 1, 1..last + 2),
                    (2, 0..1, last + 2..last + 3),
                ] {
                    let i = (dx + 3 * dz) as usize;
                    match self.columns[i] {
                        Some(c) => block_row[dst.clone()]
                            .copy_from_slice(&c.section(section).row(in_y, in_z)[src.clone()]),
                        None => block_row[dst.clone()].fill(Block::Air),
                    }
                    match self.lights[i] {
                        Some(l) => {
                            light_row[dst.clone()].copy_from_slice(&l.row(world_y, in_z)[src])
                        }
                        None => light_row[dst].fill(0),
                    }
                }
            }
        }
    }
}

/// Меши секции по проходам отрисовки.
pub struct SectionMeshes {
    /// Непрозрачные блоки и листва (`RenderLayer::Opaque` и `Cutout`): у листвы альфа
    /// двоичная, ей хватает `discard` без смешения.
    pub solid: MeshData,
    /// Вода (`RenderLayer::Translucent`): рисуется после `solid` со смешением.
    pub water: MeshData,
}

impl SectionMeshes {
    fn empty() -> Self {
        Self {
            solid: MeshData::new(),
            water: MeshData::new(),
        }
    }
}

/// Копирует блоки секции в середину кэша срезами по строкам.
/// Вынесено из [`mesh_section`], чтобы замер по этапам (`mesh_stages`) видел этап отдельно.
fn fill_blocks(blocks: &mut Neighborhood, section: &Section) {
    for y in 0..SECTION_SIZE {
        for z in 0..SECTION_SIZE {
            let start = Neighborhood::index(IVec3::new(0, y, z));
            blocks.0[start..start + SECTION_SIZE as usize].copy_from_slice(section.row(y, z));
        }
    }
}

/// Добирает у кэша блоки соседей по краям и заполняет кэш света для всех клеток −1..=16.
/// Общий вариант с замыканиями — для секции вне мира (тесты); из мира кэш строит
/// [`Surroundings`].
fn fill_halo(
    blocks: &mut Neighborhood,
    lights: &mut Lights,
    neighbor: impl Fn(IVec3) -> Block,
    light: impl Fn(IVec3) -> u8,
) {
    for y in -1..=SECTION_SIZE {
        for z in -1..=SECTION_SIZE {
            for x in -1..=SECTION_SIZE {
                let local = IVec3::new(x, y, z);
                let index = Neighborhood::index(local);
                if !Section::contains(local) {
                    blocks.0[index] = neighbor(local);
                }
                lights.0[index] = light(local);
            }
        }
    }
}

fn new_caches() -> (Box<Neighborhood>, Box<Lights>) {
    (
        Box::new(Neighborhood(
            [Block::Air; (PADDED * PADDED * PADDED) as usize],
        )),
        Box::new(Lights([0; (PADDED * PADDED * PADDED) as usize])),
    )
}

/// Строит меши видимых граней секции по замыканиям. Позиции вершин локальные
/// (от угла секции, см. [`Vertex`]).
///
/// `neighbor` возвращает блок по локальным координатам за краем секции: вызывается для
/// любых координат вне 0..16 в пределах −1..=16 по каждой оси (рёбра и углы — две или
/// три компоненты вне 0..16). `light` — уровень света по локальным координатам
/// секции, для всех клеток −1..=16 по каждой оси. У секции из одного воздуха оба не
/// вызываются. Мир строит меши через [`mesh_world_section`] и [`mesh_world_column`],
/// без вызова замыкания на клетку.
pub fn mesh_section(
    section: &Section,
    neighbor: impl Fn(IVec3) -> Block,
    light: impl Fn(IVec3) -> u8,
    options: MeshOptions,
) -> SectionMeshes {
    if section.is_air() {
        return SectionMeshes::empty();
    }
    let (mut blocks, mut lights) = new_caches();
    fill_blocks(&mut blocks, section);
    fill_halo(&mut blocks, &mut lights, neighbor, light);
    mesh_cached(&blocks, &lights, options)
}

/// Грань блока рисуется, если сосед в её направлении не непрозрачный: так грани за
/// листвой и водой остаются в меше, сквозь них их видно. У воды правило другое — грань
/// только к воздуху: между водой и чем-либо ещё поверхности не видно, а грань воды к
/// листве легла бы в одну плоскость с гранью листвы. Если над водой не вода, её верх
/// опущен до [`WATER_SURFACE`]. AO у воды нет: только яркость стороны.
///
/// Свет плавный, как в Minecraft: в вершине — среднее уровней клетки перед гранью и
/// трёх клеток рядом с ней у этого угла (тех же, что у AO), без непрозрачных: иначе
/// стена тянула бы вершину к темноте, которой там нет. Угловую клетку не берём и
/// тогда, когда обе боковые непрозрачные: она не видна из угла, и свет просочился
/// бы сквозь стену. В вершину уходят раздельно `shade` (сторона × AO) и средние уровни
/// солнца и блоков в половинках ([`half_average`]); яркость из них считает шейдер со
/// тьмой неба. У воды — уровни клетки перед гранью, одни на грань. Диагональ квада
/// выбирается по дневной яркости ([`day_shade`]).
fn mesh_cached(blocks: &Neighborhood, lights: &Lights, options: MeshOptions) -> SectionMeshes {
    let mut meshes = SectionMeshes::empty();
    let faces = FACES.each_ref().map(|face| {
        (
            face,
            Neighborhood::stride(face.normal),
            corner_strides(face),
        )
    });
    let up = Neighborhood::stride(IVec3::Y);
    for y in 0..SECTION_SIZE {
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                let local = IVec3::new(x, y, z);
                let index = Neighborhood::index(local);
                let block = blocks.0[index];
                if block == Block::Air {
                    continue;
                }
                // Позиция вершины — шестнадцатые доли блока от угла секции.
                let base = local * POSITION_UNITS;
                if block.shape() != Shape::Cube {
                    let (sky, block_light) = lights.at(index, 0);
                    let light = [sky, block_light].map(|l| half_average(l, 1));
                    push_shaped(&mut meshes.solid, block, base, light);
                    continue;
                }
                if block.is_water() {
                    let top = if blocks.at(index, up).is_water() {
                        POSITION_UNITS
                    } else {
                        WATER_SURFACE_UNITS
                    };
                    for (face, normal, _) in &faces {
                        // Растения и факел не закрывают грань, но и не заполняют клетку.
                        if blocks.at(index, *normal).shape() != Shape::Cube
                            || blocks.at(index, *normal) == Block::Air
                        {
                            let (sky, block) = lights.at(index, *normal);
                            let shade = quantize_shade(face.shade);
                            let light = [sky, block].map(|l| half_average(l, 1));
                            push_water_face(&mut meshes.water, face, base, top, shade, light);
                        }
                    }
                    continue;
                }
                for (face, normal, corners) in &faces {
                    if blocks.opaque(index, *normal) {
                        continue;
                    }
                    let Some(tile) = block.tile(face.facing) else {
                        continue;
                    };
                    let front = lights.at(index, *normal);
                    let corner_data = corners.map(|[side1, side2, corner]| {
                        let open1 = !blocks.opaque(index, side1);
                        let open2 = !blocks.opaque(index, side2);
                        let open_corner = !blocks.opaque(index, corner);
                        let ao = if options.ambient_occlusion {
                            vertex_ao(!open1, !open2, !open_corner)
                        } else {
                            3
                        };
                        let (mut sky, mut block_sum, mut count) = (front.0, front.1, 1);
                        for (offset, open) in [
                            (side1, open1),
                            (side2, open2),
                            (corner, open_corner && (open1 || open2)),
                        ] {
                            if open {
                                let (s, b) = lights.at(index, offset);
                                sky += s;
                                block_sum += b;
                                count += 1;
                            }
                        }
                        let shade = quantize_shade(face.shade * AO_BRIGHTNESS[usize::from(ao)]);
                        (
                            shade,
                            [half_average(sky, count), half_average(block_sum, count)],
                        )
                    });
                    // Свет вершины — больший из двух средних: два источника не складываются,
                    // а под открытым небом без светокамня картинка прежняя.
                    let days =
                        corner_data.map(|(shade, [sun, block])| day_shade(shade, sun, block));
                    let layer = tile.layer() as u8;
                    let vertices = face.corners.iter().zip(CORNER_UVS).zip(corner_data).map(
                        |((&corner, uv), (shade, light))| {
                            let p = base + IVec3::from(corner) * POSITION_UNITS;
                            Vertex::new(
                                [p.x as u16, p.y as u16, p.z as u16],
                                uv,
                                shade,
                                layer,
                                light,
                            )
                        },
                    );
                    meshes.solid.push_quad(vertices, quad_indices(days));
                }
            }
        }
    }
    meshes
}

/// Яркость сторон креста: свет не зависит от направления, растение не должно быть
/// темнее одной стороной.
const CROSS_SHADE: f32 = 1.0;

/// Столбик факела в шагах позиции: центр клетки, [`TORCH_PIXELS`] по x и z, высота
/// от низа клетки до верха тайла без `TORCH_TOP_ROW` строк (10 из 16).
const TORCH_BOX: ([i32; 3], [i32; 3]) = (
    [TORCH_PIXELS.0 as i32, 0, TORCH_PIXELS.0 as i32],
    [
        TORCH_PIXELS.1 as i32,
        (TILE_SIZE - TORCH_TOP_ROW) as i32,
        TORCH_PIXELS.1 as i32,
    ],
);

/// Геометрия блока не-куба: крест или столбик. Без AO, свет — одной клетки блока
/// (`light` в половинках уровня), одинаковый на всех вершинах. Всё в слое Cutout вместе
/// с кубами.
fn push_shaped(mesh: &mut MeshData, block: Block, base: IVec3, light: [u8; 2]) {
    let u = POSITION_UNITS;
    let vertex = |p: IVec3, uv: [i32; 2], shade: u8, layer: u8| {
        let p = base + p;
        Vertex::new(
            [p.x as u16, p.y as u16, p.z as u16],
            [uv[0] as u8, uv[1] as u8],
            shade,
            layer,
            light,
        )
    };
    match block.shape() {
        Shape::Cube => {}
        Shape::Cross => {
            let layer = block.tile(Facing::Side).map_or(0, |t| t.layer() as u8);
            let shade = quantize_shade(CROSS_SHADE);
            // Две диагонали клетки; каждая — квад в обе стороны (иначе `cull_mode: Back`
            // скроет обратную): те же вершины в обратном порядке.
            for (a, b) in [([0, 0], [u, u]), ([u, 0], [0, u])] {
                let quad = [
                    vertex(IVec3::new(a[0], 0, a[1]), [0, u], shade, layer),
                    vertex(IVec3::new(b[0], 0, b[1]), [u, u], shade, layer),
                    vertex(IVec3::new(b[0], u, b[1]), [u, 0], shade, layer),
                    vertex(IVec3::new(a[0], u, a[1]), [0, 0], shade, layer),
                ];
                mesh.push_quad(quad.into_iter(), [0, 1, 2, 0, 2, 3]);
                mesh.push_quad(quad.into_iter().rev(), [0, 1, 2, 0, 2, 3]);
            }
        }
        Shape::Torch => {
            let (lo, hi) = TORCH_BOX;
            let layer = Tile::Torch.layer() as u8;
            for face in FACES.iter().filter(|f| f.facing != Facing::Bottom) {
                let shade = quantize_shade(face.shade);
                let vertices = face.corners.iter().map(|&[cx, cy, cz]| {
                    let c = [cx, cy, cz];
                    let p = IVec3::from(std::array::from_fn(
                        |i| {
                            if c[i] == 1 { hi[i] } else { lo[i] }
                        },
                    ));
                    // Текстура берётся по центру тайла: боковая грань — древко с огоньком
                    // сверху (v от верха столбика), верхняя — два верхних ряда огонька.
                    let uv = if face.facing == Facing::Top {
                        [
                            TORCH_PIXELS.0 as i32 + (TORCH_PIXELS.1 - TORCH_PIXELS.0) as i32 * cx,
                            TORCH_TOP_ROW as i32 + 2 * (1 - cz),
                        ]
                    } else {
                        let side = if face.normal.x != 0 { cz } else { cx };
                        let side = if face.normal.x > 0 || face.normal.z < 0 {
                            1 - side
                        } else {
                            side
                        };
                        [
                            TORCH_PIXELS.0 as i32 + (TORCH_PIXELS.1 - TORCH_PIXELS.0) as i32 * side,
                            TORCH_TOP_ROW as i32
                                + (TILE_SIZE as i32 - TORCH_TOP_ROW as i32) * (1 - cy),
                        ]
                    };
                    vertex(p, uv, shade, layer)
                });
                mesh.push_quad(vertices, [0, 1, 2, 0, 2, 3]);
            }
        }
    }
}

/// Грань воды; `top` — высота верха блока над `base` в шагах позиции (16 или
/// [`WATER_SURFACE_UNITS`]), `shade` и `light` — одни на все её вершины.
fn push_water_face(
    mesh: &mut MeshData,
    face: &Face,
    base: IVec3,
    top: i32,
    shade: u8,
    light: [u8; 2],
) {
    let vertices = face
        .corners
        .iter()
        .zip(CORNER_UVS)
        .map(|(&[x, y, z], [u, v])| {
            let y = if y == 1 { top } else { 0 };
            // На боковой грани v = 1 − y, как у полной (`CORNER_UVS`): опущенный верх
            // обрезает тайл, а не сжимает его.
            let v = if face.facing == Facing::Side {
                (POSITION_UNITS - y) as u8
            } else {
                v
            };
            let p = base + IVec3::new(x, 0, z) * POSITION_UNITS + IVec3::new(0, y, 0);
            Vertex::new(
                [p.x as u16, p.y as u16, p.z as u16],
                [u, v],
                shade,
                Tile::Water.layer() as u8,
                light,
            )
        });
    mesh.push_quad(vertices, [0, 1, 2, 0, 2, 3]);
}

impl MeshData {
    fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Добавляет квад из четырёх вершин; `order` — индексы двух треугольников в нём.
    fn push_quad(&mut self, vertices: impl Iterator<Item = Vertex>, order: [u16; 6]) {
        // Худший случай — шахматная расстановка: 2048 блоков × 24 = 49 152 вершины.
        debug_assert!(
            self.vertices.len() + 4 <= usize::from(u16::MAX) + 1,
            "section mesh overflows u16 indices"
        );
        let first = self.vertices.len() as u16;
        self.vertices.extend(vertices);
        self.indices.extend(order.map(|i| first + i));
    }
}

/// Средний уровень `sum / count` (count 1..=4) в половинках уровня, с округлением.
fn half_average(sum: u32, count: u32) -> u8 {
    ((4 * sum + count) / (2 * count)) as u8
}

/// Индексы двух треугольников квада, против часовой стрелки, как и `Face::corners`.
/// Яркость интерполируется по треугольникам, и тень от тёмной вершины на диагонали
/// тянулась бы косой полосой через весь квад. Режем по диагонали, концы которой в
/// сумме светлее (1–3, если пара 0–2 темнее): одиночная тёмная вершина тогда остаётся
/// в одном треугольнике. Сравниваем итоговую яркость (те же байты, что уйдут в
/// вершину), а не уровни AO: тень от света режется так же. Ночью диагональ не
/// пересчитывается — меши остаются прежними.
fn quad_indices(shades: [u8; 4]) -> [u16; 6] {
    let sum = |a: usize, b: usize| u16::from(shades[a]) + u16::from(shades[b]);
    if sum(0, 2) < sum(1, 3) {
        [1, 2, 3, 1, 3, 0]
    } else {
        [0, 1, 2, 0, 2, 3]
    }
}

/// Меши всех секций колонки по замыканиям, снизу вверх (`[0]` — y 0..16). Общий
/// вариант для колонки вне мира (тесты); из мира — [`mesh_world_column`].
///
/// Соседей по вертикали внутри колонки берёт из неё самой; `neighbor` вызывается только
/// для локальных координат колонки за её краем: x или z в −1 или 16 (в том числе оба —
/// угол колонки), либо y равен −1 или 128. `light` — уровень света по локальным
/// координатам колонки, внутри неё и за краем в тех же пределах.
pub fn mesh_column(
    column: &Column,
    neighbor: impl Fn(IVec3) -> Block,
    light: impl Fn(IVec3) -> u8,
    options: MeshOptions,
) -> Vec<SectionMeshes> {
    (0..COLUMN_SECTIONS)
        .map(|sy| {
            let offset = IVec3::new(0, sy * SECTION_SIZE, 0);
            mesh_section(
                column.section(sy),
                |local| {
                    let in_column = local + offset;
                    if Column::contains(in_column) {
                        column.get(in_column)
                    } else {
                        neighbor(in_column)
                    }
                },
                |local| light(local + offset),
                options,
            )
        })
        .collect()
}

/// Меши всех секций загруженной колонки `key`, снизу вверх; грани и свет на стыках —
/// из соседних колонок мира. `None`, если колонка не загружена.
pub fn mesh_world_column(
    world: &World,
    light: &LightMap,
    key: ColumnPos,
    options: MeshOptions,
) -> Option<Vec<SectionMeshes>> {
    let around = Surroundings::new(world, light, key)?;
    let (mut blocks, mut lights) = new_caches();
    let column = around.columns[4]?;
    Some(
        (0..COLUMN_SECTIONS)
            .map(|sy| {
                if column.section(sy).is_air() {
                    return SectionMeshes::empty();
                }
                around.fill(sy, &mut blocks, &mut lights);
                mesh_cached(&blocks, &lights, options)
            })
            .collect(),
    )
}

/// Меш одной секции `key` с соседями
/// из `world` и светом из `light` — для перестройки после правки блока или света.
/// `None`, если колонка не загружена или номер секции вне 0..COLUMN_SECTIONS.
pub fn mesh_world_section(
    world: &World,
    light: &LightMap,
    key: SectionPos,
    options: MeshOptions,
) -> Option<SectionMeshes> {
    if !(0..COLUMN_SECTIONS).contains(&key.y()) {
        return None;
    }
    let around = Surroundings::new(world, light, key.column())?;
    if around.columns[4]?.section(key.y()).is_air() {
        return Some(SectionMeshes::empty());
    }
    let (mut blocks, mut lights) = new_caches();
    around.fill(key.y(), &mut blocks, &mut lights);
    Some(mesh_cached(&blocks, &lights, options))
}

#[cfg(test)]
mod bench;
#[cfg(test)]
mod tests;
