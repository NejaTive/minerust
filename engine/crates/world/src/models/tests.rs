use super::*;
use crate::block::{FACING_NEG_X, FACING_NEG_Z, FACING_POS_X, FACING_POS_Z};

fn tip(frame: Frame, element: &Element) -> Vec3 {
    let top = [8.0, element.to[1], 8.0];
    let p = element.rotation.map_or(top, |r| rotate(top, r));
    Vec3::from(frame.point(p))
}

#[test]
fn lever_elements_keep_their_vanilla_pixels() {
    assert_eq!(LEVER_BASE, ([5.0, 0.0, 4.0], [11.0, 3.0, 12.0]));
    let handle = lever_handle(false);
    assert_eq!(
        (handle.from, handle.to),
        ([7.0, 1.0, 7.0], [9.0, 11.0, 9.0])
    );
    assert_eq!(handle.uv[2], [7.0, 6.0, 9.0, 16.0]);
    assert_eq!(handle.uv[0], [7.0, 6.0, 9.0, 8.0]);
    let r = handle.rotation.unwrap();
    assert_eq!((r.origin, r.axis, r.angle), ([8.0, 1.0, 8.0], 0, -45.0));
    assert_eq!(lever_handle(true).rotation.unwrap().angle, 45.0);
}

#[test]
fn frames_are_proper_rotations() {
    for data in Block::Lever.states() {
        let f = lever_frame(data);
        assert_eq!(f.x.cross(f.y), f.z, "{data}");
        assert_eq!(f.x.abs().max_element(), 1);
    }
}

#[test]
fn wall_lever_base_sits_against_the_support_and_the_handle_leans_up_when_on() {
    for (code, back) in [
        (FACING_POS_X, IVec3::NEG_X),
        (FACING_NEG_X, IVec3::X),
        (FACING_POS_Z, IVec3::NEG_Z),
        (FACING_NEG_Z, IVec3::Z),
    ] {
        let frame = lever_frame(code);
        let base = framed_box(frame, LEVER_BASE);
        let against = if back.x + back.z > 0 { 16 } else { 0 };
        let axis = back.abs().max_position();
        let wall = if back.max_element() > 0 {
            base.max
        } else {
            base.min
        };
        assert_eq!(wall[axis], against, "{code}");
        let off = tip(frame, &lever_handle(false));
        let on = tip(frame, &lever_handle(true));
        assert!(on.y > off.y + 6.0, "{code}: {on} {off}");
        let outward = -back.as_vec3();
        assert!((on - Vec3::splat(8.0)).dot(outward) > 0.0);
    }
}

#[test]
fn floor_and_ceiling_levers_stand_away_from_their_support() {
    let floor = lever_frame(LEVER_FLOOR);
    assert_eq!(framed_box(floor, LEVER_BASE).min[1], 0);
    assert!(tip(floor, &lever_handle(true)).y > 8.0);
    let ceiling = lever_frame(LEVER_CEILING);
    assert_eq!(framed_box(ceiling, LEVER_BASE).max[1], 16);
    assert!(tip(ceiling, &lever_handle(true)).y < 8.0);
}

#[test]
fn hook_plate_is_a_narrow_board_on_the_wall_and_the_arm_swings_with_the_state() {
    let frame = hook_frame(FACING_POS_X);
    let plate = framed_box(frame, HOOK_PLATE);
    assert_eq!((plate.min[0], plate.max[0]), (0, 2));
    assert_eq!(plate.max[1] - plate.min[1], 8);
    assert_eq!(plate.max[2] - plate.min[2], 4);
    let calm = tip(frame, &hook_arm(false));
    let tripped = tip(frame, &hook_arm(true));
    assert!(calm.x > 5.0 && calm.y < 8.0);
    assert!(tripped.y > calm.y);
    let ring = hook_ring(false);
    assert_eq!(ring.uv[4], HOOK_RING_UV);
    assert_eq!(ring.to[0] - ring.from[0], 6.0);
}

#[test]
fn selection_boxes_cover_the_swinging_handle() {
    for data in Block::Lever.states() {
        let [_, handle] = lever_boxes(data);
        assert!(!handle.is_flat(), "{data}");
    }
    for data in Block::TripwireHook.states() {
        let [plate, arm] = hook_boxes(data);
        assert!(!plate.is_flat() && !arm.is_flat(), "{data}");
    }
}
