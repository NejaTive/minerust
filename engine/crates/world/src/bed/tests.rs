use super::*;
use crate::block::{Block, FACING_POS_X, FACING_POS_Z};
use crate::boxes::VoxelBox;
use crate::tile::Tile;

fn sheet() -> DecodedImage {
    let mut rgba = vec![0u8; 64 * 64 * 4];
    for (i, pixel) in rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        pixel.copy_from_slice(&[(i % 64) as u8, (i / 64) as u8, 9, 255]);
    }
    DecodedImage {
        width: 64,
        height: 64,
        rgba,
    }
}

fn pixel(image: &DecodedImage, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * image.width + x) * 4) as usize;
    [
        image.rgba[i],
        image.rgba[i + 1],
        image.rgba[i + 2],
        image.rgba[i + 3],
    ]
}

#[test]
fn data_round_trips_facing_half_and_colour() {
    for color in 0..COLOR_COUNT {
        for head in [false, true] {
            for dir in [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z] {
                let data = data(dir, head, color);
                assert_eq!(toward_head(data), dir);
                assert_eq!(is_head(data), head);
                assert_eq!(color_of(data), color);
                assert_eq!(partner_offset(data), if head { -dir } else { dir });
            }
        }
    }
}

#[test]
fn every_bed_state_is_listed_for_the_block() {
    let states: Vec<u8> = Block::Bed.states().collect();
    assert_eq!(states.len(), 128);
    assert!(states.contains(&data(IVec3::Z, true, 15)));
}

#[test]
fn top_tile_copies_the_model_top_of_each_half() {
    let sheet = sheet();
    let head = compose_top(true, &sheet);
    assert_eq!((head.width, head.height), (16, 16));
    assert_eq!(pixel(&head, 0, 0)[..2], [6, 6]);
    assert_eq!(pixel(&head, 15, 15)[..2], [21, 21]);
    let foot = compose_top(false, &sheet);
    assert_eq!(pixel(&foot, 0, 0)[..2], [6, 28]);
}

#[test]
fn side_tile_reads_the_strip_from_the_mattress_top_down() {
    let side = compose_side(true, &sheet());
    assert_eq!(pixel(&side, 4, 7)[..2], [5, 10]);
    assert_eq!(pixel(&side, 4, 12)[..2], [0, 10]);
    assert_eq!(pixel(&side, 4, 0)[..2], [5, 10]);
    assert_eq!(pixel(&side, 4, 15)[..2], [0, 10]);
    assert!(side.rgba.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
}

#[test]
fn tiles_are_scaled_with_the_sheet() {
    let mut big = sheet();
    big.width = 128;
    big.height = 128;
    big.rgba = vec![200; 128 * 128 * 4];
    assert_eq!(compose_top(true, &big).width, 32);
}

#[test]
fn bed_tiles_cover_every_colour_and_part() {
    for color in 0..COLOR_COUNT {
        for (head, top) in [(true, true), (false, true), (true, false), (false, false)] {
            assert_eq!(
                Tile::bed(color, head, top).bed_part(),
                Some((color, head, top))
            );
        }
    }
    assert_eq!(Tile::Stone.bed_part(), None);
}

#[test]
fn bed_is_nine_sixteenths_tall_and_walls_off_its_legs() {
    let data = data(IVec3::NEG_Z, true, 0);
    let boxes = Block::Bed.boxes(data, |_| (Block::Air, 0));
    assert_eq!(boxes.bounds().1.y, 9.0 / 16.0);
    let collision = Block::Bed.collision_boxes(data, |_| (Block::Air, 0));
    assert_eq!(
        collision.as_slice(),
        &[VoxelBox::new([0, 0, 0], [16, 9, 16])]
    );
    let east = Block::Bed.boxes(data | FACING_POS_X, |_| (Block::Air, 0));
    let south_foot = Block::Bed.boxes(FACING_POS_Z, |_| (Block::Air, 0));
    assert_eq!(east.len(), 3);
    assert_eq!(south_foot.len(), 3);
}
