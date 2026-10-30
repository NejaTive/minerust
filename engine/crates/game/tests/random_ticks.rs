use game::Game;
use glam::IVec3;
use world::block::Block;
use world::column::Column;
use world::pos::ColumnPos;

const GROUND: i32 = 10;

fn game_with_grass_dots() -> Game {
    let mut game = Game::new(1, None).unwrap();
    for z in -1..=1 {
        for x in -1..=1 {
            let mut column = Column::new();
            for lz in 0..16 {
                for lx in 0..16 {
                    column.set(IVec3::new(lx, GROUND - 1, lz), Block::Stone);
                    let dot = lx % 4 == 0 && lz % 4 == 0;
                    let top = if dot { Block::Grass } else { Block::Dirt };
                    column.set(IVec3::new(lx, GROUND, lz), top);
                }
            }
            let key = ColumnPos::new(x, z);
            game.world.insert(key, column);
            game.light.add_column(&game.world, key);
        }
    }
    game.set_camera_state(&[8.0, 15.0, 8.0, 0.0, 0.0, 1.0])
        .unwrap();
    game
}

fn grass(game: &Game) -> usize {
    (-16..32)
        .flat_map(|z| (-16..32).map(move |x| IVec3::new(x, GROUND, z)))
        .filter(|&pos| game.world.block(pos) == Block::Grass)
        .count()
}

fn run(game: &mut Game, from: f32, seconds: f32) -> f32 {
    let mut time = from;
    while time < from + seconds {
        time += 0.02;
        game.update(time);
    }
    time
}

#[test]
fn grass_spreads_faster_when_time_is_fast() {
    let mut normal = game_with_grass_dots();
    let mut fast = game_with_grass_dots();
    fast.set_time_fast(true);
    let start = grass(&normal);
    run(&mut normal, 0.0, 2.0);
    run(&mut fast, 0.0, 2.0);
    assert!(grass(&fast) > grass(&normal) + 20);
    assert!(grass(&normal) >= start);
}

#[test]
fn pause_stops_random_ticks() {
    let mut game = game_with_grass_dots();
    game.set_time_fast(true);
    let time = run(&mut game, 0.0, 0.5);
    game.set_paused(true);
    let before = grass(&game);
    run(&mut game, time, 2.0);
    assert_eq!(grass(&game), before);
}

#[test]
fn spread_edits_reach_interact() {
    let mut game = game_with_grass_dots();
    game.set_time_fast(true);
    run(&mut game, 0.0, 1.0);
    let edits = game.interact(1.0);
    assert!(!edits.dirty.is_empty());
    assert!(edits.lit.is_empty());
}
