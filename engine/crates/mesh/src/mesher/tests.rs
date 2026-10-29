use glam::{IVec2, Vec3};

use world::block::WATER_SURFACE;
use world::light::MAX_LIGHT;

use super::expand::{Vertex, indices, vertices};
use super::*;

const AO: MeshOptions = MeshOptions {
    ambient_occlusion: true,
};
const NO_AO: MeshOptions = MeshOptions {
    ambient_occlusion: false,
};

/// Наибольшая ошибка яркости от квантования в байт.
const QUANT: f32 = 0.5 / 255.0 + 1e-6;

fn assert_near(actual: [f32; 4], expected: [f32; 4]) {
    for (a, e) in actual.into_iter().zip(expected) {
        assert!((a - e).abs() <= QUANT, "{actual:?} vs {expected:?}");
    }
}

fn air(_: IVec3) -> Block {
    Block::Air
}

/// Всё освещено полностью: яркость вершин — только сторона и AO.
fn full(_: IVec3) -> u8 {
    MAX_LIGHT
}

/// Свет мира, где загружены все колонки `world`.
fn world_light(world: &World) -> LightMap {
    let mut light = LightMap::new();
    for key in world.keys().collect::<Vec<_>>() {
        light.add_column(world, key);
    }
    light
}

fn face_count(mesh: &MeshData) -> usize {
    indices(mesh).len() / 6
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
    let mesh = mesh_section(&Section::new(), air, full, AO).solid;
    assert_eq!(face_count(&mesh), 0);
    assert!(vertices(&mesh).is_empty());
}

#[test]
fn single_block_has_six_faces() {
    let mut section = Section::new();
    section.set(IVec3::new(5, 5, 5), Block::Dirt);
    let mesh = mesh_section(&section, air, full, AO).solid;
    assert_eq!(face_count(&mesh), 6);
    assert_eq!(vertices(&mesh).len(), 24);
}

#[test]
fn vertices_are_local_to_section() {
    let mut section = Section::new();
    section.set(IVec3::new(1, 2, 3), Block::Dirt);
    let mesh = mesh_section(&section, air, full, AO).solid;
    let min = Vec3::new(1.0, 2.0, 3.0);
    for v in &vertices(&mesh) {
        let p = Vec3::from(v.position());
        assert!(p.cmpge(min).all() && p.cmple(min + Vec3::ONE).all(), "{p}");
    }
}

#[test]
fn adjacent_blocks_share_hidden_face() {
    let mut section = Section::new();
    section.set(IVec3::new(5, 5, 5), Block::Dirt);
    section.set(IVec3::new(6, 5, 5), Block::Dirt);
    let mesh = mesh_section(&section, air, full, AO).solid;
    assert_eq!(face_count(&mesh), 10);
}

#[test]
fn solid_section_draws_only_outer_shell() {
    let mesh = mesh_section(&filled(|_| true), air, full, AO).solid;
    assert_eq!(face_count(&mesh), 6 * 256);
}

#[test]
fn solid_section_with_solid_neighbors_has_no_faces() {
    let mesh = mesh_section(
        &filled(|_| true),
        |local| {
            assert!(!Section::contains(local), "neighbor called for {local}");
            assert!(
                local.cmpge(IVec3::NEG_ONE).all() && local.cmple(IVec3::splat(16)).all(),
                "neighbor called for {local}"
            );
            Block::Stone
        },
        |local| {
            assert!(
                local.cmpge(IVec3::NEG_ONE).all() && local.cmple(IVec3::splat(16)).all(),
                "light asked for {local}"
            );
            MAX_LIGHT
        },
        AO,
    )
    .solid;
    assert_eq!(face_count(&mesh), 0);
}

#[test]
fn air_section_does_not_ask_neighbors() {
    let mesh = mesh_section(
        &Section::new(),
        |local| panic!("neighbor called for {local}"),
        |local| panic!("light asked for {local}"),
        AO,
    )
    .solid;
    assert!(vertices(&mesh).is_empty());
}

#[test]
fn checkerboard_fits_u16() {
    let section = filled(|p| (p.x + p.y + p.z) % 2 == 0);
    let mesh = mesh_section(&section, air, full, AO).solid;
    assert_eq!(face_count(&mesh), 2048 * 6);
    assert_eq!(vertices(&mesh).len(), 2048 * 24);
    // Индексы развёртки (4 вершины и 6 индексов на квад) не выходят за вершины.
    let count = vertices(&mesh).len();
    assert!(indices(&mesh).iter().all(|&i| usize::from(i) < count));
}

#[test]
fn grass_faces_use_their_tiles() {
    let mut section = Section::new();
    section.set(IVec3::ZERO, Block::Grass);
    let mesh = mesh_section(&section, air, full, AO).solid;
    for quad in vertices(&mesh).chunks(4) {
        let ys: Vec<f32> = quad.iter().map(|v| v.position()[1]).collect();
        let expected = if ys.iter().all(|&y| y == 1.0) {
            Tile::GrassTop
        } else if ys.iter().all(|&y| y == 0.0) {
            Tile::Dirt
        } else {
            Tile::GrassSide
        };
        assert!(quad.iter().all(|v| v.layer() == expected.layer() as u8));
    }
}

#[test]
fn side_faces_are_upright() {
    let mut section = Section::new();
    section.set(IVec3::ZERO, Block::Grass);
    let mesh = mesh_section(&section, air, full, AO).solid;
    let verts = vertices(&mesh);
    let sides = verts.chunks(4).filter(|quad| {
        quad.iter()
            .any(|v| v.position()[1] != quad[0].position()[1])
    });
    let mut count = 0;
    for quad in sides {
        for v in quad {
            assert_eq!(
                v.uv()[1],
                1.0 - v.position()[1],
                "v = 0 must be the top edge"
            );
        }
        count += 1;
    }
    assert_eq!(count, 4);
}

#[test]
fn faces_wind_counter_clockwise_from_outside() {
    let mut section = Section::new();
    section.set(IVec3::ZERO, Block::Grass);
    let mesh = mesh_section(&section, air, full, AO).solid;
    let center = Vec3::splat(0.5);
    let mut normals = Vec::new();
    for quad in vertices(&mesh).chunks(4) {
        let [a, b, c] = [0, 1, 2].map(|i| Vec3::from(quad[i].position()));
        let winding = (b - a).cross(c - b).normalize();
        // Сторону грани берём из геометрии: от центра блока к центру грани.
        let face_center = quad.iter().map(|v| Vec3::from(v.position())).sum::<Vec3>() / 4.0;
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
    let [a, b, c] = [0, 1, 2].map(|i| Vec3::from(quad[i].position()));
    (b - a).cross(c - b).normalize().round().as_ivec3()
}

/// Поля света вершин квада, посчитанные по миру напрямую: блок за гранью, три блока
/// перед каждой вершиной (AO) и свет клетки перед гранью и этих трёх клеток.
/// Возвращает на вершину `(сторона × AO, солнце, блоки)`; свет — в половинках уровня.
fn world_shades(
    origin: IVec3,
    block_at: impl Fn(IVec3) -> Block,
    light_at: impl Fn(IVec3) -> u8,
    quad: &[Vertex],
) -> [(f32, u8, u8); 4] {
    let normal = quad_normal(quad);
    let world = |v: &Vertex| origin.as_vec3() + Vec3::from(v.position());
    let center = quad.iter().map(world).sum::<Vec3>() / 4.0;
    let block = (center - normal.as_vec3() * 0.5).floor().as_ivec3();
    let mid = block.as_vec3() + 0.5;
    std::array::from_fn(|i| {
        let toward = (world(&quad[i]) - mid).signum().as_ivec3() * (IVec3::ONE - normal.abs());
        let axes: Vec<IVec3> = (0..3)
            .filter(|&a| toward[a] != 0)
            .map(|a| {
                let mut step = IVec3::ZERO;
                step[a] = toward[a];
                step
            })
            .collect();
        let front = block + normal;
        let cells = [front + axes[0], front + axes[1], front + toward];
        let [o1, o2, oc] = cells.map(|c| block_at(c).is_opaque());
        let ao = vertex_ao(o1, o2, oc);
        let mut levels = vec![light_at(front)];
        for (cell, open) in cells.into_iter().zip([!o1, !o2, !oc && !(o1 && o2)]) {
            if open {
                levels.push(light_at(cell));
            }
        }
        let count = levels.len() as f32;
        let halves = |channel: fn(u8) -> u8| {
            let mean = levels.iter().map(|&l| f32::from(channel(l))).sum::<f32>() / count;
            (mean * 2.0 + 0.5).floor() as u8
        };
        (
            face_shade(normal) * AO_BRIGHTNESS[usize::from(ao)],
            halves(sky_level),
            halves(block_level),
        )
    })
}

fn face_shade(normal: IVec3) -> f32 {
    FACES.iter().find(|f| f.normal == normal).unwrap().shade
}

/// Каждый квад меша: яркость вершин и выбор диагонали совпадают с расчётом по
/// `block_at` и `light_at`. Возвращает число затенённых (ярче нет) вершин, где
/// `pred` от позиции истинен.
fn assert_shades_match(
    mesh: &MeshData,
    origin: IVec3,
    block_at: impl Fn(IVec3) -> Block,
    light_at: impl Fn(IVec3) -> u8,
    pred: impl Fn(Vec3) -> bool,
) -> usize {
    let mut shaded = 0;
    for (q, quad) in vertices(mesh).chunks(4).enumerate() {
        let expected = world_shades(origin, &block_at, &light_at, quad);
        let side = face_shade(quad_normal(quad));
        for (v, (shade, sun, block)) in quad.iter().zip(expected) {
            assert_eq!(v.shade_byte(), quantize_shade(shade), "{v:?}");
            assert_eq!((v.sun_halves(), v.block_halves()), (sun, block), "{v:?}");
            if v.brightness(0.0) < side - 1e-3 && pred(origin.as_vec3() + Vec3::from(v.position()))
            {
                shaded += 1;
            }
        }
        let days = expected.map(|(shade, sun, block)| day_shade(quantize_shade(shade), sun, block));
        assert_eq!(mesh.quads[q].flipped(), flip_diagonal(days), "quad {q}");
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
    let q = vertices(mesh)
        .chunks(4)
        .position(|quad| {
            quad_normal(quad) == IVec3::Y
                && quad[3].position() == (block + IVec3::Y).as_vec3().to_array()
        })
        .unwrap_or_else(|| panic!("no top face of {block}"));
    let first = (q * 4) as u16;
    let indices = std::array::from_fn(|i| indices(mesh)[q * 6 + i] - first);
    (
        vertices(mesh)[q * 4..q * 4 + 4].try_into().unwrap(),
        indices,
    )
}

fn shades(quad: &[Vertex; 4]) -> [f32; 4] {
    quad.map(|v| v.brightness(0.0))
}

/// Секция с двумя блоками рядом по x: `a` в (5, 5, 5), `b` в (6, 5, 5).
fn pair(a: Block, b: Block) -> MeshData {
    let mut section = Section::new();
    section.set(IVec3::new(5, 5, 5), a);
    section.set(IVec3::new(6, 5, 5), b);
    let meshes = mesh_section(&section, air, full, AO);
    merged(&meshes)
}

/// Твёрдое и вырезанное одним мешом: тесты граней не различают слой.
fn merged(meshes: &SectionMeshes) -> MeshData {
    let mut all = MeshData::new();
    all.quads.extend(&meshes.solid.quads);
    all.quads.extend(&meshes.cutout.quads);
    all
}

/// Есть ли в меше грань с нормалью `normal` в плоскости x = `x`.
fn has_face_at_x(mesh: &MeshData, x: f32, normal: IVec3) -> bool {
    vertices(mesh)
        .chunks(4)
        .any(|quad| quad_normal(quad) == normal && quad.iter().all(|v| v.position()[0] == x))
}

#[test]
fn stone_face_behind_leaves_is_drawn() {
    let mesh = pair(Block::Stone, Block::Leaves);
    assert!(
        has_face_at_x(&mesh, 6.0, IVec3::X),
        "stone face toward leaves"
    );
    assert!(
        !has_face_at_x(&mesh, 6.0, IVec3::NEG_X),
        "leaves face toward stone"
    );
    assert_eq!(face_count(&mesh), 6 + 5);
}

#[test]
fn leaves_next_to_leaves_draw_both_faces() {
    let mesh = pair(Block::Leaves, Block::Leaves);
    assert!(has_face_at_x(&mesh, 6.0, IVec3::X));
    assert!(has_face_at_x(&mesh, 6.0, IVec3::NEG_X));
    assert_eq!(face_count(&mesh), 12);
}

#[test]
fn leaves_do_not_shade_corners() {
    // Как в `open_floor_and_block_tops_are_unshaded`, но рядом с полом листва.
    let block = IVec3::new(8, 1, 8);
    let mut section = floor_with(&[]);
    section.set(block, Block::Leaves);
    let mesh = mesh_section(&section, air, full, AO).solid;
    let (beside, _) = top_face(&mesh, IVec3::new(7, 0, 8));
    assert_near(shades(&beside), [1.0; 4]);
    // А пол под листвой (верхняя грань пола видна сквозь неё) — тоже без затенения.
    assert_near(shades(&top_face(&mesh, IVec3::new(8, 0, 8)).0), [1.0; 4]);
    assert_shades_match(&mesh, IVec3::ZERO, in_section(&section), full, |_| true);
}

#[test]
fn open_floor_and_block_tops_are_unshaded() {
    let block = IVec3::new(8, 1, 8);
    let mesh = mesh_section(&floor_with(&[block]), air, full, AO).solid;
    assert_near(shades(&top_face(&mesh, block).0), [1.0; 4]);
    assert_near(shades(&top_face(&mesh, IVec3::new(4, 0, 4)).0), [1.0; 4]);
    // Пол у подножия блока со стороны −x: у ребра к блоку один боковой сосед.
    let (beside, _) = top_face(&mesh, IVec3::new(7, 0, 8));
    assert_near(shades(&beside), [1.0, 0.85, 0.85, 1.0]);
}

#[test]
fn inner_corner_is_darkest() {
    // Стены вдоль x = 0 и z = 0 высотой в два блока; пол в углу — блок (1, 0, 1).
    let walls: Vec<IVec3> = (1..3)
        .flat_map(|y| (0..16).flat_map(move |i| [IVec3::new(0, y, i), IVec3::new(i, y, 0)]))
        .collect();
    let section = floor_with(&walls);
    let mesh = mesh_section(&section, air, full, AO).solid;
    let (quad, indices) = top_face(&mesh, IVec3::new(1, 0, 1));
    // Углы верхней грани: (1, 2), (2, 2), (2, 1), (1, 1) по x, z.
    assert_near(shades(&quad), [0.7, 1.0, 0.7, 0.5]);
    // Диагональ 0–2 темнее (1 + 1 против 3 + 0) — режем по 1–3.
    assert_eq!(indices, [1, 2, 3, 1, 3, 0]);
    assert!(assert_shades_match(&mesh, IVec3::ZERO, in_section(&section), full, |_| true) > 0);
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
        let mesh = mesh_section(&floor_with(&[block]), air, full, AO).solid;
        let (quad, indices) = top_face(&mesh, floor);
        let mut expected = [1.0; 4];
        expected[dark] = 0.85;
        assert_near(shades(&quad), expected);
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
    let mesh = mesh_section(&section, air, full, AO).solid;
    let mut flipped = 0;
    let (verts, order) = (vertices(&mesh), indices(&mesh));
    for (q, quad) in verts.chunks(4).enumerate() {
        let normal = quad_normal(quad).as_vec3();
        let first = (q * 4) as u16;
        let indices = &order[q * 6..q * 6 + 6];
        if indices[0] != first {
            flipped += 1;
        }
        for tri in indices.chunks(3) {
            let [a, b, c] = [0, 1, 2].map(|i| Vec3::from(verts[usize::from(tri[i])].position()));
            let winding = (b - a).cross(c - b).normalize();
            assert!(
                winding.dot(normal) > 0.999,
                "quad {q}: {winding} vs {normal}"
            );
        }
    }
    assert!(flipped > 0, "no flipped quads to check");
    assert_shades_match(&mesh, IVec3::ZERO, in_section(&section), full, |_| true);
}

#[test]
fn no_ao_keeps_plain_side_shading() {
    use world::terrain::Terrain;

    let column = Terrain::new(7).generate_trees(ColumnPos::new(3, -2));
    let with = mesh_column(&column, air, full, AO);
    let without = mesh_column(&column, air, full, NO_AO);
    for (with, without) in with.iter().zip(&without) {
        assert_eq!(vertices(&with.water), vertices(&without.water));
        let (with, without) = (&with.solid, &without.solid);
        assert_eq!(vertices(with).len(), vertices(without).len());
        let order = indices(without);
        for (q, quad) in vertices(without).chunks(4).enumerate() {
            let shade = face_shade(quad_normal(quad));
            assert!(
                quad.iter().all(|v| v.shade_byte() == quantize_shade(shade)),
                "{quad:?}"
            );
            let first = (q * 4) as u16;
            assert_eq!(
                order[q * 6..q * 6 + 6],
                [0, 1, 2, 0, 2, 3].map(|i| first + i)
            );
        }
        // Кроме яркости, вершины те же.
        for (a, b) in vertices(with).iter().zip(&vertices(without)) {
            assert_eq!(
                (a.position(), a.uv(), a.layer()),
                (b.position(), b.uv(), b.layer())
            );
        }
    }
}

/// Видимые грани колонки `key` (`solid`, `water`), посчитанные по миру целиком, без
/// деления на колонки и секции: у воды — к воздуху, у остальных блоков — к не
/// непрозрачному соседу.
fn world_face_count(world: &World, key: ColumnPos) -> (usize, usize) {
    use world::column::WORLD_HEIGHT;

    let origin = key.origin();
    let (mut solid, mut water) = (0, 0);
    for y in 0..WORLD_HEIGHT {
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                let pos = origin + IVec3::new(x, y, z);
                let block = world.block(pos);
                let neighbors = FACES.iter().map(|face| world.block(pos + face.normal));
                if block.is_water() {
                    water += neighbors.filter(|&n| n == Block::Air).count();
                } else if block != Block::Air {
                    solid += neighbors.filter(|n| !n.is_opaque()).count();
                }
            }
        }
    }
    (solid, water)
}

/// Мировой угол секции `sy` колонки с углом `origin`.
fn section_origin(origin: IVec3, sy: usize) -> IVec3 {
    origin + IVec3::Y * (sy as i32 * SECTION_SIZE)
}

/// Грани всех секций колонки (`solid`, `water`).
fn column_face_count(meshes: &[SectionMeshes]) -> (usize, usize) {
    meshes.iter().fold((0, 0), |(solid, water), m| {
        (
            solid + face_count(&m.solid) + face_count(&m.cutout),
            water + face_count(&m.water),
        )
    })
}

#[test]
fn no_seams_between_columns() {
    use world::terrain::Terrain;

    // Центр с отрицательными координатами, чтобы задеть и div_euclid на стыках.
    let center = ColumnPos::new(-1, 2);
    let terrain = Terrain::new(1337);
    let mut world = World::new();
    for z in -1..=1 {
        for x in -1..=1 {
            let key = center + IVec2::new(x, z);
            world.insert(key, terrain.generate_trees(key));
        }
    }

    let light = world_light(&world);
    let origin = center.origin();
    let meshes = mesh_world_column(&world, &light, center, AO).unwrap();
    assert_eq!(meshes.len(), COLUMN_SECTIONS as usize);
    let expected = world_face_count(&world, center);
    assert!(expected.0 > 256, "flat terrain would not test side faces");
    assert_eq!(column_face_count(&meshes), expected);

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
    let light_at = |p| light.packed(p);
    let edge: usize = meshes
        .iter()
        .enumerate()
        .map(|(sy, mesh)| {
            assert_shades_match(
                &mesh.solid,
                section_origin(origin, sy),
                block_at,
                light_at,
                on_edge,
            )
        })
        .sum();
    let seam: usize = meshes
        .iter()
        .enumerate()
        .map(|(sy, mesh)| {
            let at = section_origin(origin, sy);
            assert_shades_match(&mesh.solid, at, block_at, light_at, on_section_seam)
        })
        .sum();
    assert!(edge > 0 && seam > 0, "edge {edge}, seam {seam}");
}

#[test]
fn tree_on_column_border_matches_whole_world() {
    use world::terrain::Terrain;
    use world::trees::border_tree;

    let terrain = Terrain::new(1337);
    let tree = border_tree(&terrain);
    let trunk = ColumnPos::of_block(tree.base);
    let mut world = World::new();
    for z in -2..=2 {
        for x in -2..=2 {
            let key = trunk + IVec2::new(x, z);
            world.insert(key, terrain.generate_trees(key));
        }
    }
    // Колонка ствола и все соседи, куда заходит крона.
    let mut keys: Vec<ColumnPos> = tree.leaves().map(ColumnPos::of_block).collect();
    keys.sort();
    keys.dedup();
    assert!(keys.len() > 1);
    let light = world_light(&world);
    for key in keys {
        let origin = key.origin();
        let column = world.column(key).unwrap();
        assert!(
            (0..COLUMN_SECTIONS).any(|sy| {
                let section = column.section(sy);
                (0..16 * 16 * 16)
                    .any(|i| section.get(IVec3::new(i % 16, i / 256, i / 16 % 16)) == Block::Leaves)
            }),
            "no leaves in {key}"
        );
        let meshes = mesh_world_column(&world, &light, key, AO).unwrap();
        assert_eq!(
            column_face_count(&meshes),
            world_face_count(&world, key),
            "{key}"
        );
        for (sy, mesh) in meshes.iter().enumerate() {
            assert_shades_match(
                &mesh.solid,
                section_origin(origin, sy),
                |p| world.block(p),
                |p| light.packed(p),
                |_| true,
            );
        }
    }
}

/// Меши секции `key` с соседями из `world`, в сравнимом виде.
fn world_mesh(world: &World, light: &LightMap, key: SectionPos) -> [(Vec<Vertex>, Vec<u16>); 3] {
    let SectionMeshes {
        solid,
        cutout,
        water,
    } = mesh_world_section(world, light, key, AO).unwrap();
    [
        (vertices(&solid), indices(&solid)),
        (vertices(&cutout), indices(&cutout)),
        (vertices(&water), indices(&water)),
    ]
}

#[test]
fn edits_on_seams_remesh_exactly_affected_sections() {
    use world::terrain::Terrain;
    use world::world::affected_sections;

    let terrain = Terrain::new(1337);
    let mut world = World::new();
    for z in -2..=1 {
        for x in -2..=1 {
            let key = ColumnPos::new(x, z);
            world.insert(key, terrain.generate_trees(key));
        }
    }
    let mut light = world_light(&world);
    // Секции колонок -1..=0, у которых загружены все 8 соседних колонок.
    let sections: Vec<SectionPos> = (-1..=0)
        .flat_map(|z| {
            (-1..=0)
                .flat_map(move |x| (0..COLUMN_SECTIONS).map(move |sy| SectionPos::new(x, sy, z)))
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
        let before: Vec<_> = sections
            .iter()
            .map(|&k| world_mesh(&world, &light, k))
            .collect();
        assert!(world.set_block(pos, block));
        let mut affected = affected_sections(pos);
        affected.extend(light.block_changed(&world, pos));
        let mut changed = 0;
        for (&key, before) in sections.iter().zip(before) {
            if world_mesh(&world, &light, key) != before {
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

/// Секция с блоками `blocks`, остальное — воздух; за краем тоже воздух.
fn section_of(blocks: &[(IVec3, Block)]) -> SectionMeshes {
    let mut section = Section::new();
    for &(pos, block) in blocks {
        section.set(pos, block);
    }
    mesh_section(&section, air, full, AO)
}

/// Нормали квадов меша по порядку вершин (по четыре на квад).
fn quad_normals(mesh: &MeshData) -> Vec<IVec3> {
    vertices(mesh).chunks(4).map(quad_normal).collect()
}

#[test]
fn cross_has_both_sides_of_both_planes() {
    let at = IVec3::new(5, 5, 5);
    for block in [Block::TallGrass, Block::Poppy, Block::Dandelion] {
        let mesh = section_of(&[(at, block)]).cutout;
        assert_eq!(vertices(&mesh).len(), 16, "{block:?}");
        assert_eq!(indices(&mesh).len(), 24, "{block:?}");
        let normals = quad_normals(&mesh);
        // Диагонали (±1, 0, ±1): у каждой плоскости обе стороны.
        for n in [(1, -1), (-1, 1), (1, 1), (-1, -1)] {
            let count = normals.iter().filter(|v| v.x == n.0 && v.z == n.1).count();
            assert_eq!(count, 1, "{block:?} normal {n:?}: {normals:?}");
        }
        for v in &vertices(&mesh) {
            assert_eq!(v.layer(), block.tile(Facing::Side).unwrap().layer() as u8);
            let [x, y, z] = v.position();
            assert!(
                (5.0..=6.0).contains(&x) && (5.0..=6.0).contains(&y) && (5.0..=6.0).contains(&z)
            );
        }
    }
}

#[test]
fn cross_has_light_of_its_own_cell_and_full_shade() {
    let mesh = section_of(&[(IVec3::new(5, 5, 5), Block::Poppy)]).cutout;
    let first = vertices(&mesh)[0];
    assert!(vertices(&mesh).iter().all(|v| v.shade_byte() == 255
        && v.sun_halves() == first.sun_halves()
        && v.block_halves() == first.block_halves()));
}

#[test]
fn cube_next_to_cross_draws_its_face_toward_it() {
    let mesh = pair(Block::Stone, Block::TallGrass);
    // Камень рисует все шесть граней, трава — свой крест.
    assert_eq!(vertices(&mesh).len(), 24 + 16);
}

#[test]
fn cross_does_not_hide_cube_faces_or_shade_corners() {
    let floor = IVec3::new(5, 4, 5);
    let plain = section_of(&[(floor, Block::Stone)]).solid;
    let with = section_of(&[(floor, Block::Stone), (floor + IVec3::Y, Block::Torch)]).solid;
    // Верх камня остался, и тени от факела на нём нет: вершины камня те же.
    assert_eq!(
        &vertices(&with)[..vertices(&plain).len()],
        &vertices(&plain)[..]
    );
}

#[test]
fn torch_is_a_narrow_column_with_five_faces() {
    let at = IVec3::new(5, 5, 5);
    let mesh = section_of(&[(at, Block::Torch)]).cutout;
    assert_eq!(vertices(&mesh).len(), 20);
    let normals = quad_normals(&mesh);
    assert!(!normals.contains(&IVec3::NEG_Y), "no bottom face");
    assert!(normals.contains(&IVec3::Y));
    let (mut min, mut max) = (Vec3::MAX, Vec3::MIN);
    for v in &vertices(&mesh) {
        min = min.min(v.position().into());
        max = max.max(v.position().into());
        assert_eq!(v.layer(), Tile::Torch.layer() as u8);
    }
    let origin = Vec3::new(5.0, 5.0, 5.0);
    assert_eq!(min - origin, Vec3::new(7.0, 0.0, 7.0) / 16.0);
    assert_eq!(max - origin, Vec3::new(9.0, 10.0, 9.0) / 16.0);
}

#[test]
fn torch_light_comes_from_its_cell() {
    let torch = IVec3::new(5, 20, 5);
    let (world, light) = small_world(|p| if p == torch { Block::Torch } else { Block::Air });
    let mesh = mesh_world_section(&world, &light, SectionPos::new(0, 1, 0), AO).unwrap();
    assert!(
        vertices(&mesh.solid)
            .iter()
            .all(|v| v.block_halves() == 2 * 14)
    );
}

#[test]
fn water_block_in_air_has_six_lowered_faces() {
    let mesh = section_of(&[(IVec3::new(5, 5, 5), Block::Water)]);
    assert_eq!(face_count(&mesh.solid), 0);
    assert_eq!(face_count(&mesh.water), 6);
    let top = 5.0 + WATER_SURFACE;
    for quad in vertices(&mesh.water).chunks(4) {
        assert!(quad.iter().all(|v| v.layer() == Tile::Water.layer() as u8));
        // Без AO: одна яркость стороны на весь квад.
        let shade = face_shade(quad_normal(quad));
        assert!(
            quad.iter().all(|v| v.shade_byte() == quantize_shade(shade)),
            "{quad:?}"
        );
        for v in quad {
            assert!([5.0, top].contains(&v.position()[1]), "{v:?}");
        }
        if quad_normal(quad).y == 0 {
            for v in quad {
                assert_eq!(v.uv()[1], 6.0 - v.position()[1], "{v:?}");
            }
        }
    }
}

#[test]
fn adjacent_water_shares_hidden_face() {
    let mesh = section_of(&[
        (IVec3::new(5, 5, 5), Block::Water),
        (IVec3::new(6, 5, 5), Block::Water),
    ]);
    assert_eq!(face_count(&mesh.water), 10);
    assert_eq!(face_count(&mesh.solid), 0);
}

#[test]
fn water_has_no_face_toward_stone_but_stone_does() {
    let mesh = section_of(&[
        (IVec3::new(5, 5, 5), Block::Stone),
        (IVec3::new(6, 5, 5), Block::Water),
    ]);
    assert!(
        has_face_at_x(&mesh.solid, 6.0, IVec3::X),
        "stone toward water"
    );
    assert!(
        !has_face_at_x(&mesh.water, 6.0, IVec3::NEG_X),
        "water toward stone"
    );
    assert_eq!(face_count(&mesh.solid), 6);
    assert_eq!(face_count(&mesh.water), 5);
}

#[test]
fn water_has_no_face_toward_leaves() {
    let mesh = section_of(&[
        (IVec3::new(5, 5, 5), Block::Leaves),
        (IVec3::new(6, 5, 5), Block::Water),
    ]);
    assert!(
        has_face_at_x(&mesh.cutout, 6.0, IVec3::X),
        "leaves toward water"
    );
    assert_eq!(face_count(&mesh.water), 5);
}

#[test]
fn water_under_water_is_full_height_without_top() {
    let (low, high) = (IVec3::new(5, 5, 5), IVec3::new(5, 6, 5));
    let mesh = section_of(&[(low, Block::Water), (high, Block::Water)]);
    assert_eq!(face_count(&mesh.water), 10);
    let tops: Vec<f32> = vertices(&mesh.water)
        .chunks(4)
        .filter(|quad| quad_normal(quad) == IVec3::Y)
        .flat_map(|quad| quad.iter().map(|v| v.position()[1]))
        .collect();
    assert_eq!(
        tops,
        [6.0 + WATER_SURFACE; 4],
        "only the upper block has a top"
    );
    // Боковые грани нижнего блока — до самого верха, без щели под верхним.
    let water_vertices = vertices(&mesh.water);
    let low_sides = water_vertices
        .chunks(4)
        .filter(|quad| quad_normal(quad).y == 0 && quad.iter().any(|v| v.position()[1] == 5.0));
    for quad in low_sides {
        let max = quad.iter().map(|v| v.position()[1]).fold(0.0, f32::max);
        assert_eq!(max, 6.0, "{quad:?}");
    }
}

#[test]
fn water_under_stone_is_lowered_without_top() {
    let mesh = section_of(&[
        (IVec3::new(5, 5, 5), Block::Water),
        (IVec3::new(5, 6, 5), Block::Stone),
    ]);
    assert_eq!(face_count(&mesh.water), 5);
    let max = vertices(&mesh.water)
        .iter()
        .map(|v| v.position()[1])
        .fold(0.0, f32::max);
    assert_eq!(max, 5.0 + WATER_SURFACE);
    // Низ камня над водой виден.
    assert_eq!(face_count(&mesh.solid), 6);
}

#[test]
fn lake_on_column_seams_matches_whole_world() {
    use world::terrain::{Terrain, lake_key};

    let terrain = Terrain::new(1337);
    let center = lake_key(&terrain);
    let mut world = World::new();
    for z in -1..=1 {
        for x in -1..=1 {
            let key = center + IVec2::new(x, z);
            world.insert(key, terrain.generate_trees(key));
        }
    }
    let light = world_light(&world);
    let origin = center.origin();
    let meshes = mesh_world_column(&world, &light, center, AO).unwrap();
    let expected = world_face_count(&world, center);
    assert!(expected.1 > 0, "no water faces in {center}");
    assert_eq!(column_face_count(&meshes), expected);
    for (sy, mesh) in meshes.iter().enumerate() {
        assert_shades_match(
            &mesh.solid,
            section_origin(origin, sy),
            |p| world.block(p),
            |p| light.packed(p),
            |_| true,
        );
    }
    // Вода на западном краю колонки, и сосед за краем — тоже вода: грани между
    // ними нет ни в одной из двух колонок.
    let seam_water = (0..SECTION_SIZE).any(|z| {
        let p = origin + IVec3::new(0, world::terrain::WATER_LEVEL, z);
        world.block(p).is_water() && world.block(p - IVec3::X).is_water()
    });
    assert!(seam_water, "lake does not cross the seam");
}

/// Кэш из колонок (срезами) и кэш из замыканий дают побитово те же меши — в том числе
/// у границ мира сверху и снизу и рядом с незагруженными колонками.
#[test]
fn world_cache_matches_closure_cache() {
    use world::terrain::Terrain;

    let terrain = Terrain::new(1337);
    let mut world = World::new();
    // Колонка (2, 0) намеренно без восточного соседа (3, 0) и юго-восточного.
    for z in -1..=1 {
        for x in -1..=2 {
            let key = ColumnPos::new(x, z);
            world.insert(key, terrain.generate_trees(key));
        }
    }
    let mut light = LightMap::new();
    for key in world.keys().collect::<Vec<_>>() {
        light.add_column(&world, key);
    }
    for key in [
        ColumnPos::new(0, 0),
        ColumnPos::new(2, 0),
        ColumnPos::new(-1, 1),
    ] {
        let origin = key.origin();
        let fast = mesh_world_column(&world, &light, key, AO).unwrap();
        let slow = mesh_column(
            world.column(key).unwrap(),
            |local| world.block(origin + local),
            light.column_sampler(key),
            AO,
        );
        for (sy, (a, b)) in fast.iter().zip(&slow).enumerate() {
            assert!(
                vertices(&a.solid) == vertices(&b.solid),
                "{key} section {sy}"
            );
            assert!(indices(&a.solid) == indices(&b.solid), "{key} section {sy}");
            assert!(vertices(&a.water) == vertices(&b.water), "{key} water {sy}");
            assert!(indices(&a.water) == indices(&b.water), "{key} water {sy}");
            let single = mesh_world_section(&world, &light, key.section(sy as i32), AO).unwrap();
            assert!(
                vertices(&single.solid) == vertices(&a.solid),
                "{key} single {sy}"
            );
        }
    }
}

/// Квантованная яркость и позиции совпадают с точным расчётом из блоков и света
/// мира с точностью до шага квантования (`assert_shades_match` сравнивает байт
/// вершины с `quantize_shade` независимого расчёта по миру).
#[test]
fn quantization_error_is_below_one_step() {
    let (world, light) = small_world(|p| if p.y < 5 { Block::Stone } else { Block::Air });
    let mesh = mesh_world_section(&world, &light, SectionPos::new(0, 0, 0), AO).unwrap();
    for v in &vertices(&mesh.solid) {
        let exact = v.shade_byte() as f32 / 255.0;
        assert_eq!(v.shade(), exact);
        assert!(v.position().iter().all(|c| (0.0..=16.0).contains(c)));
    }
}

/// Мир 3×3 колонок вокруг (0, 0) из `block_at` и его свет.
fn small_world(block_at: impl Fn(IVec3) -> Block) -> (World, LightMap) {
    use world::column::WORLD_HEIGHT;

    let mut world = World::new();
    for z in -1..=1 {
        for x in -1..=1 {
            let key = ColumnPos::new(x, z);
            let mut column = Column::new();
            for y in 0..WORLD_HEIGHT {
                for lz in 0..SECTION_SIZE {
                    for lx in 0..SECTION_SIZE {
                        let local = IVec3::new(lx, y, lz);
                        let pos = local + IVec3::new(x, 0, z) * SECTION_SIZE;
                        column.set(local, block_at(pos));
                    }
                }
            }
            world.insert(key, column);
        }
    }
    let light = world_light(&world);
    (world, light)
}

#[test]
fn glowstone_lights_a_dark_room_by_the_brighter_channel() {
    // Закрытая комната 1 блок высотой с светокамнем: солнца нет, свет только блочный.
    let (world, light) = small_world(|p| match (p.y, p.x, p.z) {
        (1, 8, 8) => Block::Glowstone,
        (1, ..) => Block::Air,
        _ => Block::Stone,
    });
    let mesh = mesh_world_section(&world, &light, SectionPos::default(), AO)
        .unwrap()
        .solid;
    assert_shades_match(
        &mesh,
        IVec3::ZERO,
        |p| world.block(p),
        |p| light.packed(p),
        |_| true,
    );
    let top_of_floor = |x: f32| {
        vertices(&mesh)
            .iter()
            .filter(|v| v.position()[1] == 1.0 && v.position()[0] == x && v.position()[2] == 4.0)
            .map(|v| v.brightness(0.0))
            .fold(0.0f32, f32::max)
    };
    assert!(top_of_floor(7.0) > top_of_floor(1.0));
}

#[test]
fn floor_under_canopy_is_darker_and_fades() {
    // Пол на y = 0, навес на y = 3 над x ≥ 6: под ним свет убывает от края.
    let (world, light) = small_world(|p| {
        if p.y == 0 || (p.y == 3 && p.x >= 6) {
            Block::Stone
        } else {
            Block::Air
        }
    });
    let mesh = mesh_world_section(&world, &light, SectionPos::default(), AO)
        .unwrap()
        .solid;
    let brightest = |x| {
        shades(&top_face(&mesh, IVec3::new(x, 0, 8)).0)
            .into_iter()
            .fold(0.0, f32::max)
    };
    assert_eq!(brightest(2), 1.0);
    let under: Vec<f32> = (6..14).map(brightest).collect();
    assert!(under[0] < 1.0, "{under:?}");
    assert!(under.windows(2).all(|w| w[1] < w[0]), "{under:?}");
    assert!(under[7] < 0.5, "{under:?}");
    assert_shades_match(
        &mesh,
        IVec3::ZERO,
        |p| world.block(p),
        |p| light.packed(p),
        |_| true,
    );
}

#[test]
fn opaque_cells_do_not_darken_average() {
    // Блок на открытом полу: у его подножия AO, но свет 15 — сам блок (уровень 0)
    // в среднее не входит, и яркость та же, что без света.
    let block = IVec3::new(8, 1, 8);
    let (world, light) = small_world(|p| {
        if p.y == 0 || p == block {
            Block::Stone
        } else {
            Block::Air
        }
    });
    let mesh = mesh_world_section(&world, &light, SectionPos::default(), AO)
        .unwrap()
        .solid;
    let (beside, _) = top_face(&mesh, IVec3::new(7, 0, 8));
    assert_near(shades(&beside), [1.0, 0.85, 0.85, 1.0]);
}

#[test]
fn water_takes_light_of_cell_in_front() {
    // Лужа под навесом и такая же под открытым небом.
    let (world, light) = small_world(|p| match p.y {
        0 => Block::Stone,
        1 if p.x == 2 || p.x == 12 => Block::Water,
        4 if p.x >= 8 => Block::Stone,
        _ => Block::Air,
    });
    let water = mesh_world_section(&world, &light, SectionPos::default(), AO)
        .unwrap()
        .water;
    let top_shade = |x: i32| {
        let water_vertices = vertices(&water);
        let quad = water_vertices
            .chunks(4)
            .find(|q| {
                quad_normal(q) == IVec3::Y
                    && q[0].position()[0] == x as f32
                    && q[0].position()[2] == 8.0
            })
            .unwrap();
        assert!(
            quad.iter()
                .all(|v| v.brightness(0.0) == quad[0].brightness(0.0)),
            "{quad:?}"
        );
        quad[0].brightness(0.0)
    };
    assert_eq!(top_shade(2), 1.0);
    let covered = world::light::brightness(light.sky(IVec3::new(12, 2, 8)));
    assert!((top_shade(12) - covered).abs() <= QUANT, "{covered}");
    assert!(top_shade(12) < 0.5);
}

#[test]
fn world_section_outside_is_none() {
    let mut world = World::new();
    world.insert(ColumnPos::ZERO, Column::new());
    let light = world_light(&world);
    for (key, some) in [
        (SectionPos::new(0, 0, 0), true),
        (SectionPos::new(0, COLUMN_SECTIONS, 0), false),
        (SectionPos::new(0, -1, 0), false),
        (SectionPos::new(1, 0, 0), false),
    ] {
        assert_eq!(
            mesh_world_section(&world, &light, key, AO).is_some(),
            some,
            "{key}"
        );
    }
}
