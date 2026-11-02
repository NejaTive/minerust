use glam::Vec3;

use super::expand::{Vertex, vertices};
use super::*;

const AT: IVec3 = IVec3::new(5, 5, 5);
const LAYER_HEIGHT: f32 = 2.0 / 16.0;

fn air(_: IVec3) -> Block {
    Block::Air
}

fn full(_: IVec3) -> u8 {
    MAX_LIGHT
}

fn mesh_with(blocks: &[(IVec3, Block)]) -> SectionMeshes {
    let mut section = Section::new();
    for &(pos, block) in blocks {
        section.set(pos, block);
    }
    mesh_section(&section, air, full, MeshOptions::default())
}

fn normal(quad: &[Vertex]) -> IVec3 {
    let [a, b, c] = [0, 1, 2].map(|i| Vec3::from(quad[i].position()));
    (b - a).cross(c - b).normalize().round().as_ivec3()
}

fn local(vertex: Vertex) -> Vec3 {
    Vec3::from(vertex.position()) - AT.as_vec3()
}

#[test]
fn snow_layer_is_a_two_sixteenths_box_in_the_solid_pass() {
    let meshes = mesh_with(&[(AT, Block::Snow)]);
    assert!(meshes.cutout.quads.is_empty() && meshes.water.quads.is_empty());
    let verts = vertices(&meshes.solid);
    assert_eq!(verts.len(), 24);
    for vertex in &verts {
        assert_eq!(vertex.layer(), Tile::Snow.layer() as u8);
        let p = local(*vertex);
        assert!(
            (0.0..=1.0).contains(&p.x) && (0.0..=1.0).contains(&p.z),
            "{p}"
        );
        assert!(p.y == 0.0 || (p.y - LAYER_HEIGHT).abs() < 1e-6, "{p}");
    }
    for quad in verts.chunks(4) {
        let n = normal(quad);
        let tops = quad
            .iter()
            .filter(|v| local(**v).y > 0.5 * LAYER_HEIGHT)
            .count();
        match n {
            IVec3::Y => assert_eq!(tops, 4),
            IVec3::NEG_Y => assert_eq!(tops, 0),
            _ => assert_eq!(tops, 2, "{n}"),
        }
    }
}

#[test]
fn snow_sides_show_only_the_top_strip_of_the_texture() {
    let verts = vertices(&mesh_with(&[(AT, Block::Snow)]).solid);
    for quad in verts.chunks(4) {
        if normal(quad).y != 0 {
            continue;
        }
        for vertex in quad {
            let (v, top) = (vertex.uv()[1], local(*vertex).y > 0.0);
            let expected = if top { 1.0 - LAYER_HEIGHT } else { 1.0 };
            assert!((v - expected).abs() < 1e-6, "v {v}");
        }
    }
}

#[test]
fn snow_hides_faces_against_opaque_blocks_and_neighbouring_snow() {
    let meshes = mesh_with(&[
        (AT, Block::Snow),
        (AT - IVec3::Y, Block::Stone),
        (AT + IVec3::X, Block::Snow),
        (AT + IVec3::Z, Block::Stone),
    ]);
    let around = |center: IVec3| {
        vertices(&meshes.solid)
            .chunks(4)
            .filter(|q| {
                q.iter().all(|v| {
                    (Vec3::from(v.position()) - center.as_vec3() - 0.5)
                        .abs()
                        .max_element()
                        <= 0.5 + 1e-6
                })
            })
            .map(normal)
            .collect::<Vec<_>>()
    };
    let first = around(AT);
    assert!(first.contains(&IVec3::Y));
    assert!(!first.contains(&IVec3::NEG_Y), "bottom sits on stone");
    assert!(!first.contains(&IVec3::X), "side against more snow");
    assert!(!first.contains(&IVec3::Z), "side against stone");
    assert!(first.contains(&IVec3::NEG_X) && first.contains(&IVec3::NEG_Z));
}

#[test]
fn snow_on_leaves_keeps_its_bottom_face() {
    let meshes = mesh_with(&[(AT, Block::Snow), (AT - IVec3::Y, Block::Leaves)]);
    let snow_normals: Vec<IVec3> = vertices(&meshes.solid).chunks(4).map(normal).collect();
    assert!(snow_normals.contains(&IVec3::NEG_Y));
}

#[test]
fn snow_does_not_hide_the_block_below() {
    let meshes = mesh_with(&[(AT, Block::Snow), (AT - IVec3::Y, Block::SnowyGrass)]);
    let top_of_ground = vertices(&meshes.solid)
        .chunks(4)
        .filter(|q| {
            normal(q) == IVec3::Y && q.iter().all(|v| (local(*v).y - (-1.0 + 1.0)).abs() < 1e-6)
        })
        .count();
    assert_eq!(top_of_ground, 1);
}

#[test]
fn ice_goes_to_its_own_pass_as_a_full_cube() {
    let meshes = mesh_with(&[(AT, Block::Ice)]);
    assert!(meshes.solid.quads.is_empty() && meshes.cutout.quads.is_empty());
    let verts = vertices(&meshes.ice);
    assert_eq!(verts.len(), 24);
    assert!(verts.iter().all(|v| v.layer() == Tile::Ice.layer() as u8));
    let top = verts.chunks(4).find(|q| normal(q) == IVec3::Y).unwrap();
    assert!(top.iter().all(|v| (local(*v).y - 1.0).abs() < 1e-6));
}

#[test]
fn ice_hides_faces_against_ice_and_opaque_blocks() {
    let meshes = mesh_with(&[
        (AT, Block::Ice),
        (AT + IVec3::X, Block::Ice),
        (AT - IVec3::Y, Block::Stone),
        (AT + IVec3::Z, Block::Water),
    ]);
    let quads = vertices(&meshes.ice);
    let at_ice = |center: IVec3| {
        quads
            .chunks(4)
            .filter(|q| {
                q.iter().all(|v| {
                    (Vec3::from(v.position()) - center.as_vec3() - 0.5)
                        .abs()
                        .max_element()
                        <= 0.5 + 1e-6
                })
            })
            .map(normal)
            .collect::<Vec<_>>()
    };
    let first = at_ice(AT);
    assert!(!first.contains(&IVec3::X), "ice against ice");
    assert!(!first.contains(&IVec3::NEG_Y), "ice on stone");
    assert!(
        first.contains(&IVec3::Z),
        "ice side against water stays visible"
    );
    assert!(first.contains(&IVec3::Y));
}

#[test]
fn ice_templates_are_not_fluid_scroll_flagged() {
    let table = crate::templates::table_bytes();
    for index in [SNOW_TOP, SNOW_SIDE, SNOW_SIDE + 3, CUBE_FACE] {
        let at = usize::from(index) * 32 + 20;
        let flag = f32::from_le_bytes(table[at..at + 4].try_into().unwrap());
        assert_eq!(flag, 0.0, "template {index}");
    }
}

#[test]
fn ice_keeps_underside_over_water_and_top_under_water() {
    let over = mesh_with(&[(AT, Block::Ice), (AT - IVec3::Y, Block::Water)]);
    let ice_bottom = vertices(&over.ice)
        .chunks(4)
        .filter(|q| q[0].layer() == Tile::Ice.layer() as u8 && normal(q) == IVec3::NEG_Y)
        .count();
    assert_eq!(ice_bottom, 1);
    let under = mesh_with(&[(AT, Block::Ice), (AT + IVec3::Y, Block::Water)]);
    let tops = vertices(&under.ice)
        .chunks(4)
        .filter(|q| normal(q) == IVec3::Y && q.iter().all(|v| (local(*v).y - 1.0).abs() < 1e-6))
        .count();
    assert_eq!(tops, 1);
}

#[test]
fn water_cell_in_ice_is_closed_by_full_height_ice_sides() {
    let hole = AT + IVec3::new(0, 0, 0);
    let ring = [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z];
    let mut blocks = vec![(hole, Block::Water), (hole - IVec3::Y, Block::Stone)];
    blocks.extend(ring.map(|d| (hole + d, Block::Ice)));
    blocks.extend(
        [(1, 1), (1, -1), (-1, 1), (-1, -1)].map(|(x, z)| (hole + IVec3::new(x, 0, z), Block::Ice)),
    );
    let meshes = mesh_with(&blocks);
    let mut quads = vertices(&meshes.water);
    quads.extend(vertices(&meshes.ice));
    for dir in ring {
        let wall: Vec<&[Vertex]> = quads
            .chunks(4)
            .filter(|q| q[0].layer() == Tile::Ice.layer() as u8 && normal(q) == -dir)
            .filter(|q| {
                q.iter()
                    .all(|v| local(*v).dot(dir.as_vec3()) == f32::from(dir.x + dir.z > 0))
            })
            .collect();
        assert_eq!(wall.len(), 1, "one ice side facing the water from {dir}");
        let ys: Vec<f32> = wall[0].iter().map(|v| local(*v).y).collect();
        let (low, high) = (
            ys.iter().cloned().fold(f32::MAX, f32::min),
            ys.iter().cloned().fold(f32::MIN, f32::max),
        );
        assert_eq!((low, high), (0.0, 1.0), "full height ice side at {dir}");
    }
    let water_sides = quads
        .chunks(4)
        .filter(|q| q.iter().all(|v| v.layer() == Tile::Water.layer() as u8) && normal(q).y == 0)
        .count();
    assert_eq!(water_sides, 0, "water faces toward ice are hidden");
    let top = quads
        .chunks(4)
        .find(|q| q[0].layer() == Tile::Water.layer() as u8 && normal(q) == IVec3::Y)
        .expect("water top");
    assert!(top.iter().all(|v| (local(*v).y - 0.875).abs() < 1e-6));
}

fn ice_wall_heights(water: Block, data: u8, above: Option<Block>) -> Vec<(f32, f32)> {
    let mut section = Section::new();
    section.set_with_data(AT, water, data);
    for (x, z) in [
        (1, 0),
        (1, 1),
        (1, -1),
        (0, 1),
        (0, -1),
        (-1, 0),
        (-1, 1),
        (-1, -1),
    ] {
        section.set(AT + IVec3::new(x, 0, z), Block::Ice);
    }
    section.set(AT - IVec3::Y, Block::Stone);
    if let Some(block) = above {
        section.set(AT + IVec3::Y, block);
    }
    let meshes = mesh_section(&section, air, full, MeshOptions::default());
    vertices(&meshes.ice)
        .chunks(4)
        .filter(|q| q[0].layer() == Tile::Ice.layer() as u8 && normal(q) == IVec3::NEG_X)
        .filter(|q| q.iter().all(|v| local(*v).x == 1.0))
        .map(|q| {
            let ys = q.iter().map(|v| local(*v).y);
            (
                ys.clone().fold(f32::MAX, f32::min),
                ys.fold(f32::MIN, f32::max),
            )
        })
        .collect()
}

#[test]
fn ice_side_over_water_is_full_height_at_any_water_level() {
    for data in [0, 1, 4] {
        assert_eq!(ice_wall_heights(Block::Water, data, None), [(0.0, 1.0)]);
    }
}

#[test]
fn ice_side_next_to_deep_water_stays_full_height() {
    assert_eq!(
        ice_wall_heights(Block::Water, 0, Some(Block::Water)),
        [(0.0, 1.0)]
    );
}

mod frozen_lake {
    use std::collections::HashMap;

    use super::super::tests::world_light;
    use super::*;
    use world::column::Column;

    const ICE_Y: i32 = 62;

    fn lake() -> World {
        let mut world = World::new();
        for cz in -1..=1 {
            for cx in -1..=1 {
                let mut column = Column::new();
                for z in 0..16 {
                    for x in 0..16 {
                        let (wx, wz) = (cx * 16 + x, cz * 16 + z);
                        column.set(IVec3::new(x, 40, z), Block::Gravel);
                        for y in 41..ICE_Y {
                            column.set(IVec3::new(x, y, z), Block::Water);
                        }
                        let hole = wz == 3 && (-6..=7).contains(&wx);
                        let top = if hole { Block::Water } else { Block::Ice };
                        column.set(IVec3::new(x, ICE_Y, z), top);
                    }
                }
                world.insert(ColumnPos::new(cx, cz), column);
            }
        }
        world
    }

    fn water_quads(world: &World) -> Vec<(IVec3, IVec3, u8)> {
        let light = world_light(world);
        let mut out = Vec::new();
        for cz in -1..=1 {
            for cx in -1..=1 {
                let key = ColumnPos::new(cx, cz);
                let meshes = mesh_world_column(world, &light, key, MeshOptions::default()).unwrap();
                for (sy, section) in meshes.iter().enumerate() {
                    let origin = key.origin() + IVec3::Y * (sy as i32 * 16);
                    for quad in section.water.quads.iter().chain(&section.ice.quads) {
                        out.push((
                            origin + IVec3::from(quad.cell().map(i32::from)),
                            normal_of(quad.template()),
                            quad.layer(),
                        ));
                    }
                }
            }
        }
        out
    }

    fn normal_of(template: u8) -> IVec3 {
        use crate::templates::WATER_SIDE_SLOPED;
        const SIDES: [IVec3; 4] = [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z];
        let side = |first: u8| SIDES[usize::from(template - first)];
        match template {
            0 | 6 | 53 => IVec3::Y,
            1 | 15 => IVec3::NEG_Y,
            2..=5 => side(2),
            7..=10 => side(7),
            11..=14 => side(11),
            t if (WATER_SIDE_SLOPED..WATER_SIDE_SLOPED + 4).contains(&t) => side(WATER_SIDE_SLOPED),
            other => panic!("template {other}"),
        }
    }

    #[test]
    fn no_face_is_emitted_twice_across_section_borders() {
        let quads = water_quads(&lake());
        let mut seen: HashMap<(IVec3, IVec3, u8), usize> = HashMap::new();
        for q in &quads {
            *seen.entry(*q).or_default() += 1;
        }
        let duplicated: Vec<_> = seen.iter().filter(|(_, n)| **n > 1).collect();
        assert!(duplicated.is_empty(), "{duplicated:?}");
    }

    #[test]
    fn every_ice_cell_has_one_top_and_one_underside_over_water() {
        let world = lake();
        let quads = water_quads(&world);
        let ice = Tile::Ice.layer() as u8;
        let count = |cell: IVec3, n: IVec3| {
            quads
                .iter()
                .filter(|q| q.0 == cell && q.1 == n && q.2 == ice)
                .count()
        };
        let mut checked = 0;
        for z in -16..32 {
            for x in -16..32 {
                let cell = IVec3::new(x, ICE_Y, z);
                if world.block(cell) != Block::Ice {
                    continue;
                }
                checked += 1;
                assert_eq!(count(cell, IVec3::Y), 1, "top of {cell}");
                assert_eq!(count(cell, IVec3::NEG_Y), 1, "bottom of {cell}");
            }
        }
        assert!(checked > 1000);
    }

    #[test]
    fn ice_gets_a_side_face_exactly_where_it_meets_water() {
        let world = lake();
        let quads = water_quads(&world);
        let ice = Tile::Ice.layer() as u8;
        let walls: Vec<&(IVec3, IVec3, u8)> = quads
            .iter()
            .filter(|q| q.2 == ice && q.1.y == 0 && world.block(q.0 + q.1) == Block::Water)
            .collect();
        let expected = (-16..32)
            .flat_map(|z| (-16..32).map(move |x| IVec3::new(x, ICE_Y, z)))
            .filter(|c| world.block(*c) == Block::Ice)
            .map(|c| {
                [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z]
                    .into_iter()
                    .filter(|n| world.block(c + *n) == Block::Water)
                    .count()
            })
            .sum::<usize>();
        assert_eq!(walls.len(), expected);
    }
}

#[test]
fn ice_has_its_own_mesh_apart_from_water() {
    let mut blocks = vec![(AT, Block::Water)];
    for dir in [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z] {
        blocks.push((AT + dir, Block::Ice));
    }
    let meshes = mesh_with(&blocks);
    let ice = Tile::Ice.layer() as u8;
    assert!(!meshes.water.quads.is_empty() && !meshes.ice.quads.is_empty());
    assert!(meshes.water.quads.iter().all(|q| q.layer() != ice));
    assert!(meshes.ice.quads.iter().all(|q| q.layer() == ice));
}

#[test]
fn water_under_ice_has_no_surface_of_its_own() {
    let mut blocks = vec![];
    for x in -1..=1 {
        for z in -1..=1 {
            blocks.push((AT + IVec3::new(x, 0, z), Block::Water));
            blocks.push((AT + IVec3::new(x, 1, z), Block::Ice));
            blocks.push((AT + IVec3::new(x, -1, z), Block::Stone));
        }
    }
    let meshes = mesh_with(&blocks);
    let water = Tile::Water.layer() as u8;
    let surfaces = vertices(&meshes.water)
        .chunks(4)
        .filter(|q| q[0].layer() == water && normal(q) == IVec3::Y)
        .count();
    assert_eq!(surfaces, 0);
}

#[test]
fn water_in_a_hole_keeps_its_surface_and_full_height_neighbours_under_ice() {
    let hole = AT + IVec3::Y;
    let mut blocks = vec![(AT, Block::Water), (hole, Block::Water)];
    for dir in [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z] {
        blocks.push((AT + dir, Block::Water));
        blocks.push((hole + dir, Block::Ice));
        blocks.push((AT + dir - IVec3::Y, Block::Stone));
    }
    blocks.push((AT - IVec3::Y, Block::Stone));
    for (x, z) in [(1, 1), (1, -1), (-1, 1), (-1, -1)] {
        blocks.push((hole + IVec3::new(x, 0, z), Block::Ice));
        blocks.push((AT + IVec3::new(x, 0, z), Block::Water));
    }
    let meshes = mesh_with(&blocks);
    let water = Tile::Water.layer() as u8;
    let tops: Vec<f32> = vertices(&meshes.water)
        .chunks(4)
        .filter(|q| q[0].layer() == water && normal(q) == IVec3::Y)
        .map(|q| local(q[0]).y)
        .collect();
    assert_eq!(tops.len(), 1);
    assert!((tops[0] - 1.875).abs() < 1e-6, "{tops:?}");
}
