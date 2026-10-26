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

/// Строит меш видимых граней секции: грань рисуется, если блок твёрдый,
/// а сосед в её направлении — нет. Позиции вершин мировые.
///
/// `origin` — мировые координаты минимального угла секции в блоках (координаты секции × 16).
/// `neighbor` вызывается только для локальных координат за краем секции
/// (одна компонента равна -1 или 16) и возвращает блок там.
pub fn mesh_section(
    section: &Section,
    origin: IVec3,
    neighbor: impl Fn(IVec3) -> Block,
) -> MeshData {
    let mut mesh = MeshData {
        vertices: Vec::new(),
        indices: Vec::new(),
    };
    let block_at = |local: IVec3| {
        if Section::contains(local) {
            section.get(local)
        } else {
            neighbor(local)
        }
    };

    for y in 0..SECTION_SIZE {
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                let local = IVec3::new(x, y, z);
                let block = section.get(local);
                if !block.is_solid() {
                    continue;
                }
                let base = (origin + local).as_vec3();
                for face in &FACES {
                    if block_at(local + face.normal).is_solid() {
                        continue;
                    }
                    let Some(tile) = block.tile(face.facing) else {
                        continue;
                    };
                    // Худший случай — шахматная расстановка: 2048 блоков × 24 = 49 152 вершины.
                    debug_assert!(
                        mesh.vertices.len() + 4 <= usize::from(u16::MAX) + 1,
                        "section mesh overflows u16 indices"
                    );
                    let first = mesh.vertices.len() as u16;
                    mesh.vertices
                        .extend(
                            face.corners
                                .iter()
                                .zip(CORNER_UVS)
                                .map(|(&corner, uv)| Vertex {
                                    position: (base + glam::Vec3::from(corner)).to_array(),
                                    uv,
                                    shade: face.shade,
                                    layer: tile.layer(),
                                }),
                        );
                    mesh.indices
                        .extend([first, first + 1, first + 2, first, first + 2, first + 3]);
                }
            }
        }
    }
    mesh
}

/// Меши всех секций колонки, снизу вверх (`[0]` — y 0..16).
///
/// `origin` — мировые координаты минимального угла колонки (y = 0). Соседей по
/// вертикали внутри колонки берёт из неё самой; `neighbor` вызывается только для
/// локальных координат колонки за её краем (x или z равен -1 или 16, y равен -1 или 128).
pub fn mesh_column(
    column: &Column,
    origin: IVec3,
    neighbor: impl Fn(IVec3) -> Block,
) -> Vec<MeshData> {
    (0..COLUMN_SECTIONS)
        .map(|sy| {
            let offset = IVec3::new(0, sy * SECTION_SIZE, 0);
            mesh_section(column.section(sy), origin + offset, |local| {
                let in_column = local + offset;
                if Column::contains(in_column) {
                    column.get(in_column)
                } else {
                    neighbor(in_column)
                }
            })
        })
        .collect()
}

/// Меш одной секции по ключу `(x колонки, номер секции 0..8, z колонки)` с соседями
/// из `world` — для перестройки после правки блока. `None`, если колонка не загружена
/// или номер секции вне 0..COLUMN_SECTIONS.
pub fn mesh_world_section(world: &World, key: IVec3) -> Option<MeshData> {
    if !(0..COLUMN_SECTIONS).contains(&key.y) {
        return None;
    }
    let column = world.column(IVec2::new(key.x, key.z))?;
    let origin = key * SECTION_SIZE;
    Some(mesh_section(column.section(key.y), origin, |local| {
        world.block(origin + local)
    }))
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::*;
    use crate::texture::Tile;

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
        let mesh = mesh_section(&Section::new(), IVec3::ZERO, air);
        assert_eq!(face_count(&mesh), 0);
        assert!(mesh.vertices.is_empty());
    }

    #[test]
    fn single_block_has_six_faces() {
        let mut section = Section::new();
        section.set(IVec3::new(5, 5, 5), Block::Dirt);
        let mesh = mesh_section(&section, IVec3::ZERO, air);
        assert_eq!(face_count(&mesh), 6);
        assert_eq!(mesh.vertices.len(), 24);
    }

    #[test]
    fn vertices_are_in_world_space() {
        let mut section = Section::new();
        section.set(IVec3::new(1, 2, 3), Block::Dirt);
        let origin = IVec3::new(16, 0, -32);
        let mesh = mesh_section(&section, origin, air);
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
        let mesh = mesh_section(&section, IVec3::ZERO, air);
        assert_eq!(face_count(&mesh), 10);
    }

    #[test]
    fn solid_section_draws_only_outer_shell() {
        let mesh = mesh_section(&filled(|_| true), IVec3::ZERO, air);
        assert_eq!(face_count(&mesh), 6 * 256);
    }

    #[test]
    fn solid_section_with_solid_neighbors_has_no_faces() {
        let mesh = mesh_section(&filled(|_| true), IVec3::ZERO, |local| {
            let outside = local
                .to_array()
                .iter()
                .filter(|&&c| c == -1 || c == 16)
                .count();
            let inside = local
                .to_array()
                .iter()
                .filter(|&&c| (0..16).contains(&c))
                .count();
            assert_eq!((outside, inside), (1, 2), "neighbor called for {local}");
            Block::Stone
        });
        assert_eq!(face_count(&mesh), 0);
    }

    #[test]
    fn checkerboard_fits_u16() {
        let section = filled(|p| (p.x + p.y + p.z) % 2 == 0);
        let mesh = mesh_section(&section, IVec3::ZERO, air);
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
        let mesh = mesh_section(&section, IVec3::ZERO, air);
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
        let mesh = mesh_section(&section, IVec3::ZERO, air);
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
        let mesh = mesh_section(&section, IVec3::ZERO, air);
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

    #[test]
    fn no_faces_between_columns() {
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
        let meshes = mesh_column(column, origin, |local| world.block(origin + local));
        assert_eq!(meshes.len(), COLUMN_SECTIONS as usize);
        let meshed: usize = meshes.iter().map(face_count).sum();

        // Те же грани, посчитанные по миру целиком, без деления на колонки и секции.
        let dirs = [
            IVec3::X,
            IVec3::NEG_X,
            IVec3::Y,
            IVec3::NEG_Y,
            IVec3::Z,
            IVec3::NEG_Z,
        ];
        let mut expected = 0;
        for y in 0..WORLD_HEIGHT {
            for z in 0..SECTION_SIZE {
                for x in 0..SECTION_SIZE {
                    let pos = origin + IVec3::new(x, y, z);
                    if world.block(pos).is_solid() {
                        expected += dirs
                            .iter()
                            .filter(|&&d| !world.block(pos + d).is_solid())
                            .count();
                    }
                }
            }
        }
        assert!(expected > 256, "flat terrain would not test side faces");
        assert_eq!(meshed, expected);
    }

    /// Грани в секции `key`, посчитанные по миру напрямую, без мешера.
    fn world_faces(world: &World, key: IVec3) -> usize {
        let origin = key * SECTION_SIZE;
        let mut count = 0;
        for y in 0..SECTION_SIZE {
            for z in 0..SECTION_SIZE {
                for x in 0..SECTION_SIZE {
                    let pos = origin + IVec3::new(x, y, z);
                    if world.block(pos).is_solid() {
                        count += FACES
                            .iter()
                            .filter(|face| !world.block(pos + face.normal).is_solid())
                            .count();
                    }
                }
            }
        }
        count
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
        // Секции колонок -1..=0, чьи соседи по граням все загружены.
        let sections: Vec<IVec3> = (-1..=0)
            .flat_map(|z| {
                (-1..=0).flat_map(move |x| (0..COLUMN_SECTIONS).map(move |sy| IVec3::new(x, sy, z)))
            })
            .collect();

        let surface = terrain.surface_height(-1, 0);
        // Угол колонок 0/-1 по x и секций по высоте: под землёй, на стыке трёх секций.
        let underground = IVec3::new(-1, 32, 0);
        assert!(surface > 40, "edit must be underground, surface {surface}");
        let edits = [
            (underground, Block::Air),
            // Над поверхностью, на краю колонки по z и на стыке секций сверху.
            (IVec3::new(-16, 79, -1), Block::Stone),
            // Ставим обратно рядом с выкопанным: грани должны исчезнуть.
            (underground, Block::Dirt),
            (IVec3::new(0, surface, -16), Block::Air),
        ];
        for (pos, block) in edits {
            let before: Vec<usize> = sections.iter().map(|&k| world_faces(&world, k)).collect();
            assert!(world.set_block(pos, block));
            let affected = affected_sections(pos);
            for (&key, before) in sections.iter().zip(before) {
                let after = world_faces(&world, key);
                if affected.contains(&key) {
                    let mesh = mesh_world_section(&world, key).unwrap();
                    assert_eq!(face_count(&mesh), after, "{pos}: section {key}");
                } else {
                    assert_eq!(
                        after, before,
                        "{pos}: section {key} changed but not affected"
                    );
                }
            }
        }
    }

    #[test]
    fn world_section_outside_is_none() {
        let mut world = World::new();
        world.insert(IVec2::ZERO, Column::new());
        assert!(mesh_world_section(&world, IVec3::new(0, 0, 0)).is_some());
        assert!(mesh_world_section(&world, IVec3::new(0, COLUMN_SECTIONS, 0)).is_none());
        assert!(mesh_world_section(&world, IVec3::new(0, -1, 0)).is_none());
        assert!(mesh_world_section(&world, IVec3::new(1, 0, 0)).is_none());
    }
}
