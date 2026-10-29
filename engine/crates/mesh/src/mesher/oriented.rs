//! Блоки с байтом данных: бревно по оси и факел на стене.

use glam::Vec3;

use super::expand::{Vertex, vertices};
use super::*;
use world::block::{
    ATTACH_NEG_X, ATTACH_NEG_Z, ATTACH_POS_X, ATTACH_POS_Z, AXIS_X, AXIS_Y, AXIS_Z,
};

const AT: IVec3 = IVec3::new(5, 5, 5);

fn air(_: IVec3) -> Block {
    Block::Air
}

fn full(_: IVec3) -> u8 {
    MAX_LIGHT
}

/// Одинокий блок с данными в воздухе, полностью освещённый.
fn single(block: Block, data: u8) -> SectionMeshes {
    let mut section = Section::new();
    section.set_with_data(AT, block, data);
    mesh_section(&section, air, full, MeshOptions::default())
}

fn normal(quad: &[Vertex]) -> Vec3 {
    let [a, b, c] = [0, 1, 2].map(|i| Vec3::from(quad[i].position()));
    (b - a).cross(c - b).normalize()
}

fn axis_vector(axis: usize) -> IVec3 {
    let mut v = IVec3::ZERO;
    v[axis] = 1;
    v
}

#[test]
fn log_axis_puts_ends_on_the_axis_faces_and_bark_elsewhere() {
    for (data, axis) in [(AXIS_Y, 1), (AXIS_X, 0), (AXIS_Z, 2)] {
        let mesh = single(Block::Log, data).solid;
        let verts = vertices(&mesh);
        assert_eq!(verts.len(), 24, "axis {axis}");
        for quad in verts.chunks(4) {
            let n = normal(quad).round().as_ivec3();
            let on_axis = n.abs() == axis_vector(axis);
            let want = if on_axis { Tile::LogTop } else { Tile::LogSide };
            assert!(
                quad.iter().all(|v| v.layer() == want.layer() as u8),
                "axis {axis} normal {n}"
            );
        }
    }
}

#[test]
fn bark_fibers_run_along_the_axis() {
    for (data, axis) in [(AXIS_Y, 1), (AXIS_X, 0), (AXIS_Z, 2)] {
        let verts = vertices(&single(Block::Log, data).solid);
        let mut checked = 0;
        for quad in verts.chunks(4) {
            if quad[0].layer() != Tile::LogSide.layer() as u8 {
                continue;
            }
            for (a, b) in [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)] {
                let (pa, pb) = (quad[a].position(), quad[b].position());
                let along = (0..3).all(|k| k == axis || pa[k] == pb[k]) && pa[axis] != pb[axis];
                if along {
                    // Вдоль оси меняется только v: вертикаль тайла (кора) идёт по бревну.
                    let (ua, ub) = (quad[a].uv(), quad[b].uv());
                    assert_eq!(ua[0], ub[0], "axis {axis}");
                    assert_eq!((ua[1] - ub[1]).abs(), 1.0, "axis {axis}");
                    checked += 1;
                }
            }
        }
        // У каждой из четырёх коровых граней два ребра вдоль оси.
        assert_eq!(checked, 8, "axis {axis}");
    }
}

#[test]
fn bark_uv_is_rotated_not_mirrored() {
    // Обход углов в UV совпадает с обходом углов грани: определитель одного знака.
    for data in [AXIS_X, AXIS_Z] {
        for quad in vertices(&single(Block::Log, data).solid).chunks(4) {
            let uv = |i: usize| quad[i].uv();
            let det = (uv(1)[0] - uv(0)[0]) * (uv(2)[1] - uv(1)[1])
                - (uv(1)[1] - uv(0)[1]) * (uv(2)[0] - uv(1)[0]);
            // На стоячем бревне, которое не зеркалится, определитель отрицателен
            // (v растёт вниз): значит, и здесь.
            assert!(det < 0.0, "{data} {quad:?}");
        }
    }
}

#[test]
fn standing_log_is_the_same_as_without_data() {
    let plain = {
        let mut section = Section::new();
        section.set(AT, Block::Log);
        mesh_section(&section, air, full, MeshOptions::default())
    };
    let standing = single(Block::Log, AXIS_Y);
    assert_eq!(plain.solid.quads, standing.solid.quads);
    // Мусорный байт читается как стоячее бревно.
    assert_eq!(plain.solid.quads, single(Block::Log, 99).solid.quads);
    // Стоячее бревно целиком из шаблонов куба.
    assert!(plain.solid.quads.iter().all(|q| q.template() < 6));
}

#[test]
fn lying_log_keeps_ao_and_light_like_a_cube() {
    // Лежачее бревно у стены: AO и свет по тем же углам, что и у стоячего.
    let wall = IVec3::new(6, 5, 5);
    let mesh = |data| {
        let mut section = Section::new();
        section.set(wall, Block::Stone);
        section.set_with_data(AT, Block::Log, data);
        mesh_section(&section, air, full, MeshOptions::default()).solid
    };
    let (standing, lying) = (mesh(AXIS_Y), mesh(AXIS_X));
    let shades = |m: &MeshData| {
        let mut all: Vec<_> = m
            .quads
            .iter()
            .map(|q| (q.cell(), q.ao(), q.flipped()))
            .collect();
        all.sort();
        all
    };
    // Грани те же (скрытая одна и та же), AO по ним совпадает; различаются шаблоны и тайлы.
    assert_eq!(shades(&standing), shades(&lying));
}

#[test]
fn wall_torch_leans_away_from_the_wall_in_its_hitbox() {
    let u = 1.0 / 16.0;
    for (data, lean) in [
        (ATTACH_POS_X, IVec3::NEG_X),
        (ATTACH_NEG_X, IVec3::X),
        (ATTACH_POS_Z, IVec3::NEG_Z),
        (ATTACH_NEG_Z, IVec3::Z),
    ] {
        let mesh = single(Block::Torch, data).cutout;
        let verts = vertices(&mesh);
        assert_eq!(verts.len(), 20, "{data}");
        assert!(verts.iter().all(|v| v.layer() == Tile::Torch.layer() as u8));
        let (mut min, mut max) = (Vec3::MAX, Vec3::MIN);
        for v in &verts {
            min = min.min(v.position().into());
            max = max.max(v.position().into());
        }
        let origin = AT.as_vec3();
        // Рамка выделения совпадает с мешем.
        let (lo, hi) = Block::Torch.bounds(data);
        assert_eq!((min - origin, max - origin), (lo, hi), "{data}");
        // Основание у стены клетки, верх уходит в сторону наклона на 4/16.
        let toward_wall = -lean.as_vec3();
        let wall_side = if toward_wall.max_element() > 0.0 {
            1.0
        } else {
            0.0
        };
        let lean_axis = lean.abs().max_position();
        let edge = if wall_side == 1.0 { max } else { min } - origin;
        assert_eq!(edge[lean_axis], wall_side, "{data}");
        assert_eq!(((max - min) * 16.0)[lean_axis], 6.0, "{data}");
        // Верхняя грань горизонтальна и на высоте 13/16; низа нет.
        let normals: Vec<_> = verts.chunks(4).map(normal).collect();
        assert!(normals.iter().all(|n| n.y > -0.5), "no bottom face");
        let top: Vec<_> = verts
            .chunks(4)
            .filter(|q| q.iter().all(|v| v.position()[1] == origin.y + 13.0 * u))
            .collect();
        assert_eq!(top.len(), 1, "{data}");
        // Бока наклонены: ни один не вертикален, кроме двух перпендикулярных наклону.
        let tilted = normals
            .iter()
            .filter(|n| n.y.abs() > 0.1 && n.y.abs() < 0.9)
            .count();
        assert_eq!(tilted, 2, "{data}");
    }
}

#[test]
fn wall_torch_angle_is_about_22_degrees() {
    let degrees = (4.0f32 / 10.0).atan().to_degrees();
    assert!((degrees - 22.0).abs() < 0.5, "{degrees}");
}

#[test]
fn unknown_torch_data_is_a_floor_torch() {
    let floor = single(Block::Torch, 0).cutout.quads;
    assert_eq!(single(Block::Torch, 5).cutout.quads, floor);
    assert_eq!(single(Block::Torch, 255).cutout.quads, floor);
}

#[test]
fn wall_torch_light_comes_from_its_cell() {
    let mesh = single(Block::Torch, ATTACH_NEG_Z).cutout;
    assert!(
        vertices(&mesh)
            .iter()
            .all(|v| v.sun_halves() == 2 * MAX_LIGHT)
    );
}

#[test]
fn neighbor_section_data_does_not_change_the_mesh() {
    // Мешеру нужны только блоки соседей (непрозрачность), а не их данные.
    let mut world = World::new();
    for x in -1..=1 {
        for z in -1..=1 {
            world.insert(ColumnPos::new(x, z), Column::new());
        }
    }
    let edge = IVec3::new(15, 5, 5);
    let mesh_with = |world: &mut World, data| {
        world.set_block_with_data(edge, Block::Torch, 0);
        world.set_block_with_data(edge + IVec3::X, Block::Log, data);
        let mut light = LightMap::new();
        for key in world.keys().collect::<Vec<_>>() {
            light.add_column(world, key);
        }
        mesh_world_section(
            world,
            &light,
            SectionPos::new(0, 0, 0),
            MeshOptions::default(),
        )
        .unwrap()
    };
    let a = mesh_with(&mut world, AXIS_Y);
    let b = mesh_with(&mut world, AXIS_X);
    assert_eq!(a.solid.quads, b.solid.quads);
    assert_eq!(a.cutout.quads, b.cutout.quads);
}
