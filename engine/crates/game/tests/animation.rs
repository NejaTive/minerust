use game::Game;

fn run(game: &mut Game, from: f32, seconds: f32) -> f32 {
    let mut time = from;
    while time < from + seconds {
        time += 0.02;
        game.update(time);
    }
    time
}

#[test]
fn anim_time_grows_with_ticks_and_stops_on_pause() {
    let mut game = Game::new(1, None).unwrap();
    assert_eq!(game.anim_time(), 0.0);
    let time = run(&mut game, 0.0, 2.0);
    let running = game.anim_time();
    assert!((1.0..=2.0).contains(&running), "got {running}");

    game.set_paused(true);
    let time = run(&mut game, time, 2.0);
    assert_eq!(game.anim_time(), running);

    game.set_paused(false);
    run(&mut game, time, 1.0);
    assert!(game.anim_time() > running + 0.4);
}
