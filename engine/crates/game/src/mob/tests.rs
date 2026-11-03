use super::*;

const FLOOR: i32 = 63;

fn flat(pos: IVec3) -> Collision {
    if pos.y <= FLOOR {
        Collision::FULL
    } else {
        Collision::NONE
    }
}

fn dry(_: IVec3) -> Option<(FluidKind, f32)> {
    None
}

fn mob(kind: MobKind, seed: u32) -> Mob {
    Mob::new(kind, Vec3::new(0.5, 64.0, 0.5), 0.0, seed)
}

fn pig(seed: u32) -> Mob {
    mob(MobKind::Pig, seed)
}

fn run<C, F>(mob: &mut Mob, ticks: u32, collision: &C, fluid: &F)
where
    C: Fn(IVec3) -> Collision,
    F: Fn(IVec3) -> Option<(FluidKind, f32)>,
{
    let around = Surroundings {
        collision,
        fluid,
        player_eye: Vec3::new(2.0, 65.6, 0.5),
    };
    for _ in 0..ticks {
        mob.tick(&around);
    }
}

#[test]
fn hitboxes_match_minecraft() {
    for (kind, width, height) in [
        (MobKind::Pig, 0.9, 0.9),
        (MobKind::Chicken, 0.4, 0.7),
        (MobKind::Cow, 0.9, 1.4),
        (MobKind::Sheep, 0.9, 1.3),
    ] {
        let (min, max) = mob(kind, 1).body.aabb();
        let expected = Vec3::new(width, height, width);
        assert!(
            (max - min - expected).abs().max_element() < 1e-4,
            "{kind:?}"
        );
    }
}

#[test]
fn wandering_is_deterministic_and_stays_on_the_ground() {
    let (mut a, mut b) = (pig(7), pig(7));
    run(&mut a, 3000, &flat, &dry);
    run(&mut b, 3000, &flat, &dry);
    assert_eq!(a.body.feet, b.body.feet);
    assert_eq!(a.body.feet.y, 64.0);
    assert!(a.body.feet.distance(Vec3::new(0.5, 64.0, 0.5)) > 1.0);
}

#[test]
fn falls_to_the_floor() {
    let mut mob = pig(3);
    mob.body.feet.y = 70.0;
    run(&mut mob, 120, &flat, &dry);
    assert_eq!(mob.body.feet.y, 64.0);
    assert!(mob.body.on_ground);
}

#[test]
fn does_not_walk_off_a_tall_cliff() {
    let edge = |pos: IVec3| {
        let ground = if pos.x < 0 { 50 } else { FLOOR };
        if pos.y <= ground {
            Collision::FULL
        } else {
            Collision::NONE
        }
    };
    for seed in 1..6 {
        let mut mob = Mob::new(MobKind::Pig, Vec3::new(1.5, 64.0, 0.5), 0.0, seed);
        run(&mut mob, 6000, &edge, &dry);
        assert!(mob.body.feet.y > 63.9, "seed {seed}: {:?}", mob.body.feet);
    }
}

#[test]
fn steps_up_one_block() {
    let wall = |pos: IVec3| {
        if pos.y <= FLOOR || pos.x >= 3 && pos.y == 64 {
            Collision::FULL
        } else {
            Collision::NONE
        }
    };
    let mut mob = pig(1);
    mob.plan = Plan::Walk {
        target: Vec2::new(6.0, 0.5),
        ticks: 0,
        checkpoint: Vec2::new(0.5, 0.5),
    };
    run(&mut mob, 600, &wall, &dry);
    assert!(mob.body.feet.x > 3.0);
    assert!(mob.body.feet.y >= 64.9);
}

#[test]
fn floats_in_water() {
    let water = |pos: IVec3| (pos.y > FLOOR && pos.y <= 66).then_some((FluidKind::Water, 1.0));
    let mut mob = pig(5);
    mob.body.feet.y = 64.0;
    run(&mut mob, 600, &flat, &water);
    assert!(mob.body.feet.y > 65.0, "{}", mob.body.feet.y);
}

#[test]
fn record_round_trips() {
    let mob = Mob::new(MobKind::Pig, Vec3::new(4.0, 70.0, -3.0), 1.25, 9);
    let again = Mob::from_record(&mob.record(), 9);
    assert_eq!(again.record(), mob.record());
}

#[test]
fn angle_difference_wraps() {
    assert!((angle_diff(0.1, TAU - 0.1) - 0.2).abs() < 1e-5);
}

fn walk_east(mob: &mut Mob) {
    mob.plan = Plan::Walk {
        target: Vec2::new(6.0, 0.5),
        ticks: 0,
        checkpoint: Vec2::new(0.5, 0.5),
    };
}

fn cells_above_floor(blocks: &'static [(i32, i32)], from_x: i32) -> impl Fn(IVec3) -> Collision {
    move |pos| {
        let solid = pos.y <= FLOOR
            || pos.x >= from_x && blocks.iter().any(|&(lo, hi)| (lo..=hi).contains(&pos.y));
        if solid {
            Collision::FULL
        } else {
            Collision::NONE
        }
    }
}

#[test]
fn hops_onto_a_block_without_snapping_up() {
    let step = cells_above_floor(&[(64, 64)], 3);
    let mut mob = pig(1);
    walk_east(&mut mob);
    let around = Surroundings {
        collision: &step,
        fluid: &dry,
        player_eye: Vec3::new(0.5, 65.6, 0.5),
    };
    let mut worst_rise: f32 = 0.0;
    for _ in 0..600 {
        let before = mob.body.feet.y;
        mob.tick(&around);
        worst_rise = worst_rise.max(mob.body.feet.y - before);
    }
    assert!(
        mob.body.feet.x > 3.0 && mob.body.feet.y >= 64.99,
        "{:?}",
        mob.body.feet
    );
    assert!(worst_rise < 0.5, "{worst_rise}");
}

#[test]
fn does_not_jump_at_a_two_block_wall() {
    let wall = cells_above_floor(&[(64, 65)], 3);
    let mut mob = pig(1);
    walk_east(&mut mob);
    let mut peak: f32 = 0.0;
    let around = Surroundings {
        collision: &wall,
        fluid: &dry,
        player_eye: Vec3::new(0.5, 65.6, 0.5),
    };
    for _ in 0..600 {
        mob.tick(&around);
        peak = peak.max(mob.body.feet.y);
    }
    assert!(mob.body.feet.x < 3.0);
    assert_eq!(peak, 64.0);
}

#[test]
fn does_not_jump_under_a_ceiling() {
    let low = |pos: IVec3| {
        let solid = pos.y <= FLOOR || pos.x >= 3 && pos.y == 64 || pos.y == 65;
        if solid {
            Collision::FULL
        } else {
            Collision::NONE
        }
    };
    let mut mob = pig(1);
    walk_east(&mut mob);
    let mut peak: f32 = 0.0;
    let around = Surroundings {
        collision: &low,
        fluid: &dry,
        player_eye: Vec3::new(0.5, 65.6, 0.5),
    };
    for _ in 0..600 {
        mob.tick(&around);
        peak = peak.max(mob.body.feet.y);
    }
    assert!(mob.body.feet.x < 3.0);
    assert_eq!(peak, 64.0);
}

#[test]
fn every_kind_wanders_and_stays_on_the_ground() {
    for kind in MobKind::ALL {
        let mut mob = mob(kind, 7);
        run(&mut mob, 3000, &flat, &dry);
        assert_eq!(mob.body.feet.y, 64.0, "{kind:?}");
        assert!(
            mob.body.feet.distance(Vec3::new(0.5, 64.0, 0.5)) > 1.0,
            "{kind:?}"
        );
    }
}

fn fall_speed(kind: MobKind) -> f32 {
    let mut mob = mob(kind, 3);
    mob.body.feet.y = 200.0;
    let mut fastest = 0.0_f32;
    let around = Surroundings {
        collision: &flat,
        fluid: &dry,
        player_eye: Vec3::ZERO,
    };
    for _ in 0..90 {
        mob.tick(&around);
        fastest = fastest.max(-mob.body.velocity.y);
    }
    fastest
}

#[test]
fn chicken_falls_slowly() {
    assert!(fall_speed(MobKind::Chicken) <= 3.9 + 1e-3);
    assert!(fall_speed(MobKind::Pig) > 10.0);
}

#[test]
fn chicken_flaps_in_the_air_and_settles_on_the_ground() {
    let mut chicken = mob(MobKind::Chicken, 3);
    chicken.body.feet.y = 70.0;
    run(&mut chicken, 20, &flat, &dry);
    let airborne = chicken.pose();
    assert!(airborne.flap_amount > 0.9);
    assert!(airborne.flap_phase > 1.0);
    run(&mut chicken, 200, &flat, &dry);
    assert!(chicken.body.on_ground);
    assert_eq!(chicken.pose().flap_amount, 0.0);
}

#[test]
fn other_kinds_never_flap() {
    let mut cow = mob(MobKind::Cow, 3);
    cow.body.feet.y = 70.0;
    run(&mut cow, 40, &flat, &dry);
    assert_eq!(cow.pose().flap_amount, 0.0);
    assert_eq!(cow.pose().flap_phase, 0.0);
}
