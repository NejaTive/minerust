use glam::{IVec2, IVec3};

use crate::block::{Block, Facing};
use crate::column::{COLUMN_SECTIONS, Column};
use crate::mesh::Vertex;
use crate::section::{SECTION_SIZE, Section};
use crate::world::World;

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
    corners: [[f32; 3]; 4],
}

/// UV углов грани в порядке `Face::corners`: у боковых граней v = 0 оказывается
/// сверху, и кайма травы смотрит вверх.
const CORNER_UVS: [[f32; 2]; 4] = [[0., 1.], [1., 1.], [1., 0.], [0., 0.]];

const FACES: [Face; 6] = [
    Face {
        normal: IVec3::Y,
        facing: Facing::Top,
        shade: 1.0,
        corners: [[0., 1., 1.], [1., 1., 1.], [1., 1., 0.], [0., 1., 0.]],
    },
    Face {
        normal: IVec3::NEG_Y,
        facing: Facing::Bottom,
        shade: 0.5,
        corners: [[0., 0., 0.], [1., 0., 0.], [1., 0., 1.], [0., 0., 1.]],
    },
    Face {
        normal: IVec3::X,
        facing: Facing::Side,
        shade: 0.6,
        corners: [[1., 0., 1.], [1., 0., 0.], [1., 1., 0.], [1., 1., 1.]],
    },
    Face {
        normal: IVec3::NEG_X,
        facing: Facing::Side,
        shade: 0.6,
        corners: [[0., 0., 0.], [0., 0., 1.], [0., 1., 1.], [0., 1., 0.]],
    },
    Face {
        normal: IVec3::Z,
        facing: Facing::Side,
        shade: 0.8,
        corners: [[0., 0., 1.], [1., 0., 1.], [1., 1., 1.], [0., 1., 1.]],
    },
    Face {
        normal: IVec3::NEG_Z,
        facing: Facing::Side,
        shade: 0.8,
        corners: [[1., 0., 0.], [0., 0., 0.], [0., 1., 0.], [1., 1., 0.]],
    },
];

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
/// `side1`, `side2` — твёрдые ли блоки, примыкающие к вершине по рёбрам грани,
/// `corner` — блок по диагонали; все три — в слое перед гранью. Если обе стороны
/// твёрдые, угловой блок не виден и уровень 0 независимо от него.
pub fn vertex_ao(side1: bool, side2: bool, corner: bool) -> u8 {
    if side1 && side2 {
        0
    } else {
        3 - (u8::from(side1) + u8::from(side2) + u8::from(corner))
    }
}

/// Сторона куба блоков секции с полем в один блок со всех сторон.
const PADDED: i32 = SECTION_SIZE + 2;

/// Твёрдость блоков секции и всех её соседей в пределах −1..=16 по каждой оси.
/// AO смотрит на 12 блоков на грань, и брать их из массива намного дешевле, чем
/// через `neighbor` (в движке это поиск колонки в `HashMap`).
struct Solidity([bool; (PADDED * PADDED * PADDED) as usize]);

impl Solidity {
    /// Индекс локальных координат секции, −1..=16 по каждой оси.
    fn index(local: IVec3) -> usize {
        let p = local + IVec3::ONE;
        (p.x + PADDED * p.z + PADDED * PADDED * p.y) as usize
    }

    /// Сдвиг индекса при шаге на `offset`: так соседи блока берутся без пересчёта координат.
    fn stride(offset: IVec3) -> isize {
        (offset.x + PADDED * offset.z + PADDED * PADDED * offset.y) as isize
    }

    fn at(&self, index: usize, offset: isize) -> bool {
        self.0[index.wrapping_add_signed(offset)]
    }
}

/// Для каждой вершины грани — сдвиги индекса в [`Solidity`] к трём блокам перед ней:
/// два боковых и угловой.
type CornerStrides = [[isize; 3]; 4];

fn corner_strides(face: &Face) -> CornerStrides {
    let normal_axis = face.normal.abs().max_position();
    let [a, b] = [1, 2].map(|i| (normal_axis + i) % 3);
    face.corners.map(|corner| {
        // От центра блока к вершине: ±1 по каждой оси.
        let toward = (glam::Vec3::from(corner) * 2.0 - 1.0).as_ivec3();
        let along = |axis: usize| {
            let mut step = IVec3::ZERO;
            step[axis] = toward[axis];
            step
        };
        let (side1, side2) = (along(a), along(b));
        [side1, side2, side1 + side2].map(|step| Solidity::stride(face.normal + step))
    })
}

/// Строит меш видимых граней секции: грань рисуется, если блок твёрдый,
/// а сосед в её направлении — нет. Позиции вершин мировые.
///
/// `origin` — мировые координаты минимального угла секции в блоках (координаты секции × 16).
/// `neighbor` возвращает блок по локальным координатам за краем секции: вызывается для
/// любых координат вне 0..16 в пределах −1..=16 по каждой оси (рёбра и углы — две или
/// три компоненты вне 0..16). У секции из одного воздуха не вызывается.
pub fn mesh_section(
    section: &Section,
    origin: IVec3,
    neighbor: impl Fn(IVec3) -> Block,
    options: MeshOptions,
) -> MeshData {
    let mut mesh = MeshData {
        vertices: Vec::new(),
        indices: Vec::new(),
    };
    let mut solid = Solidity([false; (PADDED * PADDED * PADDED) as usize]);
    let mut any_solid = false;
    for y in 0..SECTION_SIZE {
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                let local = IVec3::new(x, y, z);
                let is_solid = section.get(local).is_solid();
                solid.0[Solidity::index(local)] = is_solid;
                any_solid |= is_solid;
            }
        }
    }
    if !any_solid {
        return mesh;
    }
    for y in -1..=SECTION_SIZE {
        for z in -1..=SECTION_SIZE {
            for x in -1..=SECTION_SIZE {
                let local = IVec3::new(x, y, z);
                if !Section::contains(local) {
                    solid.0[Solidity::index(local)] = neighbor(local).is_solid();
                }
            }
        }
    }

    let faces = FACES
        .each_ref()
        .map(|face| (face, Solidity::stride(face.normal), corner_strides(face)));
    for y in 0..SECTION_SIZE {
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                let local = IVec3::new(x, y, z);
                let block = section.get(local);
                if !block.is_solid() {
                    continue;
                }
                let index = Solidity::index(local);
                let base = (origin + local).as_vec3();
                for (face, normal, corners) in &faces {
                    if solid.at(index, *normal) {
                        continue;
                    }
                    let Some(tile) = block.tile(face.facing) else {
                        continue;
                    };
                    let ao = if options.ambient_occlusion {
                        corners.map(|[side1, side2, corner]| {
                            vertex_ao(
                                solid.at(index, side1),
                                solid.at(index, side2),
                                solid.at(index, corner),
                            )
                        })
                    } else {
                        [3; 4]
                    };
                    // Худший случай — шахматная расстановка: 2048 блоков × 24 = 49 152 вершины.
                    debug_assert!(
                        mesh.vertices.len() + 4 <= usize::from(u16::MAX) + 1,
                        "section mesh overflows u16 indices"
                    );
                    let first = mesh.vertices.len() as u16;
                    mesh.vertices
                        .extend(face.corners.iter().zip(CORNER_UVS).zip(ao).map(
                            |((&corner, uv), ao)| Vertex {
                                position: (base + glam::Vec3::from(corner)).to_array(),
                                uv,
                                shade: face.shade * AO_BRIGHTNESS[usize::from(ao)],
                                layer: tile.layer(),
                            },
                        ));
                    mesh.indices.extend(quad_indices(ao).map(|i| first + i));
                }
            }
        }
    }
    mesh
}

/// Индексы двух треугольников квада, против часовой стрелки, как и `Face::corners`.
/// Яркость интерполируется по треугольникам, и тень от тёмной вершины на диагонали
/// тянулась бы косой полосой через весь квад. Режем по диагонали, концы которой в
/// сумме светлее (1–3, если пара 0–2 темнее): одиночная тёмная вершина тогда остаётся
/// в одном треугольнике.
fn quad_indices(ao: [u8; 4]) -> [u16; 6] {
    if ao[0] + ao[2] < ao[1] + ao[3] {
        [1, 2, 3, 1, 3, 0]
    } else {
        [0, 1, 2, 0, 2, 3]
    }
}

/// Меши всех секций колонки, снизу вверх (`[0]` — y 0..16).
///
/// `origin` — мировые координаты минимального угла колонки (y = 0). Соседей по
/// вертикали внутри колонки берёт из неё самой; `neighbor` вызывается только для
/// локальных координат колонки за её краем: x или z в −1 или 16 (в том числе оба —
/// угол колонки), либо y равен −1 или 128.
pub fn mesh_column(
    column: &Column,
    origin: IVec3,
    neighbor: impl Fn(IVec3) -> Block,
    options: MeshOptions,
) -> Vec<MeshData> {
    (0..COLUMN_SECTIONS)
        .map(|sy| {
            let offset = IVec3::new(0, sy * SECTION_SIZE, 0);
            mesh_section(
                column.section(sy),
                origin + offset,
                |local| {
                    let in_column = local + offset;
                    if Column::contains(in_column) {
                        column.get(in_column)
                    } else {
                        neighbor(in_column)
                    }
                },
                options,
            )
        })
        .collect()
}

/// Меш одной секции по ключу `(x колонки, номер секции 0..8, z колонки)` с соседями
/// из `world` — для перестройки после правки блока. `None`, если колонка не загружена
/// или номер секции вне 0..COLUMN_SECTIONS.
pub fn mesh_world_section(world: &World, key: IVec3, options: MeshOptions) -> Option<MeshData> {
    if !(0..COLUMN_SECTIONS).contains(&key.y) {
        return None;
    }
    let column = world.column(IVec2::new(key.x, key.z))?;
    let origin = key * SECTION_SIZE;
    Some(mesh_section(
        column.section(key.y),
        origin,
        |local| world.block(origin + local),
        options,
    ))
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::*;
    use crate::texture::Tile;

    const AO: MeshOptions = MeshOptions {
        ambient_occlusion: true,
    };
    const NO_AO: MeshOptions = MeshOptions {
        ambient_occlusion: false,
    };

    fn air(_: IVec3) -> Block {
        Block::Air
    }

    fn face_count(mesh: &MeshData) -> usize {
        mesh.indices.len() / 6
    }

    fn filled(pred: impl Fn(IVec3) -> bool) -> Section {
        let mut section = Section::new();
        for y in 0..SECTION_SIZE {
            for z in 0..SECTION_SIZE {
                for x in 0..SECTION_SIZE {
                    let local = IVec3::new(x, y, z);
                    if pred(local) {
                        section.set(local, Block::Stone);
                    }
                }
            }
        }
        section
    }

    #[test]
    fn empty_section_has_no_faces() {
        let mesh = mesh_section(&Section::new(), IVec3::ZERO, air, AO);
        assert_eq!(face_count(&mesh), 0);
        assert!(mesh.vertices.is_empty());
    }

    #[test]
    fn single_block_has_six_faces() {
        let mut section = Section::new();
        section.set(IVec3::new(5, 5, 5), Block::Dirt);
        let mesh = mesh_section(&section, IVec3::ZERO, air, AO);
        assert_eq!(face_count(&mesh), 6);
        assert_eq!(mesh.vertices.len(), 24);
    }

    #[test]
    fn vertices_are_in_world_space() {
        let mut section = Section::new();
        section.set(IVec3::new(1, 2, 3), Block::Dirt);
        let origin = IVec3::new(16, 0, -32);
        let mesh = mesh_section(&section, origin, air, AO);
        let min = Vec3::new(17.0, 2.0, -29.0);
        for v in &mesh.vertices {
            let p = Vec3::from(v.position);
            assert!(p.cmpge(min).all() && p.cmple(min + Vec3::ONE).all(), "{p}");
        }
    }

    #[test]
    fn adjacent_blocks_share_hidden_face() {
        let mut section = Section::new();
        section.set(IVec3::new(5, 5, 5), Block::Dirt);
        section.set(IVec3::new(6, 5, 5), Block::Dirt);
        let mesh = mesh_section(&section, IVec3::ZERO, air, AO);
        assert_eq!(face_count(&mesh), 10);
    }

    #[test]
    fn solid_section_draws_only_outer_shell() {
        let mesh = mesh_section(&filled(|_| true), IVec3::ZERO, air, AO);
        assert_eq!(face_count(&mesh), 6 * 256);
    }

    #[test]
    fn solid_section_with_solid_neighbors_has_no_faces() {
        let mesh = mesh_section(
            &filled(|_| true),
            IVec3::ZERO,
            |local| {
                assert!(!Section::contains(local), "neighbor called for {local}");
                assert!(
                    local.cmpge(IVec3::NEG_ONE).all() && local.cmple(IVec3::splat(16)).all(),
                    "neighbor called for {local}"
                );
                Block::Stone
            },
            AO,
        );
        assert_eq!(face_count(&mesh), 0);
    }

    #[test]
    fn air_section_does_not_ask_neighbors() {
        let mesh = mesh_section(
            &Section::new(),
            IVec3::ZERO,
            |local| panic!("neighbor called for {local}"),
            AO,
        );
        assert!(mesh.vertices.is_empty());
    }

    #[test]
    fn checkerboard_fits_u16() {
        let section = filled(|p| (p.x + p.y + p.z) % 2 == 0);
        let mesh = mesh_section(&section, IVec3::ZERO, air, AO);
        assert_eq!(face_count(&mesh), 2048 * 6);
        assert_eq!(mesh.vertices.len(), 2048 * 24);
        assert!(
            mesh.indices
                .iter()
                .all(|&i| usize::from(i) < mesh.vertices.len())
        );
    }

    #[test]
    fn grass_faces_use_their_tiles() {
        let mut section = Section::new();
        section.set(IVec3::ZERO, Block::Grass);
        let mesh = mesh_section(&section, IVec3::ZERO, air, AO);
        for quad in mesh.vertices.chunks(4) {
            let ys: Vec<f32> = quad.iter().map(|v| v.position[1]).collect();
            let expected = if ys.iter().all(|&y| y == 1.0) {
                Tile::GrassTop
            } else if ys.iter().all(|&y| y == 0.0) {
                Tile::Dirt
            } else {
                Tile::GrassSide
            };
            assert!(quad.iter().all(|v| v.layer == expected.layer()));
        }
    }

    #[test]
    fn side_faces_are_upright() {
        let mut section = Section::new();
        section.set(IVec3::ZERO, Block::Grass);
        let mesh = mesh_section(&section, IVec3::ZERO, air, AO);
        let sides = mesh
            .vertices
            .chunks(4)
            .filter(|quad| quad.iter().any(|v| v.position[1] != quad[0].position[1]));
        let mut count = 0;
        for quad in sides {
            for v in quad {
                assert_eq!(v.uv[1], 1.0 - v.position[1], "v = 0 must be the top edge");
            }
            count += 1;
        }
        assert_eq!(count, 4);
    }

    #[test]
    fn faces_wind_counter_clockwise_from_outside() {
        let mut section = Section::new();
        section.set(IVec3::ZERO, Block::Grass);
        let mesh = mesh_section(&section, IVec3::ZERO, air, AO);
        let center = Vec3::splat(0.5);
        let mut normals = Vec::new();
        for quad in mesh.vertices.chunks(4) {
            let [a, b, c] = [0, 1, 2].map(|i| Vec3::from(quad[i].position));
            let winding = (b - a).cross(c - b).normalize();
            // Сторону грани берём из геометрии: от центра блока к центру грани.
            let face_center = quad.iter().map(|v| Vec3::from(v.position)).sum::<Vec3>() / 4.0;
            let normal = (face_center - center) * 2.0;
            assert!(
                winding.dot(normal) > 0.999,
                "winding {winding}, normal {normal}"
            );
            normals.push(normal.round().as_ivec3());
        }
        normals.sort_by_key(|n| n.to_array());
        normals.dedup();
        assert_eq!(normals.len(), 6);
    }

    /// Нормаль квада по обходу первых трёх вершин: обход против часовой снаружи.
    fn quad_normal(quad: &[Vertex]) -> IVec3 {
        let [a, b, c] = [0, 1, 2].map(|i| Vec3::from(quad[i].position));
        (b - a).cross(c - b).normalize().round().as_ivec3()
    }

    /// Уровни AO вершин квада, посчитанные по миру напрямую: блок за гранью и три
    /// блока перед каждой вершиной.
    fn world_ao(block_at: impl Fn(IVec3) -> Block, quad: &[Vertex]) -> [u8; 4] {
        let normal = quad_normal(quad);
        let center = quad.iter().map(|v| Vec3::from(v.position)).sum::<Vec3>() / 4.0;
        let block = (center - normal.as_vec3() * 0.5).floor().as_ivec3();
        let mid = block.as_vec3() + 0.5;
        std::array::from_fn(|i| {
            let toward = (Vec3::from(quad[i].position) - mid).signum().as_ivec3()
                * (IVec3::ONE - normal.abs());
            let axes: Vec<IVec3> = (0..3)
                .filter(|&a| toward[a] != 0)
                .map(|a| {
                    let mut step = IVec3::ZERO;
                    step[a] = toward[a];
                    step
                })
                .collect();
            let front = block + normal;
            vertex_ao(
                block_at(front + axes[0]).is_solid(),
                block_at(front + axes[1]).is_solid(),
                block_at(front + toward).is_solid(),
            )
        })
    }

    fn face_shade(normal: IVec3) -> f32 {
        FACES.iter().find(|f| f.normal == normal).unwrap().shade
    }

    /// Каждый квад меша: яркость вершин и выбор диагонали совпадают с расчётом по
    /// `block_at`. Возвращает число затенённых вершин, где `pred` от позиции истинен.
    fn assert_ao_matches(
        mesh: &MeshData,
        block_at: impl Fn(IVec3) -> Block,
        pred: impl Fn(Vec3) -> bool,
    ) -> usize {
        let mut shaded = 0;
        for (q, quad) in mesh.vertices.chunks(4).enumerate() {
            let ao = world_ao(&block_at, quad);
            let shade = face_shade(quad_normal(quad));
            for (v, ao) in quad.iter().zip(ao) {
                assert_eq!(v.shade, shade * AO_BRIGHTNESS[usize::from(ao)], "{v:?}");
                if ao < 3 && pred(Vec3::from(v.position)) {
                    shaded += 1;
                }
            }
            let first = (q * 4) as u16;
            let indices = &mesh.indices[q * 6..q * 6 + 6];
            assert_eq!(indices, quad_indices(ao).map(|i| first + i), "quad {q}");
        }
        shaded
    }

    #[test]
    fn vertex_ao_levels() {
        let cases = [
            ((false, false, false), 3),
            ((false, false, true), 2),
            ((true, false, false), 2),
            ((false, true, false), 2),
            ((true, false, true), 1),
            ((false, true, true), 1),
            ((true, true, false), 0),
            ((true, true, true), 0),
        ];
        for ((side1, side2, corner), level) in cases {
            assert_eq!(
                vertex_ao(side1, side2, corner),
                level,
                "{side1} {side2} {corner}"
            );
        }
    }

    /// Пол на y = 0 по всей секции и `extra` поверх него.
    fn floor_with(extra: &[IVec3]) -> Section {
        filled(|p| p.y == 0 || extra.contains(&p))
    }

    /// Вершины верхней грани блока `block` в порядке `Face::corners` и её индексы.
    fn top_face(mesh: &MeshData, block: IVec3) -> ([Vertex; 4], [u16; 6]) {
        let q = mesh
            .vertices
            .chunks(4)
            .position(|quad| {
                quad_normal(quad) == IVec3::Y
                    && quad[3].position == (block + IVec3::Y).as_vec3().to_array()
            })
            .unwrap_or_else(|| panic!("no top face of {block}"));
        let first = (q * 4) as u16;
        let indices = std::array::from_fn(|i| mesh.indices[q * 6 + i] - first);
        (mesh.vertices[q * 4..q * 4 + 4].try_into().unwrap(), indices)
    }

    fn shades(quad: &[Vertex; 4]) -> [f32; 4] {
        quad.map(|v| v.shade)
    }

    #[test]
    fn open_floor_and_block_tops_are_unshaded() {
        let block = IVec3::new(8, 1, 8);
        let mesh = mesh_section(&floor_with(&[block]), IVec3::ZERO, air, AO);
        assert_eq!(shades(&top_face(&mesh, block).0), [1.0; 4]);
        assert_eq!(shades(&top_face(&mesh, IVec3::new(4, 0, 4)).0), [1.0; 4]);
        // Пол у подножия блока со стороны −x: у ребра к блоку один боковой сосед.
        let (beside, _) = top_face(&mesh, IVec3::new(7, 0, 8));
        assert_eq!(shades(&beside), [1.0, 0.85, 0.85, 1.0]);
    }

    #[test]
    fn inner_corner_is_darkest() {
        // Стены вдоль x = 0 и z = 0 высотой в два блока; пол в углу — блок (1, 0, 1).
        let walls: Vec<IVec3> = (1..3)
            .flat_map(|y| (0..16).flat_map(move |i| [IVec3::new(0, y, i), IVec3::new(i, y, 0)]))
            .collect();
        let section = floor_with(&walls);
        let mesh = mesh_section(&section, IVec3::ZERO, air, AO);
        let (quad, indices) = top_face(&mesh, IVec3::new(1, 0, 1));
        // Углы верхней грани: (1, 2), (2, 2), (2, 1), (1, 1) по x, z.
        assert_eq!(shades(&quad), [0.7, 1.0, 0.7, 0.5]);
        // Диагональ 0–2 темнее (1 + 1 против 3 + 0) — режем по 1–3.
        assert_eq!(indices, [1, 2, 3, 1, 3, 0]);
        assert!(assert_ao_matches(&mesh, in_section(&section), |_| true) > 0);
    }

    /// Блок секции, за её краем — воздух.
    fn in_section(section: &Section) -> impl Fn(IVec3) -> Block {
        |p| {
            if Section::contains(p) {
                section.get(p)
            } else {
                Block::Air
            }
        }
    }

    /// Пара вершин квада, общая для двух его треугольников.
    fn diagonal(indices: [u16; 6]) -> [u16; 2] {
        let mut shared: Vec<u16> = indices[..3]
            .iter()
            .copied()
            .filter(|i| indices[3..].contains(i))
            .collect();
        shared.sort();
        shared.try_into().unwrap()
    }

    #[test]
    fn diagonal_avoids_single_dark_vertex() {
        let floor = IVec3::new(5, 0, 5);
        // Блок по диагонали над одним из углов верхней грани `floor`: затеняет одну
        // вершину. Углы грани по порядку: (5, 6), (6, 6), (6, 5), (5, 5) по x, z.
        let cases = [
            (IVec3::new(4, 1, 6), 0),
            (IVec3::new(6, 1, 6), 1),
            (IVec3::new(6, 1, 4), 2),
            (IVec3::new(4, 1, 4), 3),
        ];
        for (block, dark) in cases {
            let mesh = mesh_section(&floor_with(&[block]), IVec3::ZERO, air, AO);
            let (quad, indices) = top_face(&mesh, floor);
            let mut expected = [1.0; 4];
            expected[dark] = 0.85;
            assert_eq!(shades(&quad), expected, "{block}");
            assert!(
                !diagonal(indices).contains(&(dark as u16)),
                "{block}: {indices:?}"
            );
        }
    }

    #[test]
    fn flipped_quads_keep_winding() {
        // Рваный рельеф, чтобы встретились обе диагонали.
        let section = filled(|p| p.y < 4 + (p.x * 7 + p.z * 13) % 5);
        let mesh = mesh_section(&section, IVec3::new(-16, 32, 48), air, AO);
        let mut flipped = 0;
        for (q, quad) in mesh.vertices.chunks(4).enumerate() {
            let normal = quad_normal(quad).as_vec3();
            let first = (q * 4) as u16;
            let indices = &mesh.indices[q * 6..q * 6 + 6];
            if indices[0] != first {
                flipped += 1;
            }
            for tri in indices.chunks(3) {
                let [a, b, c] =
                    [0, 1, 2].map(|i| Vec3::from(mesh.vertices[usize::from(tri[i])].position));
                let winding = (b - a).cross(c - b).normalize();
                assert!(
                    winding.dot(normal) > 0.999,
                    "quad {q}: {winding} vs {normal}"
                );
            }
        }
        assert!(flipped > 0, "no flipped quads to check");
        assert_ao_matches(
            &mesh,
            |p| in_section(&section)(p - IVec3::new(-16, 32, 48)),
            |_| true,
        );
    }

    #[test]
    fn no_ao_keeps_plain_side_shading() {
        use crate::terrain::Terrain;

        let column = Terrain::new(7).generate(IVec2::new(3, -2));
        let origin = IVec3::new(48, 0, -32);
        let with = mesh_column(&column, origin, air, AO);
        let without = mesh_column(&column, origin, air, NO_AO);
        for (with, without) in with.iter().zip(&without) {
            assert_eq!(with.vertices.len(), without.vertices.len());
            for (q, quad) in without.vertices.chunks(4).enumerate() {
                let shade = face_shade(quad_normal(quad));
                assert!(quad.iter().all(|v| v.shade == shade), "{quad:?}");
                let first = (q * 4) as u16;
                assert_eq!(
                    without.indices[q * 6..q * 6 + 6],
                    [0, 1, 2, 0, 2, 3].map(|i| first + i)
                );
            }
            // Кроме яркости, вершины те же.
            for (a, b) in with.vertices.iter().zip(&without.vertices) {
                assert_eq!((a.position, a.uv, a.layer), (b.position, b.uv, b.layer));
            }
        }
    }

    #[test]
    fn no_seams_between_columns() {
        use crate::column::WORLD_HEIGHT;
        use crate::terrain::Terrain;

        // Центр с отрицательными координатами, чтобы задеть и div_euclid на стыках.
        let center = IVec2::new(-1, 2);
        let terrain = Terrain::new(1337);
        let mut world = World::new();
        for z in -1..=1 {
            for x in -1..=1 {
                let key = center + IVec2::new(x, z);
                world.insert(key, terrain.generate(key));
            }
        }

        let origin = IVec3::new(center.x * SECTION_SIZE, 0, center.y * SECTION_SIZE);
        let column = world.column(center).unwrap();
        let meshes = mesh_column(column, origin, |local| world.block(origin + local), AO);
        assert_eq!(meshes.len(), COLUMN_SECTIONS as usize);
        let meshed: usize = meshes.iter().map(face_count).sum();

        // Те же грани, посчитанные по миру целиком, без деления на колонки и секции.
        let mut expected = 0;
        for y in 0..WORLD_HEIGHT {
            for z in 0..SECTION_SIZE {
                for x in 0..SECTION_SIZE {
                    let pos = origin + IVec3::new(x, y, z);
                    if world.block(pos).is_solid() {
                        expected += FACES
                            .iter()
                            .filter(|face| !world.block(pos + face.normal).is_solid())
                            .count();
                    }
                }
            }
        }
        assert!(expected > 256, "flat terrain would not test side faces");
        assert_eq!(meshed, expected);

        // AO по миру целиком; отдельно считаем затенённые вершины на рёбрах колонки
        // (вертикальных, где сходятся четыре колонки) и на стыках секций, чтобы тест
        // точно задевал соседей по диагонали.
        let min = origin.as_vec3();
        let on_edge = |p: Vec3| {
            let local = p - min;
            [local.x, local.z].iter().all(|&c| c == 0.0 || c == 16.0)
        };
        let on_section_seam = |p: Vec3| {
            let local = p - min;
            (local.x == 0.0 || local.x == 16.0 || local.z == 0.0 || local.z == 16.0)
                && local.y % 16.0 == 0.0
        };
        let block_at = |p| world.block(p);
        let edge: usize = meshes
            .iter()
            .map(|mesh| assert_ao_matches(mesh, block_at, on_edge))
            .sum();
        let seam: usize = meshes
            .iter()
            .map(|mesh| assert_ao_matches(mesh, block_at, on_section_seam))
            .sum();
        assert!(edge > 0 && seam > 0, "edge {edge}, seam {seam}");
    }

    /// Меш секции `key` с соседями из `world`, в сравнимом виде.
    fn world_mesh(world: &World, key: IVec3) -> (Vec<Vertex>, Vec<u16>) {
        let mesh = mesh_world_section(world, key, AO).unwrap();
        (mesh.vertices, mesh.indices)
    }

    #[test]
    fn edits_on_seams_remesh_exactly_affected_sections() {
        use crate::terrain::Terrain;
        use crate::world::affected_sections;

        let terrain = Terrain::new(1337);
        let mut world = World::new();
        for z in -2..=1 {
            for x in -2..=1 {
                let key = IVec2::new(x, z);
                world.insert(key, terrain.generate(key));
            }
        }
        // Секции колонок -1..=0, у которых загружены все 8 соседних колонок.
        let sections: Vec<IVec3> = (-1..=0)
            .flat_map(|z| {
                (-1..=0).flat_map(move |x| (0..COLUMN_SECTIONS).map(move |sy| IVec3::new(x, sy, z)))
            })
            .collect();

        let surface = terrain.surface_height(-1, 0);
        let corner_surface = terrain.surface_height(-1, -1);
        // Угол колонок 0/-1 по x и секций по высоте: под землёй, на стыке трёх секций.
        let underground = IVec3::new(-1, 32, 0);
        assert!(surface > 40, "edit must be underground, surface {surface}");
        let edits = [
            (underground, Block::Air),
            // Над поверхностью, на краю колонки по z и на стыке секций сверху.
            (IVec3::new(-16, 79, -1), Block::Stone),
            // Ставим обратно рядом с выкопанным: грани должны исчезнуть.
            (underground, Block::Dirt),
            (
                IVec3::new(0, terrain.surface_height(0, -16), -16),
                Block::Air,
            ),
            // На поверхности в углу четырёх колонок: затенение у диагональных соседей.
            (IVec3::new(-1, corner_surface + 1, -1), Block::Stone),
            (IVec3::new(0, corner_surface + 1, 0), Block::Stone),
            (IVec3::new(-1, corner_surface + 1, -1), Block::Air),
        ];
        for (pos, block) in edits {
            let before: Vec<_> = sections.iter().map(|&k| world_mesh(&world, k)).collect();
            assert!(world.set_block(pos, block));
            let affected = affected_sections(pos);
            let mut changed = 0;
            for (&key, before) in sections.iter().zip(before) {
                if world_mesh(&world, key) != before {
                    changed += 1;
                    assert!(
                        affected.contains(&key),
                        "{pos}: section {key} changed but not affected"
                    );
                }
            }
            assert!(changed > 0, "{pos}: edit changed nothing");
        }
    }

    #[test]
    fn world_section_outside_is_none() {
        let mut world = World::new();
        world.insert(IVec2::ZERO, Column::new());
        assert!(mesh_world_section(&world, IVec3::new(0, 0, 0), AO).is_some());
        assert!(mesh_world_section(&world, IVec3::new(0, COLUMN_SECTIONS, 0), AO).is_none());
        assert!(mesh_world_section(&world, IVec3::new(0, -1, 0), AO).is_none());
        assert!(mesh_world_section(&world, IVec3::new(1, 0, 0), AO).is_none());
    }
}
