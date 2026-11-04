use game::Game;
use glam::{IVec3, Vec3};
use world::block::{Block, DOOR_OPEN, FACING_NEG_Z, FACING_POS_X, GATE_OPEN, RAIL_EAST_WEST};
use world::column::Column;
use world::pos::ColumnPos;

const FLOOR: i32 = 10;
const Y: i32 = FLOOR + 1;

fn game_with_floor() -> Game {
    let mut game = Game::new(1, None).unwrap();
    for z in -1..=1 {
        for x in -1..=1 {
            let mut column = Column::new();
            for lz in 0..16 {
                for lx in 0..16 {
                    column.set(IVec3::new(lx, FLOOR, lz), Block::Stone);
                }
            }
            let key = ColumnPos::new(x, z);
            game.world.insert(key, column);
            game.light.add_column(&game.world, key);
        }
    }
    game
}

fn aim(game: &mut Game, from: Vec3, to: Vec3) {
    let delta = to - from;
    let yaw = delta.z.atan2(delta.x);
    let pitch = delta.y.atan2(delta.x.hypot(delta.z));
    game.set_camera_state(&[from.x, from.y, from.z, yaw, pitch, 1.0])
        .unwrap();
}

fn right_click(game: &mut Game, time: &mut f32) {
    *time += 0.3;
    game.update(*time);
    game.mouse_button(2, true);
    game.interact(*time);
    game.mouse_button(2, false);
}

fn at(x: i32, z: i32) -> IVec3 {
    IVec3::new(x, Y, z)
}

fn centre(pos: IVec3, height: f32) -> Vec3 {
    Vec3::new(
        pos.x as f32 + 0.5,
        pos.y as f32 + height,
        pos.z as f32 + 0.5,
    )
}

#[test]
fn right_click_opens_and_closes_a_gate_away_from_the_player() {
    let mut game = game_with_floor();
    let gate = at(6, 6);
    game.world
        .set_block_with_data(gate, Block::OakFenceGate, FACING_NEG_Z);
    game.select_block("stone");
    let mut time = 0.0;
    let post = centre(gate, 0.7) + Vec3::new(-0.44, 0.0, 0.0);
    aim(&mut game, post + Vec3::new(0.0, 0.7, 3.0), post);
    right_click(&mut game, &mut time);
    assert_eq!(game.world.block(gate), Block::OakFenceGate);
    assert_eq!(game.world.block_data(gate), GATE_OPEN);
    assert_eq!(
        game.world.block(gate + IVec3::NEG_Z),
        Block::Air,
        "no block placed"
    );
    right_click(&mut game, &mut time);
    assert_eq!(game.world.block_data(gate), 0);
}

#[test]
fn an_open_gate_has_no_collision_and_a_closed_one_is_one_and_a_half_high() {
    let mut game = game_with_floor();
    let gate = at(6, 6);
    game.world
        .set_block_with_data(gate, Block::OakFenceGate, FACING_NEG_Z | GATE_OPEN);
    assert!(game.world.collision_boxes_at(gate).is_empty());
    game.world
        .set_block_with_data(gate, Block::OakFenceGate, FACING_NEG_Z);
    assert_eq!(game.world.collision_boxes_at(gate).top(), 1.5);
}

#[test]
fn a_door_is_placed_whole_opens_by_hand_and_breaks_whole() {
    let mut game = game_with_floor();
    let floor = IVec3::new(6, FLOOR, 6);
    game.select_block("oak_door");
    let mut time = 0.0;
    aim(
        &mut game,
        centre(floor, 4.0) + Vec3::new(-3.0, 0.0, 0.0),
        centre(floor, 1.0),
    );
    right_click(&mut game, &mut time);
    let door = floor + IVec3::Y;
    assert_eq!(game.world.block(door), Block::OakDoor);
    assert_eq!(game.world.block(door + IVec3::Y), Block::OakDoorUpper);
    assert_eq!(game.world.block_data(door) & 3, FACING_POS_X);

    aim(
        &mut game,
        centre(door, 1.2) + Vec3::new(-2.0, 0.0, 0.0),
        centre(door, 0.5),
    );
    right_click(&mut game, &mut time);
    assert_eq!(game.world.block_data(door), FACING_POS_X | DOOR_OPEN);

    game.world.set_block(door, Block::Air);
    let _ = game.world.set_block_supported(door + IVec3::Y, Block::Air);
    assert_eq!(game.world.block(door), Block::Air);
}

#[test]
fn an_iron_door_stays_shut_to_the_hand() {
    let mut game = game_with_floor();
    let door = at(6, 6);
    game.world.place_door(door, Block::IronDoor, FACING_POS_X);
    game.select_block("stone");
    let mut time = 0.0;
    aim(
        &mut game,
        centre(door, 1.2) + Vec3::new(-2.0, 0.0, 0.0),
        centre(door, 0.5),
    );
    right_click(&mut game, &mut time);
    assert_eq!(game.world.block_data(door) & DOOR_OPEN, 0);
}

#[test]
fn rails_pick_their_shape_from_the_neighbours_and_rebuild_them() {
    let mut game = game_with_floor();
    game.select_block("rail");
    let mut time = 0.0;
    for x in [5, 6] {
        let floor = IVec3::new(x, FLOOR, 6);
        aim(
            &mut game,
            centre(floor, 4.0) + Vec3::new(0.0, 0.0, -0.5),
            centre(floor, 1.0),
        );
        right_click(&mut game, &mut time);
    }
    assert_eq!(game.world.block(at(5, 6)), Block::Rail);
    assert_eq!(game.world.block_data(at(6, 6)), RAIL_EAST_WEST);
    assert_eq!(game.world.block_data(at(5, 6)), RAIL_EAST_WEST);
}

#[test]
fn a_fence_is_selected_one_block_high() {
    let mut game = game_with_floor();
    let fence = at(6, 6);
    game.world.set_block(fence, Block::OakFence);
    aim(
        &mut game,
        Vec3::new(2.0, Y as f32 + 1.4, 6.5),
        Vec3::new(6.5, Y as f32 + 1.4, 6.5),
    );
    game.update(0.0);
    game.interact(0.1);
    assert!(
        game.target_block().is_none(),
        "ray above the post top passes over it"
    );
    aim(
        &mut game,
        Vec3::new(2.0, Y as f32 + 0.5, 6.5),
        Vec3::new(6.5, Y as f32 + 0.5, 6.5),
    );
    game.interact(0.2);
    let (pos, (min, max)) = game.target_block().expect("hit");
    assert_eq!(pos, fence);
    assert_eq!(max.y - min.y, 1.0);
}

#[test]
fn a_ladder_goes_onto_a_wall_only() {
    let mut game = game_with_floor();
    let wall = at(8, 8);
    for y in 0..3 {
        game.world.set_block(wall + IVec3::Y * y, Block::Stone);
    }
    game.select_block("ladder");
    let mut time = 0.0;
    aim(
        &mut game,
        centre(wall, 1.3) + Vec3::new(-3.0, 0.0, 0.0),
        centre(wall, 0.5),
    );
    right_click(&mut game, &mut time);
    let ladder = wall + IVec3::NEG_X;
    assert_eq!(game.world.block(ladder), Block::Ladder);
    assert_eq!(game.world.block_data(ladder), world::block::FACING_NEG_X);

    aim(
        &mut game,
        centre(at(2, 2), 4.0),
        centre(IVec3::new(2, FLOOR, 2), 1.0),
    );
    right_click(&mut game, &mut time);
    assert_eq!(game.world.block(at(2, 2)), Block::Air, "not on a floor");

    game.world.set_block_supported(wall, Block::Air);
    assert_eq!(
        game.world.block(ladder),
        Block::Air,
        "falls without its wall"
    );
}
