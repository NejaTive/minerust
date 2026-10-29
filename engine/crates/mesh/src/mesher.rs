use glam::IVec3;

use crate::mesh::{POSITION_UNITS, Quad, day_shade, quantize_shade};
use crate::templates::{
    AO_BRIGHTNESS, CROSS, CUBE_FACE, FACES, Face, LOG_BARK_X, LOG_BARK_Z, TORCH_SIDE, TORCH_TOP,
    TORCH_WALL, TORCH_WALL_PARTS, WATER_BOTTOM, WATER_SIDE, WATER_SIDE_LOW, WATER_SIDE_SLOPED,
    WATER_SURFACE_UNITS, WATER_TOP, WATER_TOP_SLOPED,
};
use world::block::{
    ATTACH_NEG_Z, ATTACH_POS_X, Block, Facing, RenderLayer, Shape, fluid_height_units,
};
use world::column::{COLUMN_SECTIONS, Column, WORLD_HEIGHT};
use world::light::{ColumnLight, LightMap, MAX_LIGHT, block_level, sky_level};
use world::pos::{ColumnPos, SectionPos};
use world::section::{SECTION_SIZE, Section};
use world::tile::{LAYER_COUNT, Tile};
use world::world::World;

// Слой хранится в вершине одним байтом.
const _: () = assert!(LAYER_COUNT <= 256);

pub struct MeshData {
    pub quads: Vec<Quad>,
}

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
pub(crate) const PADDED: i32 = SECTION_SIZE + 2;

/// Блоки секции и всех её соседей в пределах −1..=16 по каждой оси. AO смотрит на
/// 12 блоков на грань, и брать их из массива намного дешевле, чем из мира (поиск
/// колонки в `HashMap`).
pub(crate) struct Neighborhood(pub(crate) [Block; (PADDED * PADDED * PADDED) as usize]);

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

/// Байты данных блоков секции и соседей, как [`Neighborhood`] для блоков: уровень воды
/// соседней клетки нужен высоте её поверхности у стыка.
pub(crate) struct Datas(pub(crate) [u8; (PADDED * PADDED * PADDED) as usize]);

impl Datas {
    fn at(&self, index: usize, offset: isize) -> u8 {
        self.0[index.wrapping_add_signed(offset)]
    }
}

/// Байты света секции и соседей ([`world::light::pack_light`]), как [`Neighborhood`]
/// для блоков.
pub(crate) struct Lights(pub(crate) [u8; (PADDED * PADDED * PADDED) as usize]);

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
    pub(crate) columns: [Option<&'a Column>; 9],
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
    /// срезами. Ниже мира блоки — камень, свет 0; выше — воздух и полный свет. Данные
    /// блоков вне мира и незагруженных колонок — нули. Возвращает, есть ли в `datas`
    /// ненулевой байт.
    pub(crate) fn fill(
        &self,
        sy: i32,
        blocks: &mut Neighborhood,
        lights: &mut Lights,
        datas: &mut Datas,
    ) -> bool {
        let mut has_data = false;
        for y in -1..=SECTION_SIZE {
            let world_y = sy * SECTION_SIZE + y;
            for z in -1..=SECTION_SIZE {
                let row = Neighborhood::index(IVec3::new(-1, y, z));
                let range = row..row + PADDED as usize;
                let (block_row, light_row, data_row) = (
                    &mut blocks.0[range.clone()],
                    &mut lights.0[range.clone()],
                    &mut datas.0[range],
                );
                data_row.fill(0);
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
                        Some(c) => {
                            let section = c.section(section);
                            block_row[dst.clone()]
                                .copy_from_slice(&section.row(in_y, in_z)[src.clone()]);
                            if let Some(bytes) = section.data_row(in_y, in_z) {
                                data_row[dst.clone()].copy_from_slice(&bytes[src.clone()]);
                                has_data |= bytes[src.clone()].iter().any(|&b| b != 0);
                            }
                        }
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
        has_data
    }
}

/// Меши секции по проходам отрисовки.
pub struct SectionMeshes {
    /// Непрозрачные блоки (`RenderLayer::Opaque`): шейдер без `discard`, поэтому GPU
    /// может отсекать скрытые пиксели до шейдера.
    pub solid: MeshData,
    /// Листва, кресты, факел (`RenderLayer::Cutout`): у них альфа двоичная, хватает
    /// `discard` без смешения. Рисуется после `solid`.
    pub cutout: MeshData,
    /// Вода (`RenderLayer::Translucent`): рисуется после `cutout` со смешением.
    pub water: MeshData,
}

impl SectionMeshes {
    pub(crate) fn empty() -> Self {
        Self {
            solid: MeshData::new(),
            cutout: MeshData::new(),
            water: MeshData::new(),
        }
    }
}

/// Копирует блоки и их данные секции в середину кэшей срезами по строкам. Данные
/// соседей остаются нулями. Вынесено из [`mesh_section`], чтобы замер по этапам
/// (`mesh_stages`) видел этап отдельно.
fn fill_blocks(blocks: &mut Neighborhood, datas: &mut Datas, section: &Section) {
    for y in 0..SECTION_SIZE {
        for z in 0..SECTION_SIZE {
            let start = Neighborhood::index(IVec3::new(0, y, z));
            let range = start..start + SECTION_SIZE as usize;
            blocks.0[range.clone()].copy_from_slice(section.row(y, z));
            if let Some(bytes) = section.data_row(y, z) {
                datas.0[range].copy_from_slice(bytes);
            }
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

pub(crate) fn new_caches() -> (Box<Neighborhood>, Box<Lights>, Box<Datas>) {
    (
        Box::new(Neighborhood(
            [Block::Air; (PADDED * PADDED * PADDED) as usize],
        )),
        Box::new(Lights([0; (PADDED * PADDED * PADDED) as usize])),
        Box::new(Datas([0; (PADDED * PADDED * PADDED) as usize])),
    )
}

/// Строит меши видимых граней секции по замыканиям. Позиции вершин локальные
/// (от угла секции, см. [`Quad`]).
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
    let (mut blocks, mut lights, mut datas) = new_caches();
    fill_blocks(&mut blocks, &mut datas, section);
    fill_halo(&mut blocks, &mut lights, neighbor, light);
    mesh_cached(&blocks, &lights, &datas, options)
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
///
/// Высота воды зависит от уровней соседей, поэтому данные берутся из кэша `datas` с
/// полем, как блоки. У воды верх каждого угла — среднее высот четырёх клеток вокруг
/// угла ([`surface_height`]), так что поверхность наклонная и стык двух клеток воды
/// общий: боковых граней между водой не нужно.
pub(crate) fn mesh_cached(
    blocks: &Neighborhood,
    lights: &Lights,
    datas: &Datas,
    options: MeshOptions,
) -> SectionMeshes {
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
                let cell = [x as u8, y as u8, z as u8];
                let byte = || datas.0[index];
                if block.shape() != Shape::Cube {
                    let (sky, block_light) = lights.at(index, 0);
                    let light = [sky, block_light].map(|l| half_average(l, 1));
                    push_shaped(&mut meshes.cutout, block, byte(), cell, light);
                    continue;
                }
                if block.is_water() {
                    for (face_index, (face, normal, _)) in faces.iter().enumerate() {
                        // Растения и факел не закрывают грань, но и не заполняют клетку.
                        if blocks.at(index, *normal).shape() == Shape::Cube
                            && blocks.at(index, *normal) != Block::Air
                        {
                            continue;
                        }
                        let (sky, block) = lights.at(index, *normal);
                        let light = [sky, block].map(|l| half_average(l, 1));
                        let heights = face
                            .corners
                            .map(|[cx, _, cz]| surface_height(blocks, datas, index, cx, cz, up));
                        let (template, sloped) = water_template(face_index, heights);
                        let quad = Quad::new(
                            cell,
                            template,
                            false,
                            Tile::Water.layer() as u8,
                            [3; 4],
                            [light[0]; 4],
                            [light[1]; 4],
                        );
                        meshes.water.quads.push(if sloped {
                            quad.with_heights(heights)
                        } else {
                            quad
                        });
                    }
                    continue;
                }
                // Лежачее бревно; стоячее рисуется как обычный куб.
                let lying = (block == Block::Log)
                    .then(|| block.axis(byte()))
                    .filter(|&axis| axis != 1);
                for (face_index, (face, normal, corners)) in faces.iter().enumerate() {
                    if blocks.opaque(index, *normal) {
                        continue;
                    }
                    let (tile, template) = match lying {
                        Some(axis) => axis_face(block, axis, face_index, face),
                        None => (block.tile(face.facing), CUBE_FACE + face_index as u8),
                    };
                    let Some(tile) = tile else {
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
                            ao,
                        )
                    });
                    // Свет вершины — больший из двух средних: два источника не складываются,
                    // а под открытым небом без светокамня картинка прежняя.
                    let days =
                        corner_data.map(|(shade, [sun, block], _)| day_shade(shade, sun, block));
                    let layer = tile.layer() as u8;
                    let target = if block.render_layer() == RenderLayer::Cutout {
                        &mut meshes.cutout
                    } else {
                        &mut meshes.solid
                    };
                    target.quads.push(Quad::new(
                        cell,
                        template,
                        flip_diagonal(days),
                        layer,
                        corner_data.map(|c| c.2),
                        corner_data.map(|c| c.1[0]),
                        corner_data.map(|c| c.1[1]),
                    ));
                }
            }
        }
    }
    meshes
}

const TALL_WATER_UNITS: u32 = 13;
const TALL_WATER_WEIGHT: u32 = 10;

/// Высота воды клетки `index` в углу `(cx, cz)` (0 или 1 по x и z), шаги 1/16 блока:
/// среднее высот четырёх клеток вокруг угла. Вода над любой из них — угол полный (16);
/// воздух и растения считаются нулевой высоты, твёрдые блоки не учитываются. Высокая вода
/// (от [`TALL_WATER_UNITS`]) весит [`TALL_WATER_WEIGHT`] раз больше, как в Minecraft: край
/// озера рядом с воздухом провисает не до середины. `up` — сдвиг индекса на клетку вверх.
fn surface_height(
    blocks: &Neighborhood,
    datas: &Datas,
    index: usize,
    cx: i32,
    cz: i32,
    up: isize,
) -> u8 {
    let (mut sum, mut count) = (0u32, 0u32);
    for dz in [cz - 1, cz] {
        for dx in [cx - 1, cx] {
            let offset = Neighborhood::stride(IVec3::new(dx, 0, dz));
            let block = blocks.at(index, offset);
            if block.is_water() {
                if blocks.at(index, offset + up).is_water() {
                    return POSITION_UNITS as u8;
                }
                let height = u32::from(fluid_height_units(datas.at(index, offset)));
                let weight = if height >= TALL_WATER_UNITS {
                    TALL_WATER_WEIGHT
                } else {
                    1
                };
                sum += height * weight;
                count += weight;
            } else if !block.is_solid() {
                count += 1;
            }
        }
    }
    // Собственная клетка — вода, поэтому счёт не нулевой.
    ((2 * sum + count) / (2 * count)).max(1) as u8
}

/// Шаблон грани воды `face_index` (номер в `FACES`) по высотам её углов и признак
/// наклона: при наклоне высоты уходят в запись квада. Верх ровно на уровне источника и
/// полные стороны, как и прежде, не несут высот.
fn water_template(face_index: usize, heights: [u8; 4]) -> (u8, bool) {
    let source = WATER_SURFACE_UNITS as u8;
    let full = POSITION_UNITS as u8;
    match face_index {
        0 if heights == [source; 4] => (WATER_TOP, false),
        0 => (WATER_TOP_SLOPED, true),
        1 => (WATER_BOTTOM, false),
        side => {
            let side = (side - 2) as u8;
            match [heights[2], heights[3]] {
                [h, g] if h == full && g == full => (WATER_SIDE + side, false),
                [h, g] if h == source && g == source => (WATER_SIDE_LOW + side, false),
                _ => (WATER_SIDE_SLOPED + side, true),
            }
        }
    }
}

/// Тайл и шаблон грани блока с осью, лежащего вдоль `axis` (0 — x, 2 — z): торцы на
/// гранях поперёк оси, на остальных кора с UV, повёрнутой вдоль оси.
fn axis_face(block: Block, axis: usize, face_index: usize, face: &Face) -> (Option<Tile>, u8) {
    if face.normal[axis] != 0 {
        return (block.tile(Facing::Top), CUBE_FACE + face_index as u8);
    }
    let template = if axis == 0 {
        // Кора по оси X — грани 0, 1, 4, 5 из `FACES`.
        LOG_BARK_X + [0, 1, 0, 0, 2, 3][face_index]
    } else {
        LOG_BARK_Z + face_index as u8
    };
    (block.tile(Facing::Side), template)
}

/// Записи квадов блока не-куба: крест (две плоскости в обе стороны) или столбик факела
/// (стоячий или наклонённый от стены по `data`).
/// Без AO, свет — одной клетки блока (`light` в половинках уровня), одинаковый на всех углах.
/// Всегда слой Cutout.
fn push_shaped(mesh: &mut MeshData, block: Block, data: u8, cell: [u8; 3], light: [u8; 2]) {
    let quad = |template: u8, layer: u8| {
        Quad::new(
            cell,
            template,
            false,
            layer,
            [3; 4],
            [light[0]; 4],
            [light[1]; 4],
        )
    };
    match block.shape() {
        Shape::Cube => {}
        Shape::Cross => {
            let layer = block.tile(Facing::Side).map_or(0, |t| t.layer() as u8);
            for t in CROSS..CROSS + 4 {
                mesh.quads.push(quad(t, layer));
            }
        }
        Shape::Torch => {
            let layer = Tile::Torch.layer() as u8;
            if (ATTACH_POS_X..=ATTACH_NEG_Z).contains(&data) {
                let first = TORCH_WALL + TORCH_WALL_PARTS * (data - ATTACH_POS_X);
                for t in first..first + TORCH_WALL_PARTS {
                    mesh.quads.push(quad(t, layer));
                }
                return;
            }
            // Верх, затем бока.
            for t in std::iter::once(TORCH_TOP).chain(TORCH_SIDE..TORCH_TOP) {
                mesh.quads.push(quad(t, layer));
            }
        }
    }
}

impl MeshData {
    fn new() -> Self {
        Self { quads: Vec::new() }
    }
}

/// Средний уровень `sum / count` (count 1..=4) в половинках уровня, с округлением.
fn half_average(sum: u32, count: u32) -> u8 {
    ((4 * sum + count) / (2 * count)) as u8
}

/// Переворачивать ли диагональ квада: треугольники `0,1,2` и `0,2,3` или `1,2,3` и
/// `1,3,0` (оба против часовой стрелки, как и углы шаблона). Яркость интерполируется по
/// треугольникам, и тень от тёмной вершины на диагонали тянулась бы косой полосой через
/// весь квад. Режем по диагонали, концы которой в сумме светлее (1–3, если пара 0–2
/// темнее): одиночная тёмная вершина тогда остаётся в одном треугольнике. Сравниваем
/// итоговую яркость в байтах (`day_shade`), а не уровни AO: тень от света режется так
/// же. Ночью диагональ не пересчитывается — меши остаются прежними.
fn flip_diagonal(shades: [u8; 4]) -> bool {
    let sum = |a: usize, b: usize| u16::from(shades[a]) + u16::from(shades[b]);
    sum(0, 2) < sum(1, 3)
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
    let (mut blocks, mut lights, mut datas) = new_caches();
    let column = around.columns[4]?;
    Some(
        (0..COLUMN_SECTIONS)
            .map(|sy| {
                if column.section(sy).is_air() {
                    return SectionMeshes::empty();
                }
                around.fill(sy, &mut blocks, &mut lights, &mut datas);
                mesh_cached(&blocks, &lights, &datas, options)
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
    let (mut blocks, mut lights, mut datas) = new_caches();
    around.fill(key.y(), &mut blocks, &mut lights, &mut datas);
    Some(mesh_cached(&blocks, &lights, &datas, options))
}

#[cfg(test)]
mod bench;
#[cfg(test)]
mod expand;
#[cfg(test)]
mod oriented;
#[cfg(test)]
mod tests;
