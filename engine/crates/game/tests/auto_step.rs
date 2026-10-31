use game::Game;
use glam::IVec3;
use world::block::Block;
use world::column::Column;
use world::pos::ColumnPos;

const FLOOR: i32 = 10;
const EYE: f32 = (FLOOR + 1) as f32 + 1.62;

fn game_with_ledge(yaw: f32) -> Game {
    let mut game = Game::new(1, Some(&[0.5, EYE, 0.5, yaw, 0.0, 0.0])).unwrap();
    for z in -1..=1 {
        for x in -1..=1 {
            let mut column = Column::new();
            for lz in 0..16 {
                for lx in 0..16 {
                    column.set(IVec3::new(lx, FLOOR, lz), Block::Stone);
                    if x == 0 && lx >= 1 {
                        column.set(IVec3::new(lx, FLOOR + 1, lz), Block::Stone);
                    }
                }
            }
            let key = ColumnPos::new(x, z);
            game.world.insert(key, column);
            game.light.add_column(&game.world, key);
        }
    }
    game
}

#[test]
fn camera_never_dips_below_start_height_while_stepping_up() {
    let mut game = game_with_ledge(0.0);
    let mut time = 0.0;
    for _ in 0..60 {
        game.update(time);
        time += 1.0 / 240.0;
    }
    game.set_key("KeyW", true);
    let mut lowest = f32::MAX;
    let mut highest = f32::MIN;
    for _ in 0..480 {
        game.update(time);
        time += 1.0 / 240.0;
        let y = game.camera().position.y;
        lowest = lowest.min(y);
        highest = highest.max(y);
    }
    assert!(game.player().feet.y > (FLOOR + 1) as f32 + 0.99);
    assert!(lowest >= EYE - 1e-3, "lowest = {lowest}");
    assert!(highest <= EYE + 1.0 + 1e-3, "highest = {highest}");
    assert!((game.camera().position.y - (EYE + 1.0)).abs() < 1e-3);
}

fn climb(frame_times: &[f32], yaw: f32) -> Vec<(f32, f32)> {
    let mut game = game_with_ledge(yaw);
    let mut time = 0.0;
    for _ in 0..60 {
        game.update(time);
        time += 1.0 / 60.0;
    }
    game.set_key("KeyW", true);
    let mut previous = game.camera().position.y;
    let mut frames = Vec::new();
    let mut i = 0;
    while time < 1.0 + 1.5 {
        let dt = frame_times[i % frame_times.len()];
        i += 1;
        time += dt;
        game.update(time);
        let y = game.camera().position.y;
        frames.push((y - previous, dt));
        previous = y;
    }
    frames
}

#[test]
fn camera_rises_smoothly_for_any_frame_pacing() {
    let rate = game::physics::STEP_HEIGHT / game::physics::STEP_SMOOTH_TIME;
    for pacing in [&[1.0 / 144.0][..], &[1.0 / 60.0], &[0.004, 0.013]] {
        for yaw in [0.0, 0.5, -0.5] {
            let frames = climb(pacing, yaw);
            let total: f32 = frames.iter().map(|f| f.0).sum();
            assert!(
                (total - 1.0).abs() < 1e-3,
                "{pacing:?} {yaw}: total = {total}"
            );
            for (rise, dt) in frames {
                assert!(rise >= -1e-4, "{pacing:?} {yaw}: dropped by {rise}");
                let limit = rate * dt + 0.02;
                assert!(rise <= limit, "{pacing:?} {yaw}: jumped {rise} in {dt}s");
            }
        }
    }
}
