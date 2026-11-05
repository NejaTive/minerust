use super::*;
use crate::templates::{TABLE_BYTES, TEMPLATE_COUNT, WIRE_PARTS, templates};
use world::block::{Block, FACING_POS_X, HOOK_POWERED};

#[test]
fn every_state_has_its_own_run_of_templates() {
    assert_eq!(LEVER_STATES, 24);
    assert_eq!(
        lever_templates().len(),
        usize::from(LEVER_STATES * LEVER_PARTS)
    );
    assert_eq!(
        hook_templates().len(),
        usize::from(HOOK_STATES * HOOK_PARTS)
    );
    assert_eq!(wire_templates().len(), usize::from(WIRE_PARTS));
    assert_eq!(WIRE + WIRE_PARTS, TEMPLATE_COUNT as u16);
    assert_eq!(TABLE_BYTES, TEMPLATE_COUNT * 32 + 16);
    let mut firsts: Vec<u16> = Block::Lever.states().map(lever_first).collect();
    firsts.sort_unstable();
    firsts.dedup();
    assert_eq!(firsts.len(), usize::from(LEVER_STATES));
    let mut hooks: Vec<u16> = (0..8).map(hook_first).collect();
    hooks.sort_unstable();
    hooks.dedup();
    assert_eq!(hooks.len(), usize::from(HOOK_STATES));
}

#[test]
fn handle_side_faces_take_the_vanilla_uv_strip() {
    let handle = templates()[usize::from(lever_first(FACING_POS_X)) + 2];
    let us: Vec<u8> = handle.corners.iter().map(|c| c.uv[0]).collect();
    let vs: Vec<u8> = handle.corners.iter().map(|c| c.uv[1]).collect();
    assert_eq!((us.iter().min(), us.iter().max()), (Some(&7), Some(&9)));
    assert_eq!((vs.iter().min(), vs.iter().max()), (Some(&6), Some(&16)));
    let cap = templates()[usize::from(lever_first(FACING_POS_X))];
    assert!(
        cap.corners
            .iter()
            .all(|c| (7..=9).contains(&c.uv[0]) && (6..=8).contains(&c.uv[1]))
    );
}

#[test]
fn handle_quads_lie_inside_the_cell_and_lean_with_the_state() {
    let top = |data: u8| {
        let first = usize::from(lever_first(data));
        (0..6)
            .flat_map(|part| templates()[first + part].corners.map(|c| c.pos))
            .inspect(|p| assert!(p.iter().all(|&v| v <= 16), "{p:?}"))
            .map(|p| p[1])
            .max()
            .unwrap()
    };
    for facing in 0..4 {
        assert!(top(facing | LEVER_POWERED) > top(facing), "{facing}");
    }
}

#[test]
fn hook_arm_points_down_when_calm_and_up_when_tripped_and_the_ring_follows() {
    let part = |data: u8, parts: std::ops::Range<usize>| {
        let first = usize::from(hook_first(data));
        parts
            .flat_map(|p| templates()[first + p].corners.map(|c| c.pos))
            .collect::<Vec<_>>()
    };
    let low = |points: &[[u8; 3]]| points.iter().map(|p| p[1]).min().unwrap();
    let high = |points: &[[u8; 3]]| points.iter().map(|p| p[1]).max().unwrap();
    let calm = part(FACING_POS_X, 0..6);
    let tripped = part(FACING_POS_X | HOOK_POWERED, 0..6);
    assert!(low(&calm) < 8 && high(&tripped) > 8);
    let ring_calm = part(FACING_POS_X, 6..8);
    assert!(
        ring_calm
            .iter()
            .all(|p| p[1] == ring_calm[0][1] || p[1].abs_diff(ring_calm[0][1]) <= 1)
    );
    let ring_tripped = part(FACING_POS_X | HOOK_POWERED, 6..8);
    assert!(high(&ring_tripped) - low(&ring_tripped) >= 4);
    let uv: Vec<u8> = templates()[usize::from(hook_first(FACING_POS_X)) + 6]
        .corners
        .iter()
        .map(|c| c.uv[0])
        .collect();
    assert_eq!((uv.iter().min(), uv.iter().max()), (Some(&5), Some(&11)));
}

#[test]
fn wire_is_a_flat_two_sided_quad_with_the_whole_tile() {
    for along_z in [false, true] {
        let first = usize::from(wire_first(along_z));
        for part in 0..2 {
            let t = templates()[first + part];
            assert!(t.corners.iter().all(|c| c.pos[1] == 2));
            let us: Vec<u8> = t.corners.iter().map(|c| c.uv[0]).collect();
            assert_eq!((us.iter().min(), us.iter().max()), (Some(&0), Some(&16)));
        }
    }
    let front = templates()[usize::from(wire_first(false))];
    let back = templates()[usize::from(wire_first(false)) + 1];
    assert_eq!(front.corners[0], back.corners[3]);
}
