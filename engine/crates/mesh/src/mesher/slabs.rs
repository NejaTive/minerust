use glam::Vec3;
use world::block::{FACING_NEG_X, FACING_POS_X, FACING_POS_Z, SLAB_BOTTOM, SLAB_DOUBLE, SLAB_TOP};

use super::expand::{Vertex, vertices};
use super::*;

const AT: IVec3 = IVec3::new(5, 5, 5);

fn air(_: IVec3) -> Block {
    Block::Air
}

fn full(_: IVec3) -> u8 {
    MAX_LIGHT
}

fn mesh_with(blocks: &[(IVec3, Block, u8)]) -> SectionMeshes {
    mesh_lit(blocks, full)
}

fn mesh_lit(blocks: &[(IVec3, Block, u8)], light: impl Fn(IVec3) -> u8) -> SectionMeshes {
    let mut section = Section::new();
    for &(pos, block, data) in blocks {
        section.set_with_data(pos, block, data);
    }
    mesh_section(&section, air, light, MeshOptions::default())
}

fn bounds(quads: &MeshData) -> (Vec3, Vec3) {
    let points: Vec<Vec3> = vertices(quads)
        .iter()
        .map(|v| Vec3::from(v.position()))
        .collect();
    let min = points.iter().copied().reduce(Vec3::min).unwrap();
    let max = points.iter().copied().reduce(Vec3::max).unwrap();
    (min, max)
}

fn quad_vertices(quads: &MeshData) -> Vec<Vec<Vertex>> {
    vertices(quads).chunks(4).map(<[Vertex]>::to_vec).collect()
}

#[test]
fn bottom_slab_is_six_quads_half_a_block_high() {
    let meshes = mesh_with(&[(AT, Block::OakSlab, SLAB_BOTTOM)]);
    assert_eq!(meshes.solid.quads.len(), 6);
    let (min, max) = bounds(&meshes.solid);
    assert_eq!(min, AT.as_vec3());
    assert_eq!(max, AT.as_vec3() + Vec3::new(1.0, 0.5, 1.0));
}

#[test]
fn top_slab_sits_in_the_upper_half() {
    let meshes = mesh_with(&[(AT, Block::OakSlab, SLAB_TOP)]);
    let (min, max) = bounds(&meshes.solid);
    assert_eq!(min.y, AT.y as f32 + 0.5);
    assert_eq!(max.y, AT.y as f32 + 1.0);
}

#[test]
fn double_slab_looks_like_a_full_block() {
    let meshes = mesh_with(&[(AT, Block::OakSlab, SLAB_DOUBLE)]);
    assert_eq!(meshes.solid.quads.len(), 6);
    let (min, max) = bounds(&meshes.solid);
    assert_eq!((min, max), (AT.as_vec3(), AT.as_vec3() + Vec3::ONE));
}

#[test]
fn slab_side_uses_the_matching_half_of_the_texture() {
    let meshes = mesh_with(&[(AT, Block::OakSlab, SLAB_BOTTOM)]);
    for quad in quad_vertices(&meshes.solid) {
        let ys: Vec<f32> = quad.iter().map(|v| v.position()[1]).collect();
        if ys.iter().all(|&y| y == ys[0]) {
            continue;
        }
        let mut v: Vec<f32> = quad.iter().map(|vertex| vertex.uv()[1]).collect();
        v.sort_by(f32::total_cmp);
        assert_eq!((v[0], v[3]), (0.5, 1.0));
    }
    let top = quad_vertices(&meshes.solid)
        .into_iter()
        .find(|q| q.iter().all(|v| v.position()[1] == AT.y as f32 + 0.5))
        .unwrap();
    let spans = |axis: usize| {
        let values: Vec<f32> = top.iter().map(|v| v.uv()[axis]).collect();
        values.iter().copied().fold(0.0, f32::max) - values.iter().copied().fold(1.0, f32::min)
    };
    assert_eq!((spans(0), spans(1)), (1.0, 1.0));
}

#[test]
fn neighbouring_slabs_hide_the_shared_face() {
    let single = mesh_with(&[(AT, Block::OakSlab, SLAB_BOTTOM)]);
    let pair = mesh_with(&[
        (AT, Block::OakSlab, SLAB_BOTTOM),
        (AT + IVec3::X, Block::OakSlab, SLAB_BOTTOM),
    ]);
    assert_eq!(pair.solid.quads.len(), 2 * single.solid.quads.len() - 2);
}

#[test]
fn slabs_of_different_halves_keep_their_shared_face() {
    let pair = mesh_with(&[
        (AT, Block::OakSlab, SLAB_BOTTOM),
        (AT + IVec3::X, Block::OakSlab, SLAB_TOP),
    ]);
    assert_eq!(pair.solid.quads.len(), 12);
}

#[test]
fn a_full_block_hides_the_covered_slab_face_and_vice_versa() {
    let under = mesh_with(&[
        (AT - IVec3::Y, Block::Stone, 0),
        (AT, Block::OakSlab, SLAB_BOTTOM),
    ]);
    assert_eq!(under.solid.quads.len(), 5 + 5);
    let beside = mesh_with(&[
        (AT, Block::Stone, 0),
        (AT + IVec3::X, Block::OakSlab, SLAB_BOTTOM),
    ]);
    assert_eq!(beside.solid.quads.len(), 6 + 5);
    let over = mesh_with(&[
        (AT, Block::Stone, 0),
        (AT + IVec3::Y, Block::OakSlab, SLAB_BOTTOM),
    ]);
    assert_eq!(over.solid.quads.len(), 5 + 5);
}

#[test]
fn a_bottom_slab_leaves_the_neighbouring_cube_face_visible() {
    let top_slab = mesh_with(&[
        (AT, Block::Stone, 0),
        (AT + IVec3::Y, Block::OakSlab, SLAB_TOP),
    ]);
    assert_eq!(top_slab.solid.quads.len(), 6 + 6);
}

#[test]
fn a_double_slab_hides_the_face_of_a_full_block() {
    let meshes = mesh_with(&[
        (AT, Block::Stone, 0),
        (AT + IVec3::X, Block::OakSlab, SLAB_DOUBLE),
    ]);
    assert_eq!(meshes.solid.quads.len(), 10);
}

#[test]
fn straight_stair_has_eleven_visible_faces() {
    let meshes = mesh_with(&[(AT, Block::OakStairs, FACING_POS_X)]);
    assert_eq!(meshes.solid.quads.len(), 11);
    let (min, max) = bounds(&meshes.solid);
    assert_eq!((min, max), (AT.as_vec3(), AT.as_vec3() + Vec3::ONE));
}

#[test]
fn stair_step_top_covers_only_the_front_half() {
    let meshes = mesh_with(&[(AT, Block::OakStairs, FACING_POS_X)]);
    let step = quad_vertices(&meshes.solid)
        .into_iter()
        .find(|q| q.iter().all(|v| v.position()[1] == AT.y as f32 + 0.5))
        .unwrap();
    let xs: Vec<f32> = step.iter().map(|v| v.position()[0]).collect();
    assert_eq!(xs.iter().copied().fold(f32::MAX, f32::min), AT.x as f32);
    assert_eq!(
        xs.iter().copied().fold(f32::MIN, f32::max),
        AT.x as f32 + 0.5
    );
}

#[test]
fn an_outer_corner_stair_tops_out_with_a_single_quarter() {
    let meshes = mesh_with(&[
        (AT, Block::OakStairs, FACING_POS_X),
        (AT + IVec3::X, Block::OakStairs, world::block::FACING_NEG_Z),
    ]);
    let tops: Vec<_> = meshes
        .solid
        .quads
        .iter()
        .filter(|q| q.cell() == [5, 5, 5] && q.is_boxed() && q.template() == 0)
        .map(|q| q.box_rect())
        .filter(|(_, plane)| *plane == 16)
        .collect();
    assert_eq!(tops, [((8, 16, 0, 8), 16)]);
}

#[test]
fn carpet_is_a_flat_box_a_sixteenth_high() {
    let meshes = mesh_with(&[(AT, Block::Carpet, 3)]);
    assert_eq!(meshes.solid.quads.len(), 6);
    let (min, max) = bounds(&meshes.solid);
    assert_eq!(min, AT.as_vec3());
    assert_eq!(max, AT.as_vec3() + Vec3::new(1.0, 1.0 / 16.0, 1.0));
}

#[test]
fn wool_color_picks_the_tile() {
    let layer = |data| mesh_with(&[(AT, Block::Wool, data)]).solid.quads[0].layer();
    assert_eq!(u32::from(layer(0)), Tile::WoolWhite.layer());
    assert_eq!(u32::from(layer(14)), Tile::WoolRed.layer());
    let carpet = mesh_with(&[(AT, Block::Carpet, 14)]);
    assert!(
        carpet
            .solid
            .quads
            .iter()
            .all(|q| u32::from(q.layer()) == Tile::WoolRed.layer())
    );
}

#[test]
fn glass_is_cutout_and_joined_glass_shares_no_face() {
    let one = mesh_with(&[(AT, Block::Glass, 0)]);
    assert!(one.solid.quads.is_empty());
    assert_eq!(one.cutout.quads.len(), 6);
    let two = mesh_with(&[(AT, Block::Glass, 0), (AT + IVec3::X, Block::Glass, 0)]);
    assert_eq!(two.cutout.quads.len(), 10);
}

#[test]
fn slab_and_stair_faces_are_never_dark_when_the_cell_itself_gets_no_light() {
    let blocks = [
        (AT - IVec3::Y, Block::Stone, 0),
        (AT, Block::OakSlab, SLAB_BOTTOM),
        (AT + IVec3::X * 2, Block::OakStairs, FACING_POS_X),
        (AT + IVec3::new(1, 0, 3), Block::Stone, 0),
        (AT + IVec3::new(1, 0, 4), Block::OakSlab, SLAB_TOP),
    ];
    let dark: Vec<IVec3> = blocks
        .iter()
        .filter(|(_, block, _)| block.blocks_light())
        .map(|&(pos, ..)| pos)
        .collect();
    let light = |pos: IVec3| {
        if dark.contains(&pos)
            || blocks
                .iter()
                .any(|&(at, block, _)| at == pos && block.is_opaque())
        {
            0
        } else {
            MAX_LIGHT
        }
    };
    let meshes = mesh_lit(&blocks, light);
    assert!(!meshes.solid.quads.is_empty());
    for quad in &meshes.solid.quads {
        let brightest = quad.sun_halves().into_iter().max().unwrap();
        let cell = quad.cell();
        let cell = IVec3::new(cell[0].into(), cell[1].into(), cell[2].into());
        if cell.y == AT.y - 1 && !quad.is_boxed() && quad.template() != 0 {
            continue;
        }
        assert!(brightest > 0, "{quad:?}");
    }
}

#[test]
fn a_cube_beside_a_slab_is_lit_by_the_slabs_surroundings() {
    let light = |pos: IVec3| if pos == AT + IVec3::X { 0 } else { MAX_LIGHT };
    let meshes = mesh_lit(
        &[
            (AT, Block::Stone, 0),
            (AT + IVec3::X, Block::OakSlab, SLAB_BOTTOM),
        ],
        light,
    );
    let east = meshes
        .solid
        .quads
        .iter()
        .find(|q| !q.is_boxed() && q.template() == 2)
        .unwrap();
    assert!(east.sun_halves().into_iter().all(|h| h > 0));
}

#[test]
fn boxed_quad_corners_follow_the_rect() {
    let quad = crate::mesh::Quad::boxed([0, 0, 0], 0, (0, 8, 0, 16), 8, 1, [0; 2], [3; 4]);
    let data = MeshData { quads: vec![quad] };
    for vertex in vertices(&data) {
        let [x, y, z] = vertex.position();
        assert_eq!(y, 0.5);
        assert!([0.0, 0.5].contains(&x) && [0.0, 1.0].contains(&z));
        assert_eq!(vertex.uv(), [x, z]);
    }
}

fn stair_corner_quads(corner: IVec3, front: Block) -> Vec<[i32; 3]> {
    let mut world = World::new();
    for z in -1..=1 {
        for x in -1..=1 {
            world.insert(ColumnPos::new(x, z), Column::new());
        }
    }
    world.set_block_with_data(corner, Block::OakStairs, FACING_POS_X);
    world.set_block_with_data(corner + IVec3::X, front, FACING_POS_Z);
    let light = super::tests::world_light(&world);
    let meshes = mesh_world_section(
        &world,
        &light,
        SectionPos::new(corner.x.div_euclid(16), 0, 0),
        MeshOptions::default(),
    )
    .unwrap();
    let mut quads: Vec<[i32; 3]> = quad_vertices(&meshes.solid)
        .iter()
        .map(|quad| {
            let sum: Vec3 = quad.iter().map(|v| Vec3::from(v.position())).sum();
            ((sum / 4.0 - corner.as_vec3()) * 16.0)
                .round()
                .to_array()
                .map(|c| c as i32)
        })
        .filter(|c| (0..16).contains(&c[0]) && (0..16).contains(&c[2]))
        .collect();
    quads.sort();
    quads
}

#[test]
fn a_corner_stair_keeps_its_shape_on_a_section_border() {
    let middle = stair_corner_quads(IVec3::new(7, 5, 5), Block::OakStairs);
    let border = stair_corner_quads(IVec3::new(15, 5, 5), Block::OakStairs);
    let straight = stair_corner_quads(IVec3::new(7, 5, 5), Block::Air);
    assert_ne!(middle, straight);
    assert_eq!(border, middle);
}

fn cube_beside_stair_quads(stair: IVec3, data: u8) -> usize {
    let mut world = World::new();
    for z in -1..=1 {
        for x in -1..=1 {
            world.insert(ColumnPos::new(x, z), Column::new());
        }
    }
    world.set_block_with_data(stair, Block::OakStairs, data);
    let cube = stair + IVec3::X;
    world.set_block(cube, Block::Stone);
    let light = super::tests::world_light(&world);
    let meshes = mesh_world_section(
        &world,
        &light,
        SectionPos::new(cube.x.div_euclid(16), 0, 0),
        MeshOptions::default(),
    )
    .unwrap();
    quad_vertices(&meshes.solid)
        .iter()
        .filter(|quad| {
            let sum: Vec3 = quad.iter().map(|v| Vec3::from(v.position())).sum();
            let local = IVec3::new(cube.x.rem_euclid(16), cube.y, cube.z);
            let centre = sum / 4.0 - local.as_vec3();
            (0.0..1.0).contains(&centre.x) && (0.0..=1.0).contains(&centre.y)
        })
        .count()
}

#[test]
fn a_cube_beside_a_stair_on_a_section_border_is_culled_like_in_the_middle() {
    for data in [FACING_POS_X, FACING_NEG_X, FACING_POS_Z] {
        let middle = cube_beside_stair_quads(IVec3::new(7, 5, 5), data);
        let border = cube_beside_stair_quads(IVec3::new(15, 5, 5), data);
        assert_eq!(border, middle, "{data}");
    }
}

fn side_ao(blocks: &[(IVec3, Block, u8)], ambient_occlusion: bool) -> Vec<u8> {
    let mut section = Section::new();
    for &(pos, block, data) in blocks {
        section.set_with_data(pos, block, data);
    }
    let meshes = mesh_section(&section, air, full, MeshOptions { ambient_occlusion });
    let quad = meshes
        .solid
        .quads
        .iter()
        .find(|q| q.cell() == [5, 5, 5] && q.is_boxed() && q.template() == 2)
        .unwrap();
    let mut ao = quad.ao().to_vec();
    ao.sort();
    ao
}

#[test]
fn slab_faces_on_the_block_boundary_get_ambient_occlusion() {
    let slab = (AT, Block::OakSlab, SLAB_BOTTOM);
    let below = (AT + IVec3::new(1, -1, 0), Block::Stone, 0);
    assert_eq!(side_ao(&[slab], true), [3, 3, 3, 3]);
    assert_eq!(side_ao(&[slab, below], true), [2, 2, 3, 3]);
    assert_eq!(side_ao(&[slab, below], false), [3, 3, 3, 3]);
}
