use glam::IVec3;
use world::block::{Block, Facing, RenderLayer, Shape};
use world::boxes::{FaceMask, face_axes};
use world::section::SECTION_SIZE;
use world::tile::Tile;

use super::{
    CornerStrides, Datas, Lights, MeshData, Neighborhood, PADDED, half_average, vertex_ao,
};
use crate::mesh::Quad;
use crate::templates::{FACES, Face, POT_CROSS, POT_CROSS_PARTS};

const SPECIAL_NEIGHBOR: [bool; 256] = {
    let mut table = [false; 256];
    let mut i = 0;
    while i < world::block::BLOCKS.len() {
        let def = &world::block::BLOCKS[i];
        table[i] = matches!(def.shape, Shape::Boxes) || def.joins_same;
        i += 1;
    }
    table
};

const NEIGHBOR_LIT: [bool; 256] = {
    let mut table = [false; 256];
    let mut i = 0;
    while i < world::block::BLOCKS.len() {
        let def = &world::block::BLOCKS[i];
        table[i] = def.light_blocking && !def.opaque;
        i += 1;
    }
    table
};

const FIRST_NEIGHBOR_LIT: u8 = {
    let mut i = 0;
    while !NEIGHBOR_LIT[i] {
        i += 1;
    }
    i as u8
};

fn cell_at(blocks: &Neighborhood, datas: &Datas, local: IVec3) -> (Block, u8) {
    if local.cmplt(IVec3::splat(-1)).any() || local.cmpgt(IVec3::splat(SECTION_SIZE)).any() {
        return (Block::Air, 0);
    }
    let index = Neighborhood::index(local);
    (blocks.0[index], datas.0[index])
}

/// Плиты и ступени не пропускают свет, поэтому их клетка берёт свет у соседей, кроме нижнего.
pub(super) fn settle_lights(blocks: &Neighborhood, lights: &mut Lights) {
    if blocks.0.iter().map(|&b| b as u8).max() < Some(FIRST_NEIGHBOR_LIT) {
        return;
    }
    let mut updates = Vec::new();
    for (index, block) in blocks.0.iter().enumerate() {
        if !NEIGHBOR_LIT[*block as usize] {
            continue;
        }
        let side = PADDED as usize;
        let (x, z, y) = (index % side, index / side % side, index / (side * side));
        let mut best = (0, 0);
        let mut take = |dx: i32, dy: i32, dz: i32| {
            let (nx, ny, nz) = (x as i32 + dx, y as i32 + dy, z as i32 + dz);
            if [nx, ny, nz].iter().any(|&c| !(0..PADDED).contains(&c)) {
                return;
            }
            let packed = lights.0[(nx + PADDED * nz + PADDED * PADDED * ny) as usize];
            let (sky, lamp) = (packed & 0xF, packed >> 4);
            best = (best.0.max(sky), best.1.max(lamp));
        };
        for (dx, dy, dz) in [(1, 0, 0), (-1, 0, 0), (0, 0, 1), (0, 0, -1), (0, 1, 0)] {
            take(dx, dy, dz);
        }
        updates.push((index, best.0 | best.1 << 4));
    }
    for (index, packed) in updates {
        lights.0[index] = packed;
    }
}

pub(super) fn is_special_neighbor(block: Block) -> bool {
    SPECIAL_NEIGHBOR[block as usize]
}

fn face_axis(face_index: usize) -> (usize, bool) {
    let normal = FACES[face_index].normal;
    let axis = normal.abs().max_position();
    (axis, normal[axis] > 0)
}

/// Покрытие грани соседа, повёрнутой к нам; клетки за кэшем (2 от секции) считаются воздухом.
fn neighbor_mask(
    blocks: &Neighborhood,
    datas: &Datas,
    local: IVec3,
    face_index: usize,
) -> FaceMask {
    let neighbor_local = local + FACES[face_index].normal;
    let (block, data) = cell_at(blocks, datas, neighbor_local);
    if block.is_opaque() {
        return FaceMask::FULL;
    }
    if block.shape() != Shape::Boxes {
        return FaceMask::EMPTY;
    }
    let (axis, positive) = face_axis(face_index);
    block
        .boxes(data, |offset| {
            cell_at(blocks, datas, neighbor_local + offset)
        })
        .face_mask(axis, !positive)
}

/// Грань полного блока скрыта соседней коробочной формой или соседним стеклом.
pub(super) fn hides_cube_face(
    blocks: &Neighborhood,
    datas: &Datas,
    local: IVec3,
    block: Block,
    face_index: usize,
) -> bool {
    let neighbor_local = local + FACES[face_index].normal;
    let (neighbor, _) = cell_at(blocks, datas, neighbor_local);
    if block.joins_same() && neighbor == block {
        return true;
    }
    neighbor.shape() == Shape::Boxes
        && neighbor_mask(blocks, datas, local, face_index).contains(&FaceMask::FULL)
}

pub(super) fn push_boxes(
    meshes: (&mut MeshData, &mut MeshData),
    blocks: &Neighborhood,
    datas: &Datas,
    lights: &Lights,
    index: usize,
    faces: &[(&Face, isize, CornerStrides); 6],
    ambient_occlusion: bool,
) {
    let side = PADDED as usize;
    let local = IVec3::new(
        (index % side) as i32 - 1,
        (index / (side * side)) as i32 - 1,
        (index / side % side) as i32 - 1,
    );
    let block = blocks.0[index];
    let data = datas.0[index];
    let target = if block.render_layer() == RenderLayer::Cutout {
        meshes.1
    } else {
        meshes.0
    };
    let cell = [local.x as u8, local.y as u8, local.z as u8];
    let list = block.boxes(data, |offset| cell_at(blocks, datas, local + offset));
    for (box_index, voxel) in list.iter().enumerate() {
        if voxel.is_flat() {
            continue;
        }
        for (face_index, face) in FACES.iter().enumerate() {
            let (axis, positive) = face_axis(face_index);
            let plane = if positive {
                voxel.max[axis]
            } else {
                voxel.min[axis]
            };
            let (ua, va) = face_axes(axis);
            let rect = |b: &world::boxes::VoxelBox| {
                FaceMask::rect(b.min[ua], b.max[ua], b.min[va], b.max[va])
            };
            let mut mask = rect(voxel);
            for (other_index, other) in list.iter().enumerate() {
                let spans = if positive {
                    other.min[axis] <= plane && other.max[axis] > plane
                } else {
                    other.max[axis] >= plane && other.min[axis] < plane
                };
                if other_index != box_index && spans {
                    mask.subtract(&rect(other));
                }
            }
            if plane == if positive { 16 } else { 0 } {
                mask.subtract(&neighbor_mask(blocks, datas, local, face_index));
            }
            if mask.is_empty() {
                continue;
            }
            let Some(tile) = block.box_tile(box_index, face.facing, face.normal, data) else {
                continue;
            };
            let (sky, lamp) = lights.at(index, Neighborhood::stride(face.normal));
            let light = [sky, lamp].map(|l| half_average(l, 1));
            let on_boundary = plane == if positive { 16 } else { 0 };
            for rect in mask.rects() {
                let ao = if ambient_occlusion && on_boundary {
                    std::array::from_fn(|i| {
                        let corner = face.corners[i];
                        let at_block_corner = [(ua, rect.0, rect.1), (va, rect.2, rect.3)]
                            .iter()
                            .all(|&(a, lo, hi)| match corner[a] {
                                0 => lo == 0,
                                _ => hi == 16,
                            });
                        if at_block_corner {
                            let [side1, side2, diagonal] = faces[face_index].2[i];
                            vertex_ao(
                                blocks.opaque(index, side1),
                                blocks.opaque(index, side2),
                                blocks.opaque(index, diagonal),
                            )
                        } else {
                            3
                        }
                    })
                } else {
                    [3; 4]
                };
                target.quads.push(Quad::boxed(
                    cell,
                    face_index as u8,
                    rect,
                    plane,
                    tile.layer() as u16,
                    light,
                    ao,
                ));
            }
        }
    }
    push_contents(target, block, data, cell, light_of(lights, index));
}

fn light_of(lights: &Lights, index: usize) -> [u8; 2] {
    let (sky, lamp) = lights.at(index, 0);
    [sky, lamp].map(|l| half_average(l, 1))
}

fn push_contents(target: &mut MeshData, block: Block, data: u8, cell: [u8; 3], light: [u8; 2]) {
    if block == Block::Cauldron
        && let Some(plane) = Block::cauldron_water_units(data)
    {
        target.quads.push(Quad::boxed(
            cell,
            0,
            (2, 14, 2, 14),
            plane,
            Tile::Water.layer() as u16,
            light,
            [3; 4],
        ));
    }
    if block == Block::FlowerPot
        && let Some(plant) = Block::pot_plant(data)
        && let Some(tile) = plant.tile(Facing::Side)
    {
        for template in POT_CROSS..POT_CROSS + POT_CROSS_PARTS {
            target.quads.push(Quad::new(
                cell,
                template,
                false,
                tile.layer() as u16,
                [3; 4],
                [light[0]; 4],
                [light[1]; 4],
            ));
        }
    }
}
