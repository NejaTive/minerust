use super::*;

#[test]
fn turning_four_times_is_identity_and_rotates_clockwise() {
    let back = VoxelBox::new([8, 8, 0], [16, 16, 16]);
    assert_eq!(back.turned(4), back);
    assert_eq!(back.turned(1), VoxelBox::new([0, 8, 8], [16, 16, 16]));
    assert_eq!(back.turned(2), VoxelBox::new([0, 8, 0], [8, 16, 16]));
}

#[test]
fn flipping_mirrors_the_height() {
    let slab = VoxelBox::new([0, 0, 0], [16, 8, 16]);
    assert_eq!(slab.flipped_y(), VoxelBox::new([0, 8, 0], [16, 16, 16]));
}

#[test]
fn face_mask_covers_only_boxes_on_the_plane() {
    let list = BoxList::of(&[VoxelBox::new([0, 0, 0], [16, 8, 16])]);
    assert_eq!(list.face_mask(1, false), FaceMask::FULL);
    assert!(list.face_mask(1, true).is_empty());
    let side = list.face_mask(0, true);
    assert_eq!(side.0[0], u16::MAX);
    assert_eq!(side.0[7], u16::MAX);
    assert_eq!(side.0[8], 0);
}

#[test]
fn rects_cover_the_mask_exactly() {
    let mut mask = FaceMask::rect(0, 16, 0, 8);
    mask.subtract(&FaceMask::rect(8, 16, 4, 8));
    let rects = mask.rects();
    let mut rebuilt = FaceMask::EMPTY;
    for (u0, u1, v0, v1) in rects {
        let part = FaceMask::rect(u0, u1, v0, v1);
        assert!(mask.contains(&part));
        rebuilt.fill(u0, u1, v0, v1);
    }
    assert_eq!(rebuilt, mask);
}

#[test]
fn top_is_the_highest_box() {
    let list = BoxList::of(&[
        VoxelBox::new([0, 0, 0], [16, 8, 16]),
        VoxelBox::new([8, 8, 0], [16, 16, 16]),
    ]);
    assert_eq!(list.top(), 1.0);
    assert_eq!(BoxList::EMPTY.top(), 0.0);
}
