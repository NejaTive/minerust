use super::*;

const FLOOR: i32 = 10;

fn ground(pos: IVec3) -> Collision {
    if pos.y <= FLOOR {
        Collision::FULL
    } else {
        Collision::NONE
    }
}

fn primed(fuse: i32) -> PrimedTnt {
    PrimedTnt::at(IVec3::new(0, FLOOR + 5, 0), fuse, &mut Rng::new(9))
}

#[test]
fn starts_with_the_vanilla_kick() {
    let tnt = primed(FUSE_TICKS);
    assert_eq!(tnt.velocity.y, 0.2);
    assert!((horizontal(&tnt).length() - 0.02).abs() < 1e-6);
    assert_eq!(tnt.pos, Vec3::new(0.5, 15.0, 0.5));
}

#[test]
fn kick_direction_depends_on_the_seed() {
    let a = PrimedTnt::at(IVec3::ZERO, FUSE_TICKS, &mut Rng::new(1));
    let b = PrimedTnt::at(IVec3::ZERO, FUSE_TICKS, &mut Rng::new(1));
    let c = PrimedTnt::at(IVec3::ZERO, FUSE_TICKS, &mut Rng::new(2));
    assert_eq!(a.velocity, b.velocity);
    assert_ne!(a.velocity, c.velocity);
}

#[test]
fn fuse_burns_for_eighty_ticks() {
    let mut tnt = primed(FUSE_TICKS);
    for _ in 0..FUSE_TICKS - 1 {
        assert!(!tnt.tick(ground));
    }
    assert!(tnt.tick(ground));
}

#[test]
fn hops_up_then_falls_and_rests_on_the_floor() {
    let mut tnt = primed(FUSE_TICKS);
    tnt.tick(ground);
    assert!(tnt.pos.y > 15.0);
    for _ in 0..60 {
        tnt.tick(ground);
    }
    assert!(
        (tnt.pos.y - (FLOOR + 1) as f32).abs() < 1e-3,
        "{}",
        tnt.pos.y
    );
    assert!(horizontal(&tnt).length() < 0.005);
}

#[test]
fn ground_slows_horizontal_motion() {
    let mut tnt = primed(FUSE_TICKS);
    tnt.pos.y = (FLOOR + 1) as f32;
    tnt.velocity = Vec3::new(0.5, 0.0, 0.0);
    tnt.tick(ground);
    let sliding = tnt.velocity.x;
    tnt.tick(ground);
    assert!(sliding < 0.5 * DRAG);
    assert!(tnt.velocity.x < sliding * DRAG);
}

#[test]
fn walls_stop_horizontal_motion() {
    let wall = |pos: IVec3| {
        if pos.x >= 3 || pos.y <= FLOOR {
            Collision::FULL
        } else {
            Collision::NONE
        }
    };
    let mut tnt = primed(FUSE_TICKS);
    tnt.pos = Vec3::new(2.0, (FLOOR + 1) as f32, 0.5);
    tnt.velocity = Vec3::new(1.0, 0.0, 0.0);
    tnt.tick(wall);
    assert!((tnt.aabb().1.x - 3.0).abs() < 1e-3);
}

#[test]
fn swells_only_in_the_last_ten_ticks() {
    assert_eq!(swell(40.0), 1.0);
    assert_eq!(swell(10.0), 1.0);
    assert!(swell(5.0) > 1.0);
    assert!((swell(0.0) - 1.3).abs() < 1e-6);
}

fn horizontal(tnt: &PrimedTnt) -> glam::Vec2 {
    glam::Vec2::new(tnt.velocity.x, tnt.velocity.z)
}
