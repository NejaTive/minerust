use super::*;

fn body(x: f32, z: f32) -> Aabb {
    (
        Vec3::new(x - 0.45, 64.0, z - 0.45),
        Vec3::new(x + 0.45, 64.9, z + 0.45),
    )
}

#[test]
fn separate_bodies_do_not_overlap() {
    assert_eq!(overlap_depth(body(0.0, 0.0), body(1.0, 0.0)), None);
}

#[test]
fn deeper_overlap_pushes_harder() {
    let shallow = push_away(body(0.0, 0.0), body(0.8, 0.0), 0.1, 0).length();
    let deep = push_away(body(0.0, 0.0), body(0.2, 0.0), 0.8, 0).length();
    assert!(deep > shallow);
}

#[test]
fn push_points_away_from_the_first_body() {
    let push = push_away(body(0.0, 0.0), body(0.5, 0.0), 0.4, 0);
    assert!(push.x > 0.0 && push.y.abs() < 1e-6);
}

#[test]
fn coincident_bodies_still_separate() {
    let push = push_away(body(0.0, 0.0), body(0.0, 0.0), 1.0, 5);
    assert!((push.length() - PUSH_PER_TICK).abs() < 1e-6);
}
