use glam::Vec3;

use super::expand::{Vertex, vertices};
use super::*;

const AT: IVec3 = IVec3::new(5, 5, 5);
const INSET: f32 = 1.0 / 16.0;

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

fn local(vertex: Vertex) -> Vec3 {
    Vec3::from(vertex.position()) - AT.as_vec3()
}

fn normal(quad: &[Vertex]) -> IVec3 {
    let [a, b, c] = [0, 1, 2].map(|i| Vec3::from(quad[i].position()));
    (b - a).cross(c - b).normalize().round().as_ivec3()
}

#[test]
fn cactus_is_six_cutout_quads() {
    let meshes = mesh_with(&[(AT, Block::Cactus)]);
    assert!(meshes.solid.quads.is_empty());
    assert_eq!(meshes.cutout.quads.len(), 6);
}

#[test]
fn cactus_faces_are_pressed_in_by_a_sixteenth() {
    let verts = vertices(&mesh_with(&[(AT, Block::Cactus)]).cutout);
    for quad in verts.chunks(4) {
        let n = normal(quad);
        for vertex in quad {
            let p = local(*vertex);
            match n {
                IVec3::Y => {
                    assert_eq!(p.y, 1.0);
                    assert!(p.x == INSET || p.x == 1.0 - INSET, "{p}");
                    assert!(p.z == INSET || p.z == 1.0 - INSET, "{p}");
                }
                IVec3::NEG_Y => assert_eq!(p.y, 0.0),
                _ => {
                    let along = if n.x != 0 { p.x } else { p.z };
                    let expected = if n.x + n.z > 0 { 1.0 - INSET } else { INSET };
                    assert_eq!(along, expected, "{p} {n}");
                    assert!(p.y == 0.0 || p.y == 1.0, "{p}");
                }
            }
        }
    }
}

#[test]
fn cactus_uses_its_own_tiles_per_face() {
    let verts = vertices(&mesh_with(&[(AT, Block::Cactus)]).cutout);
    for quad in verts.chunks(4) {
        let expected = match normal(quad) {
            IVec3::Y => Tile::CactusTop,
            IVec3::NEG_Y => Tile::CactusBottom,
            _ => Tile::CactusSide,
        };
        assert!(quad.iter().all(|v| v.layer() == expected.layer() as u8));
    }
}

#[test]
fn cactus_hides_its_bottom_on_sand_but_keeps_sides_next_to_blocks() {
    let below = AT - IVec3::Y;
    let meshes = mesh_with(&[
        (AT, Block::Cactus),
        (below, Block::Sand),
        (AT + IVec3::X, Block::Stone),
    ]);
    let cactus: Vec<_> = vertices(&meshes.cutout).chunks(4).map(normal).collect();
    assert_eq!(cactus.len(), 5);
    assert!(!cactus.contains(&IVec3::NEG_Y));
    assert!(cactus.contains(&IVec3::X));
}
