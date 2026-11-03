use glam::Vec3;

use super::expand::{Vertex, vertices};
use super::*;
use world::block::{VINE_NEG_X, VINE_POS_X, VINE_POS_Z, VINE_TOP};

const AT: IVec3 = IVec3::new(5, 5, 5);
const INSET: f32 = 1.0 / 16.0;

fn air(_: IVec3) -> Block {
    Block::Air
}

fn full(_: IVec3) -> u8 {
    MAX_LIGHT
}

fn mesh_vine(data: u8) -> SectionMeshes {
    let mut section = Section::new();
    section.set_with_data(AT, Block::Vine, data);
    mesh_section(&section, air, full, MeshOptions::default())
}

fn local(vertex: Vertex) -> Vec3 {
    Vec3::from(vertex.position()) - AT.as_vec3()
}

#[test]
fn every_side_is_a_pair_of_cutout_quads() {
    for (data, quads) in [
        (VINE_POS_X, 2),
        (VINE_POS_X | VINE_NEG_X, 4),
        (VINE_POS_X | VINE_POS_Z | VINE_TOP, 6),
    ] {
        let meshes = mesh_vine(data);
        assert!(meshes.solid.quads.is_empty());
        assert_eq!(meshes.cutout.quads.len(), quads, "{data:#b}");
    }
}

#[test]
fn planes_sit_a_sixteenth_inside_the_attached_face() {
    for (data, axis, offset) in [
        (VINE_POS_X, 0, 1.0 - INSET),
        (VINE_NEG_X, 0, INSET),
        (VINE_POS_Z, 2, 1.0 - INSET),
        (VINE_TOP, 1, 1.0 - INSET),
    ] {
        let verts = vertices(&mesh_vine(data).cutout);
        assert_eq!(verts.len(), 8);
        for vertex in verts {
            assert_eq!(local(vertex)[axis], offset, "{data:#b}");
        }
    }
}

#[test]
fn both_sides_of_a_plane_face_opposite_ways() {
    let verts = vertices(&mesh_vine(VINE_POS_X).cutout);
    let normal = |quad: &[Vertex]| {
        let [a, b, c] = [0, 1, 2].map(|i| Vec3::from(quad[i].position()));
        (b - a).cross(c - b).normalize().round().as_ivec3()
    };
    let normals: Vec<IVec3> = verts.chunks(4).map(normal).collect();
    assert_eq!(normals[0], -normals[1]);
    assert_eq!(normals[0].abs(), IVec3::X);
}

#[test]
fn vine_uses_the_vine_tile() {
    let verts = vertices(&mesh_vine(VINE_TOP).cutout);
    assert!(verts.iter().all(|v| v.layer() == Tile::Vine.layer() as u8));
}
