use std::collections::HashSet;
use std::f32::consts::FRAC_PI_2;
use std::f32::consts::FRAC_PI_4;

use super::*;

const FLOOR: i32 = 63;
const GROUND: f32 = (FLOOR + 1) as f32;

fn world(blocks: &[IVec3]) -> impl Fn(IVec3) -> bool + use<> {
    let blocks: HashSet<IVec3> = blocks.iter().copied().collect();
    move |pos| pos.y <= FLOOR || blocks.contains(&pos)
}

fn wall_x(x: i32) -> Vec<IVec3> {
    (GROUND as i32..80)
        .flat_map(|y| (-5..5).map(move |z| IVec3::new(x, y, z)))
        .collect()
}

fn wall_z(z: i32) -> Vec<IVec3> {
    (GROUND as i32..80)
        .flat_map(|y| (-5..5).map(move |x| IVec3::new(x, y, z)))
        .collect()
}

fn standing() -> Player {
    let mut player = Player::new(Vec3::new(0.5, GROUND, 0.5), Mode::Walking);
    player.on_ground = true;
    player
}

const FORWARD: Input = Input {
    forward: 1.0,
    strafe: 0.0,
    jump: false,
    descend: false,
};

fn run(player: &mut Player, input: Input, yaw: f32, ticks: u32, solid: &impl Fn(IVec3) -> bool) {
    for _ in 0..ticks {
        player.step(&input, yaw, solid, dry);
    }
}

#[test]
fn stands_on_flat_floor() {
    let solid = world(&[]);
    let mut player = standing();
    run(&mut player, Input::default(), 0.0, 120, &solid);
    assert_eq!(player.feet.y, GROUND);
    assert!(player.on_ground);
}

#[test]
fn fall_lands_exactly_on_surface() {
    let solid = world(&[]);
    let mut player = Player::new(Vec3::new(0.5, GROUND + 50.0, 0.5), Mode::Walking);
    let mut ticks = 0;
    while !player.on_ground {
        player.step(&Input::default(), 0.0, &solid, dry);
        ticks += 1;
        assert!(ticks < 1000, "не приземлился");
        assert!(player.feet.y >= GROUND);
    }
    assert_eq!(player.feet.y, GROUND);
    assert_eq!(player.velocity.y, 0.0);
}

#[test]
fn terminal_speed_does_not_pass_thin_floor() {
    let solid = |pos: IVec3| pos.y == 10;
    let mut player = Player::new(Vec3::new(0.5, 200.0, 0.5), Mode::Walking);
    player.velocity.y = -TERMINAL_SPEED;
    run(&mut player, Input::default(), 0.0, 600, &solid);
    assert_eq!(player.feet.y, 11.0);
    assert!(player.on_ground);
}

#[test]
fn fall_speed_is_limited() {
    let solid = |_: IVec3| false;
    let mut player = Player::new(Vec3::new(0.5, 10_000.0, 0.5), Mode::Walking);
    run(&mut player, Input::default(), 0.0, 600, &solid);
    assert_eq!(player.velocity.y, -TERMINAL_SPEED);
}

#[test]
fn jump_height_between_one_and_two_blocks() {
    let solid = world(&[]);
    let mut player = standing();
    let jump = Input {
        jump: true,
        ..Input::default()
    };
    player.step(&jump, 0.0, &solid, dry);
    let mut top = player.feet.y;
    for _ in 0..120 {
        player.step(&Input::default(), 0.0, &solid, dry);
        top = top.max(player.feet.y);
    }
    let height = top - GROUND;
    assert!(height > 1.0 && height < 2.0, "высота прыжка {height}");
    assert_eq!(player.feet.y, GROUND);
}

#[test]
fn jumps_onto_one_block_but_not_two() {
    let input = Input {
        jump: true,
        ..FORWARD
    };
    let step: Vec<IVec3> = (2..20)
        .flat_map(|x| (-5..5).map(move |z| IVec3::new(x, GROUND as i32, z)))
        .collect();
    let solid = world(&step);
    let mut player = standing();
    run(&mut player, input, 0.0, 120, &solid);
    assert!(
        player.feet.y >= GROUND + 1.0,
        "не запрыгнул: {}",
        player.feet
    );
    assert!(player.feet.x > 2.3);

    let solid = world(&wall_x(2));
    let mut player = standing();
    run(&mut player, input, 0.0, 120, &solid);
    assert!(player.feet.x <= 2.0 - PLAYER_WIDTH / 2.0);
}

#[test]
fn ceiling_stops_jump() {
    let ceiling: Vec<IVec3> = (-5..5)
        .flat_map(|x| (-5..5).map(move |z| IVec3::new(x, GROUND as i32 + 2, z)))
        .collect();
    let solid = world(&ceiling);
    let mut player = standing();
    let jump = Input {
        jump: true,
        ..Input::default()
    };
    player.step(&jump, 0.0, &solid, dry);
    let limit = GROUND + 2.0 - PLAYER_HEIGHT;
    let mut hit = false;
    for _ in 0..60 {
        player.step(&Input::default(), 0.0, &solid, dry);
        assert!(player.feet.y <= limit + 1e-4);
        if (player.feet.y - limit).abs() < 1e-4 && player.velocity.y == 0.0 {
            hit = true;
        }
    }
    assert!(hit, "прыжок не упёрся в потолок");
    assert_eq!(player.feet.y, GROUND);
}

#[test]
fn walls_stop_movement_on_x_and_z() {
    let solid = world(&wall_x(2));
    let mut player = standing();
    run(&mut player, FORWARD, 0.0, 120, &solid);
    assert_eq!(player.feet.x, 2.0 - PLAYER_WIDTH / 2.0);
    assert_eq!(player.velocity.x, 0.0);

    let solid = world(&wall_z(2));
    let mut player = standing();
    run(&mut player, FORWARD, FRAC_PI_2, 120, &solid);
    assert_eq!(player.feet.z, 2.0 - PLAYER_WIDTH / 2.0);
}

#[test]
fn diagonal_slides_along_wall() {
    let solid = world(&wall_x(2));
    let mut player = standing();
    run(&mut player, FORWARD, FRAC_PI_4, 60, &solid);
    assert_eq!(player.feet.x, 2.0 - PLAYER_WIDTH / 2.0);
    let along = WALK_SPEED * FRAC_PI_4.sin();
    assert!(
        (player.feet.z - 0.5 - along).abs() < 0.01,
        "z = {}",
        player.feet.z
    );
    assert_eq!(player.feet.y, GROUND);
}

#[test]
fn leaves_corner_freely() {
    let mut blocks = wall_x(2);
    blocks.extend(wall_z(2));
    let solid = world(&blocks);
    let mut player = standing();
    run(&mut player, FORWARD, FRAC_PI_4, 60, &solid);
    let corner = 2.0 - PLAYER_WIDTH / 2.0;
    assert_eq!((player.feet.x, player.feet.z), (corner, corner));

    let back = Input {
        forward: -1.0,
        ..Input::default()
    };
    run(&mut player, back, FRAC_PI_4, 30, &solid);
    assert!(player.feet.x < corner - 1.0 && player.feet.z < corner - 1.0);
    run(&mut player, FORWARD, FRAC_PI_4, 60, &solid);
    let along = Input {
        strafe: -1.0,
        ..FORWARD
    };
    run(&mut player, along, 0.0, 30, &solid);
    assert_eq!(player.feet.x, corner);
    assert!(player.feet.z < corner - 1.0);
}

#[test]
fn flying_has_no_gravity_and_follows_vertical_input() {
    let solid = world(&[]);
    let start = GROUND + 10.0;
    let mut player = Player::new(Vec3::new(0.5, start, 0.5), Mode::Flying);
    run(&mut player, Input::default(), 0.0, 60, &solid);
    assert_eq!(player.feet.y, start);

    let up = Input {
        jump: true,
        ..Input::default()
    };
    run(&mut player, up, 0.0, 60, &solid);
    assert!((player.feet.y - start - FLY_SPEED).abs() < 0.01);
    assert_eq!(player.mode, Mode::Flying);
}

#[test]
fn flying_collides_and_lands_into_walking() {
    let solid = world(&wall_x(2));
    let mut player = Player::new(Vec3::new(0.5, GROUND + 3.0, 0.5), Mode::Flying);
    run(&mut player, FORWARD, 0.0, 60, &solid);
    assert_eq!(player.feet.x, 2.0 - PLAYER_WIDTH / 2.0);
    assert_eq!(player.mode, Mode::Flying);

    let down = Input {
        descend: true,
        ..Input::default()
    };
    run(&mut player, down, 0.0, 60, &solid);
    assert_eq!(player.feet.y, GROUND);
    assert_eq!(player.mode, Mode::Walking);
}

#[test]
fn flight_toggled_on_ground_lasts_until_descend() {
    let solid = world(&[]);
    let mut player = standing();
    player.toggle_mode();
    run(&mut player, Input::default(), 0.0, 30, &solid);
    assert_eq!(player.mode, Mode::Flying);
    assert_eq!(player.feet.y, GROUND);

    let down = Input {
        descend: true,
        ..Input::default()
    };
    player.step(&down, 0.0, &solid, dry);
    assert_eq!(player.mode, Mode::Walking);
    assert_eq!(player.feet.y, GROUND);
}

#[test]
fn toggle_into_flight_stops_vertical_motion() {
    let solid = world(&[]);
    let mut player = Player::new(Vec3::new(0.5, GROUND + 10.0, 0.5), Mode::Walking);
    run(&mut player, Input::default(), 0.0, 10, &solid);
    player.toggle_mode();
    let y = player.feet.y;
    run(&mut player, Input::default(), 0.0, 30, &solid);
    assert_eq!(player.feet.y, y);
    player.toggle_mode();
    player.step(&Input::default(), 0.0, &solid, dry);
    assert!(player.feet.y < y);
}

fn simulate(frame: f32, seconds: f32) -> (f32, Vec3) {
    let solid = world(&[]);
    let mut player = standing();
    let mut prev = player.feet;
    let mut clock = Clock::default();
    let input = Input {
        jump: true,
        ..FORWARD
    };
    let mut top = player.feet.y;
    let frames = (seconds / frame).round() as u32;
    for _ in 0..frames {
        for _ in 0..clock.advance(frame) {
            prev = player.feet;
            player.step(&input, 0.0, &solid, dry);
            top = top.max(player.feet.y);
        }
    }
    (top, prev.lerp(player.feet, clock.alpha()))
}

#[test]
fn independent_of_frame_rate() {
    let (top60, end60) = simulate(1.0 / 60.0, 2.0);
    for fps in [30.0, 144.0] {
        let (top, end) = simulate(1.0 / fps, 2.0);
        assert_eq!(top, top60, "высота прыжка при {fps} FPS");
        assert!(
            (end.x - end60.x).abs() < 1e-3,
            "путь при {fps} FPS: {} против {}",
            end.x,
            end60.x
        );
    }
    let expected = (2.0 - TICK) * WALK_SPEED;
    assert!(
        (end60.x - 0.5 - expected).abs() < 1e-3,
        "путь {}",
        end60.x - 0.5
    );
}

#[test]
fn clock_counts_ticks_and_keeps_remainder() {
    let mut clock = Clock::default();
    assert_eq!(clock.advance(TICK * 0.5), 0);
    assert!((clock.alpha() - 0.5).abs() < 1e-4);
    assert_eq!(clock.advance(TICK * 3.0), 3);
    assert!((clock.alpha() - 0.5).abs() < 1e-3);
    clock.reset();
    assert_eq!(clock.alpha(), 0.0);
}

fn dry(_: IVec3) -> bool {
    false
}

fn pool(top: i32) -> impl Fn(IVec3) -> bool {
    move |p: IVec3| {
        p.y > FLOOR && p.y <= top && (-10..10).contains(&p.x) && (-10..10).contains(&p.z)
    }
}

const JUMP: Input = Input {
    forward: 0.0,
    strafe: 0.0,
    jump: true,
    descend: false,
};

#[test]
fn sinks_slowly_in_water() {
    let (solid, water) = (world(&[]), pool(75));
    let mut player = Player::new(Vec3::new(0.5, 72.0, 0.5), Mode::Walking);
    for _ in 0..60 {
        player.step(&Input::default(), 0.0, &solid, &water);
        assert!(
            player.velocity.y >= -WATER_SINK_SPEED,
            "{}",
            player.velocity.y
        );
    }
    let sunk = 72.0 - player.feet.y;
    assert!(sunk > 0.5 && sunk <= WATER_SINK_SPEED, "sunk {sunk}");
    run_in(&mut player, Input::default(), 600, &solid, &water);
    assert_eq!(player.feet.y, GROUND);
}

fn run_in(
    player: &mut Player,
    input: Input,
    ticks: u32,
    solid: &impl Fn(IVec3) -> bool,
    water: &impl Fn(IVec3) -> bool,
) {
    for _ in 0..ticks {
        player.step(&input, 0.0, solid, water);
    }
}

#[test]
fn space_swims_up_and_floats_at_surface() {
    let (solid, water) = (world(&[]), pool(70));
    let mut player = standing();
    run_in(&mut player, JUMP, 60, &solid, &water);
    let risen = player.feet.y - GROUND;
    assert!(risen > 2.0 && risen <= SWIM_UP_SPEED, "risen {risen}");
    run_in(&mut player, JUMP, 180, &solid, &water);
    let surface = 70.0 + WATER_SURFACE;
    for _ in 0..600 {
        player.step(&JUMP, 0.0, &solid, &water);
        assert!(
            player.feet.y < surface && player.feet.y > surface - 1.0,
            "{}",
            player.feet.y
        );
        assert!(!player.eyes_in_water(&water));
    }
    run_in(&mut player, Input::default(), 120, &solid, &water);
    assert!(player.feet.y < surface - 1.5);
    assert!(player.eyes_in_water(&water));
}

#[test]
fn swims_slower_than_walking() {
    let (solid, water) = (world(&[]), pool(70));
    let mut player = Player::new(Vec3::new(-8.0, 66.0, 0.5), Mode::Walking);
    run_in(&mut player, FORWARD, 30, &solid, &water);
    assert!(
        (player.velocity.x - SWIM_SPEED).abs() < 1e-4,
        "{}",
        player.velocity.x
    );
}

#[test]
fn fall_into_water_slows_down() {
    let (solid, water) = (world(&[]), pool(70));
    let mut player = Player::new(Vec3::new(0.5, 110.0, 0.5), Mode::Walking);
    while !water(player.feet.floor().as_ivec3()) {
        player.step(&Input::default(), 0.0, &solid, &water);
    }
    assert!(player.velocity.y < -20.0, "{}", player.velocity.y);
    run_in(&mut player, Input::default(), 30, &solid, &water);
    assert!(
        player.velocity.y >= -WATER_SINK_SPEED - 0.5,
        "{}",
        player.velocity.y
    );
    assert!(player.feet.y >= GROUND);
}

#[test]
fn eyes_in_water_by_eye_cell_and_surface() {
    let water = pool(70);
    let at_eye = |y: f32| Player::new(Vec3::new(0.5, y - EYE_HEIGHT, 0.5), Mode::Walking);
    assert!(at_eye(70.5).eyes_in_water(&water));
    assert!(!at_eye(70.9).eyes_in_water(&water));
    assert!(at_eye(69.95).eyes_in_water(&water));
    assert!(!at_eye(72.0).eyes_in_water(&water));
    let outside = Player::new(Vec3::new(20.5, 66.0, 0.5), Mode::Walking);
    assert!(!outside.eyes_in_water(&water));
}

#[test]
fn flying_ignores_water() {
    let (solid, water) = (world(&[]), pool(75));
    let mut player = Player::new(Vec3::new(0.5, 70.0, 0.5), Mode::Flying);
    run_in(&mut player, Input::default(), 60, &solid, &water);
    assert_eq!(player.feet.y, 70.0);
    run_in(&mut player, FORWARD, 60, &solid, &water);
    assert!(
        (player.feet.x - 0.5 - FLY_SPEED).abs() < 1e-3,
        "{}",
        player.feet.x
    );
}

#[test]
fn noclip_passes_through_blocks_and_never_lands() {
    let rock = |_: IVec3| true;
    let mut player = Player::new(Vec3::new(0.5, 70.0, 0.5), Mode::Walking);
    player.toggle_noclip();
    assert_eq!(player.mode, Mode::Noclip);
    let down = Input {
        descend: true,
        ..FORWARD
    };
    run(&mut player, down, 0.0, 60, &rock);
    assert!(
        player.feet.x > 5.0 && player.feet.y < 65.0,
        "{}",
        player.feet
    );
    assert_eq!(player.mode, Mode::Noclip);
    assert!(!player.on_ground);

    player.toggle_noclip();
    assert_eq!(player.mode, Mode::Flying);
    player.toggle_noclip();
    player.toggle_mode();
    assert_eq!(player.mode, Mode::Walking);
}

#[test]
fn jumps_out_onto_shore_one_block_above_water() {
    let shore: Vec<IVec3> = (GROUND as i32..=67)
        .flat_map(|y| (-5..5).flat_map(move |z| (3..40).map(move |x| IVec3::new(x, y, z))))
        .collect();
    let solid = world(&shore);
    let water = |p: IVec3| pool(66)(p) && p.x < 3;
    let mut player = standing();
    let out = Input {
        jump: true,
        ..FORWARD
    };
    for _ in 0..240 {
        if player.feet.x > 3.5 {
            break;
        }
        player.step(&out, 0.0, &solid, water);
    }
    assert!(player.feet.x > 3.5, "still in water at {}", player.feet);
    run_in(&mut player, FORWARD, 60, &solid, &water);
    assert_eq!(player.feet.y, 68.0);
    assert!(player.on_ground);
}

#[test]
fn block_overlap_allows_touching_faces() {
    let player = standing();
    let cell = IVec3::new(0, GROUND as i32, 0);
    assert!(player.overlaps_block(cell));
    assert!(player.overlaps_block(cell + IVec3::Y));
    assert!(!player.overlaps_block(cell + IVec3::Y * 2));
    assert!(!player.overlaps_block(cell - IVec3::Y));
    assert!(!player.overlaps_block(cell + IVec3::X));
    assert!(!player.overlaps_block(cell - IVec3::Z));
}

#[test]
fn sprint_speeds_up_walking_and_flying() {
    let solid = world(&[]);
    let mut player = standing();
    run(&mut player, FORWARD, 0.0, 5, &solid);
    assert!((player.velocity.x - WALK_SPEED).abs() < 1e-4);
    player.start_sprint();
    run(&mut player, FORWARD, 0.0, 5, &solid);
    assert!(player.sprinting);
    assert!((player.velocity.x - WALK_SPEED * SPRINT_FACTOR).abs() < 1e-4);

    let mut flyer = Player::new(Vec3::new(0.5, 80.0, 0.5), Mode::Flying);
    flyer.start_sprint();
    run(&mut flyer, FORWARD, 0.0, 5, &solid);
    assert!((flyer.velocity.x - FLY_SPEED * FLY_SPRINT_FACTOR).abs() < 1e-4);
}

#[test]
fn sprint_ends_when_forward_released() {
    let solid = world(&[]);
    let mut player = standing();
    player.start_sprint();
    run(&mut player, FORWARD, 0.0, 5, &solid);
    run(&mut player, Input::default(), 0.0, 1, &solid);
    assert!(!player.sprinting);
    player.start_sprint();
    let back = Input {
        forward: -1.0,
        ..FORWARD
    };
    run(&mut player, back, 0.0, 1, &solid);
    assert!(!player.sprinting);
}

#[test]
fn sprint_ends_against_wall() {
    let solid = world(&wall_x(3));
    let mut player = standing();
    player.start_sprint();
    run(&mut player, FORWARD, 0.0, 120, &solid);
    assert!(!player.sprinting);
    assert!(player.feet.x < 3.0);
}

#[test]
fn sprint_ends_in_water() {
    let (solid, water) = (world(&[]), pool(70));
    let mut player = Player::new(Vec3::new(-8.0, 66.0, 0.5), Mode::Walking);
    player.start_sprint();
    run_in(&mut player, FORWARD, 1, &solid, &water);
    assert!(!player.sprinting);
}

#[test]
fn fov_scale_follows_sprint_in_set_time() {
    let mut scale = 1.0;
    for _ in 0..9 {
        scale = next_fov_scale(scale, true, TICK);
    }
    assert!((scale - SPRINT_FOV_SCALE).abs() < 1e-4, "{scale}");
    scale = next_fov_scale(scale, true, 1.0);
    assert_eq!(scale, SPRINT_FOV_SCALE);
    for _ in 0..9 {
        scale = next_fov_scale(scale, false, TICK);
    }
    assert!((scale - 1.0).abs() < 1e-4, "{scale}");
}
