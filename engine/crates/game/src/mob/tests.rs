use world::block::Block;

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
        (MobKind::Mooshroom, 0.9, 1.4),
        (MobKind::PolarBear, 1.3, 1.4),
        (MobKind::Wolf, 0.6, 0.85),
        (MobKind::Rabbit, 0.4, 0.5),
        (MobKind::Ocelot, 0.6, 0.7),
        (MobKind::Parrot, 0.5, 0.9),
        (MobKind::Squid, 0.8, 0.8),
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
        if matches!(kind, MobKind::Parrot | MobKind::Squid) {
            continue;
        }
        let mut mob = mob(kind, 7);
        run(&mut mob, 3000, &flat, &dry);
        if kind == MobKind::Rabbit {
            assert!((64.0..64.5).contains(&mob.body.feet.y), "{kind:?}");
        } else {
            assert_eq!(mob.body.feet.y, 64.0, "{kind:?}");
        }
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

fn walking_rabbit() -> Mob {
    let mut rabbit = mob(MobKind::Rabbit, 5);
    rabbit.plan = Plan::Walk {
        target: Vec2::new(60.0, 0.5),
        ticks: 0,
        checkpoint: Vec2::new(0.5, 0.5),
    };
    rabbit
}

#[test]
fn rabbit_moves_only_while_hopping() {
    let mut rabbit = walking_rabbit();
    let around = Surroundings {
        collision: &flat,
        fluid: &dry,
        player_eye: Vec3::new(0.5, 65.6, 0.5),
    };
    let (mut hops, mut grounded_ticks, mut rest_ticks) = (0, 0, 0);
    let mut was_grounded = true;
    for _ in 0..600 {
        let before = rabbit.body.feet;
        let resting = rabbit.hop_rest > 0 && rabbit.body.on_ground;
        rabbit.tick(&around);
        let moved = (rabbit.body.feet - before).truncate().length();
        if resting {
            rest_ticks += 1;
            assert!(moved < 1e-4, "slid while resting");
        }
        if rabbit.body.on_ground {
            grounded_ticks += 1;
        }
        if !was_grounded && rabbit.body.on_ground {
            hops += 1;
        }
        was_grounded = rabbit.body.on_ground;
    }
    assert!(hops >= 5, "{hops}");
    assert!(rest_ticks > 0 && grounded_ticks > 0);
    assert!(rabbit.body.feet.x > 3.0);
}

#[test]
fn rabbit_hops_low_and_other_mobs_do_not_hop() {
    let around = Surroundings {
        collision: &flat,
        fluid: &dry,
        player_eye: Vec3::new(0.5, 65.6, 0.5),
    };
    let mut rabbit = walking_rabbit();
    let mut peak = 64.0_f32;
    for _ in 0..300 {
        rabbit.tick(&around);
        peak = peak.max(rabbit.body.feet.y);
    }
    assert!(peak > 64.2 && peak < 64.6, "{peak}");
    assert_eq!(rabbit.pose().hop > 0.0, !rabbit.body.on_ground);
    let mut ocelot = mob(MobKind::Ocelot, 5);
    run(&mut ocelot, 600, &flat, &dry);
    assert_eq!(ocelot.pose().hop, 0.0);
}

#[test]
fn standing_rabbit_stays_on_the_ground() {
    let mut rabbit = mob(MobKind::Rabbit, 5);
    run(&mut rabbit, 100, &flat, &dry);
    if matches!(rabbit.plan, Plan::Idle) {
        assert_eq!(rabbit.body.feet.y, 64.0);
        assert_eq!(rabbit.pose().hop, 0.0);
    }
}

const POND_RADIUS: i32 = 3;
const SEABED: i32 = 49;
const SURFACE: i32 = 62;

fn in_pond(pos: IVec3) -> bool {
    pos.x.abs() <= POND_RADIUS && pos.z.abs() <= POND_RADIUS
}

fn pond_collision(pos: IVec3) -> Collision {
    let open_water = in_pond(pos) && pos.y > SEABED;
    if pos.y <= SURFACE && !open_water {
        Collision::FULL
    } else {
        Collision::NONE
    }
}

fn pond_fluid(pos: IVec3) -> Option<(FluidKind, f32)> {
    (in_pond(pos) && pos.y > SEABED && pos.y <= SURFACE).then_some((FluidKind::Water, 1.0))
}

fn open_sea_collision(pos: IVec3) -> Collision {
    if pos.y <= SEABED {
        Collision::FULL
    } else {
        Collision::NONE
    }
}

fn open_sea_fluid(pos: IVec3) -> Option<(FluidKind, f32)> {
    (pos.y > SEABED && pos.y <= SURFACE).then_some((FluidKind::Water, 1.0))
}

fn squid_in_open_sea(seed: u32) -> Mob {
    Mob::new(MobKind::Squid, Vec3::new(0.5, 55.0, 0.5), 0.0, seed)
}

fn horizontal_speed(mob: &Mob) -> f32 {
    mob.body.velocity.x.hypot(mob.body.velocity.z)
}

#[test]
fn squid_swims_in_bursts_that_fade_out() {
    let mut squid = squid_in_open_sea(5);
    let around = Surroundings {
        collision: &open_sea_collision,
        fluid: &open_sea_fluid,
        player_eye: Vec3::ZERO,
    };
    let (mut fastest, mut slowest) = (0.0_f32, f32::MAX);
    let (mut swing, mut closed) = (0.0_f32, false);
    for tick in 0..3000 {
        squid.tick(&around);
        if tick > 600 {
            fastest = fastest.max(horizontal_speed(&squid));
            slowest = slowest.min(horizontal_speed(&squid));
        }
        swing = swing.max(squid.pose().tentacle);
        closed |= squid.pose().tentacle == 0.0;
        assert!(squid.body.feet.y >= (SEABED + 1) as f32 - 1e-3);
        assert!(squid.body.feet.y + 0.8 <= (SURFACE + 1) as f32 + 1e-3);
    }
    assert!(fastest > 2.5 && fastest < 4.5, "{fastest}");
    assert!(slowest < 0.5, "{slowest}");
    assert!(
        swing > 0.5 && swing <= std::f32::consts::FRAC_PI_4 + 1e-3,
        "{swing}"
    );
    assert!(closed);
    assert!(squid.body.feet.distance(Vec3::new(0.5, 55.0, 0.5)) > 3.0);
}

#[test]
fn squid_tilts_with_its_motion_and_turns_towards_it() {
    let mut squid = squid_in_open_sea(9);
    let around = Surroundings {
        collision: &open_sea_collision,
        fluid: &open_sea_fluid,
        player_eye: Vec3::ZERO,
    };
    let mut tilted = false;
    for _ in 0..1500 {
        squid.tick(&around);
        let flat = Vec2::new(squid.body.velocity.x, squid.body.velocity.z);
        tilted |= squid.pose().pitch > 0.8;
        if flat.length() > 3.5 {
            let heading = flat.y.atan2(flat.x);
            assert!(angle_diff(heading, squid.yaw).abs() < 1.6);
        }
    }
    assert!(tilted);
}

#[test]
fn squid_stays_in_its_pond_and_below_the_surface() {
    for seed in 1..=3 {
        let mut squid = Mob::new(MobKind::Squid, Vec3::new(0.5, 55.0, 0.5), 0.0, seed);
        let around = Surroundings {
            collision: &pond_collision,
            fluid: &pond_fluid,
            player_eye: Vec3::ZERO,
        };
        for _ in 0..6000 {
            squid.tick(&around);
            let feet = squid.body.feet;
            assert!(feet.x.abs() <= POND_RADIUS as f32 + 1.0, "{feet}");
            assert!(feet.z.abs() <= POND_RADIUS as f32 + 1.0, "{feet}");
            assert!(feet.y + 0.8 <= (SURFACE + 1) as f32 + 0.15, "{feet}");
        }
    }
}

#[test]
fn squid_on_land_falls_and_lies_still() {
    let mut squid = mob(MobKind::Squid, 4);
    squid.body.feet.y = 70.0;
    run(&mut squid, 600, &flat, &dry);
    assert_eq!(squid.body.feet.y, 64.0);
    assert_eq!(squid.pose().tentacle, 0.0);
    let spot = squid.body.feet;
    run(&mut squid, 600, &flat, &dry);
    assert_eq!(squid.body.feet, spot);
    assert!(squid.pose().pitch > 1.0);
}

fn parrot_flight(
    seed: u32,
    ticks: u32,
    collision: &impl Fn(IVec3) -> Collision,
) -> (f32, f32, f32) {
    let mut parrot = mob(MobKind::Parrot, seed);
    let around = Surroundings {
        collision,
        fluid: &dry,
        player_eye: Vec3::new(2.0, 65.6, 0.5),
    };
    let (mut highest, mut farthest, mut landed_after_flight) = (0.0_f32, 0.0_f32, 0.0_f32);
    let mut airborne = false;
    for _ in 0..ticks {
        parrot.tick(&around);
        let feet = parrot.body.feet;
        highest = highest.max(feet.y - 64.0);
        farthest = farthest.max(feet.x.hypot(feet.z));
        assert!(feet.y >= 64.0 - 1e-3, "{feet}");
        let (min, max) = parrot.body.aabb();
        let (lo, hi) = (
            (min + 1e-3).floor().as_ivec3(),
            (max - 1e-3).floor().as_ivec3(),
        );
        for x in lo.x..=hi.x {
            for y in lo.y..=hi.y {
                for z in lo.z..=hi.z {
                    assert_eq!(collision(IVec3::new(x, y, z)).height, 0.0, "{feet}");
                }
            }
        }
        if !parrot.body.on_ground {
            airborne = true;
        } else if airborne {
            landed_after_flight += 1.0;
            airborne = false;
        }
    }
    (highest, farthest, landed_after_flight)
}

#[test]
fn parrot_takes_off_flies_far_and_lands_again() {
    let (highest, farthest, landings) = parrot_flight(3, 12000, &flat);
    assert!(highest > 1.5, "{highest}");
    assert!(farthest > 8.0, "{farthest}");
    assert!(landings >= 2.0, "{landings}");
}

#[test]
fn parrot_never_flies_through_a_wall() {
    let walled = |pos: IVec3| {
        if pos.y <= FLOOR || pos.x >= 6 && pos.y <= FLOOR + 40 {
            Collision::FULL
        } else {
            Collision::NONE
        }
    };
    for seed in 1..=4 {
        let (_, farthest, _) = parrot_flight(seed, 8000, &walled);
        assert!(farthest < 80.0);
    }
}

#[test]
fn parrot_flaps_in_the_air_and_folds_its_wings_on_the_ground() {
    let mut parrot = mob(MobKind::Parrot, 6);
    parrot.body.feet.y = 80.0;
    let around = Surroundings {
        collision: &flat,
        fluid: &dry,
        player_eye: Vec3::ZERO,
    };
    for _ in 0..60 {
        parrot.tick(&around);
    }
    assert!(parrot.pose().flap_amount > 0.9);
    assert!(-parrot.body.velocity.y <= 2.3 + 1e-3);
    for _ in 0..1200 {
        parrot.tick(&around);
        if parrot.body.on_ground {
            break;
        }
    }
    assert!(parrot.body.on_ground);
    for _ in 0..30 {
        parrot.tick(&around);
        if !parrot.body.on_ground {
            return;
        }
    }
    assert!(parrot.pose().flap_amount < 0.2);
}

#[test]
fn flight_targets_are_dry_and_within_reach() {
    let around = Surroundings {
        collision: &flat,
        fluid: &dry,
        player_eye: Vec3::ZERO,
    };
    let mut parrot = mob(MobKind::Parrot, 11);
    let mut picked = 0;
    for _ in 0..200 {
        if let Some(target) = parrot.pick_flight(&around) {
            picked += 1;
            let reach = Vec2::new(target.x - 0.5, target.z - 0.5).length();
            assert!((4.9..14.1).contains(&reach), "{reach}");
            assert!(target.y >= 64.0 && target.y < 64.0 + 4.1, "{target}");
        }
    }
    assert!(picked > 150);
    let sea = Surroundings {
        collision: &flat,
        fluid: &|pos: IVec3| (pos.y > FLOOR).then_some((FluidKind::Water, 1.0)),
        player_eye: Vec3::ZERO,
    };
    assert!((0..50).all(|_| parrot.pick_flight(&sea).is_none()));
    let void = Surroundings {
        collision: &|_: IVec3| Collision::NONE,
        fluid: &dry,
        player_eye: Vec3::ZERO,
    };
    assert!((0..50).all(|_| parrot.pick_flight(&void).is_none()));
}

fn built(blocks: &'static [(IVec3, Block, u8)]) -> impl Fn(IVec3) -> Collision {
    move |pos| {
        if pos.y <= FLOOR {
            return Collision::FULL;
        }
        let Some(&(_, block, data)) = blocks.iter().find(|(at, ..)| *at == pos) else {
            return Collision::NONE;
        };
        let around = |offset: IVec3| {
            blocks
                .iter()
                .find(|(at, ..)| *at == pos + offset)
                .map_or((Block::Air, 0), |&(_, block, data)| (block, data))
        };
        Collision::of_boxes(block.boxes(data, around), false)
    }
}

fn walk_over(blocks: &'static [(IVec3, Block, u8)], target: Vec2, ticks: u32) -> Mob {
    let collision = built(blocks);
    let mut mob = pig(1);
    mob.plan = Plan::Walk {
        target,
        ticks: 0,
        checkpoint: Vec2::new(0.5, 0.5),
    };
    let around = Surroundings {
        collision: &collision,
        fluid: &dry,
        player_eye: Vec3::new(0.5, 65.6, 0.5),
    };
    for _ in 0..ticks {
        mob.tick(&around);
    }
    mob
}

#[test]
fn a_mob_walks_over_a_row_of_slabs() {
    static ROW: [(IVec3, Block, u8); 3] = [
        (IVec3::new(2, 64, 0), Block::OakSlab, 0),
        (IVec3::new(3, 64, 0), Block::OakSlab, 0),
        (IVec3::new(4, 64, 0), Block::OakSlab, 0),
    ];
    let mob = walk_over(&ROW, Vec2::new(6.0, 0.5), 400);
    assert!(mob.body.feet.x > 4.9, "{}", mob.body.feet.x);
}

#[test]
fn a_mob_climbs_a_stair_up_to_its_high_side() {
    static STAIR: [(IVec3, Block, u8); 2] = [
        (IVec3::new(2, 64, 0), Block::OakStairs, 1),
        (IVec3::new(3, 64, 0), Block::OakStairs, 1),
    ];
    let mob = walk_over(&STAIR, Vec2::new(3.5, 0.5), 100);
    assert!(mob.body.feet.x > 2.7, "{}", mob.body.feet.x);
    assert_eq!(mob.body.feet.y, 65.0);
}
