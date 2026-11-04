use game::Game;
use glam::{IVec3, Vec3};
use world::block::{
    Block, FACING_NEG_X, FACING_NEG_Z, FACING_POS_X, FACING_POS_Z, SLAB_BOTTOM, SLAB_DOUBLE,
    SLAB_TOP, STAIRS_TOP,
};
use world::column::Column;
use world::pos::ColumnPos;

const FLOOR: i32 = 10;
const Y: i32 = FLOOR + 1;
const HEIGHT: f32 = 3.0;
const PITCH: f32 = -1.0;

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

fn click(game: &mut Game, name: &str, camera: [f32; 5], time: &mut f32) {
    assert!(game.select_block(name));
    *time += 0.3;
    game.update(*time);
    let [x, y, z, yaw, pitch] = camera;
    game.set_camera_state(&[x, y, z, yaw, pitch, 1.0]).unwrap();
    game.mouse_button(2, true);
    game.interact(*time);
    game.mouse_button(2, false);
}

fn from_above(target: Vec3, yaw: f32) -> [f32; 5] {
    let back = Vec3::new(yaw.cos(), 0.0, yaw.sin()) * (HEIGHT / -PITCH.tan());
    let camera = target - back + Vec3::Y * HEIGHT;
    [camera.x, camera.y, camera.z, yaw, PITCH]
}

fn aimed_down(x: f32, z: f32) -> [f32; 5] {
    [x, 15.0, z, 0.0, -std::f32::consts::FRAC_PI_2]
}

fn level_at(point: Vec3, from: Vec3) -> [f32; 5] {
    let delta = point - from;
    let yaw = delta.z.atan2(delta.x);
    let pitch = delta.y.atan2(delta.x.hypot(delta.z));
    [from.x, from.y, from.z, yaw, pitch]
}

fn at(x: i32, z: i32) -> IVec3 {
    IVec3::new(x, Y, z)
}

#[test]
fn slab_goes_to_the_half_that_was_clicked_on_a_side() {
    for (height, expected) in [(0.8, SLAB_TOP), (0.2, SLAB_BOTTOM)] {
        let mut game = game_with_floor();
        game.world.set_block(at(6, 5), Block::Stone);
        let camera = level_at(
            Vec3::new(6.0, Y as f32 + height, 5.5),
            Vec3::new(3.5, Y as f32 + 3.0, 5.5),
        );
        click(&mut game, "oak_slab", camera, &mut 0.0);
        assert_eq!(game.world.block(at(5, 5)), Block::OakSlab, "{height}");
        assert_eq!(game.world.block_data(at(5, 5)), expected, "{height}");
    }
}

#[test]
fn slab_on_the_floor_is_a_bottom_slab() {
    let mut game = game_with_floor();
    click(&mut game, "stone_slab", aimed_down(5.5, 5.5), &mut 0.0);
    assert_eq!(game.world.block(at(5, 5)), Block::StoneSlab);
    assert_eq!(game.world.block_data(at(5, 5)), SLAB_BOTTOM);
}

#[test]
fn clicking_the_top_of_a_bottom_slab_makes_it_double() {
    let mut game = game_with_floor();
    let mut time = 0.0;
    click(&mut game, "oak_slab", aimed_down(5.5, 5.5), &mut time);
    click(&mut game, "oak_slab", aimed_down(5.5, 5.5), &mut time);
    assert_eq!(game.world.block(at(5, 5)), Block::OakSlab);
    assert_eq!(game.world.block_data(at(5, 5)), SLAB_DOUBLE);
    assert_eq!(game.world.block(at(5, 5) + IVec3::Y), Block::Air);
    click(&mut game, "oak_slab", aimed_down(5.5, 5.5), &mut time);
    assert_eq!(game.world.block(at(5, 5) + IVec3::Y), Block::OakSlab);
    assert_eq!(game.world.block_data(at(5, 5) + IVec3::Y), SLAB_BOTTOM);
}

#[test]
fn another_material_does_not_merge_into_a_slab() {
    let mut game = game_with_floor();
    let mut time = 0.0;
    click(&mut game, "oak_slab", aimed_down(5.5, 5.5), &mut time);
    click(&mut game, "spruce_slab", aimed_down(5.5, 5.5), &mut time);
    assert_eq!(game.world.block(at(5, 5)), Block::OakSlab);
    assert_eq!(game.world.block_data(at(5, 5)), SLAB_BOTTOM);
    assert_eq!(game.world.block(at(5, 5) + IVec3::Y), Block::SpruceSlab);
}

#[test]
fn clicking_the_underside_of_a_top_slab_makes_it_double() {
    let mut game = game_with_floor();
    let above = at(5, 5) + IVec3::Y;
    game.world
        .set_block_with_data(above, Block::OakSlab, SLAB_TOP);
    let from = Vec3::new(5.5, Y as f32 + 0.5, 5.5);
    let camera = [from.x, from.y, from.z, 0.0, std::f32::consts::FRAC_PI_2];
    click(&mut game, "oak_slab", camera, &mut 0.0);
    assert_eq!(game.world.block_data(above), SLAB_DOUBLE);
}

#[test]
fn stairs_face_the_way_the_player_looks() {
    for (yaw, facing) in [
        (0.0, FACING_POS_X),
        (std::f32::consts::FRAC_PI_2, FACING_POS_Z),
        (std::f32::consts::PI, FACING_NEG_X),
        (-std::f32::consts::FRAC_PI_2, FACING_NEG_Z),
    ] {
        let mut game = game_with_floor();
        let target = Vec3::new(5.5, Y as f32, 5.5);
        click(&mut game, "oak_stairs", from_above(target, yaw), &mut 0.0);
        assert_eq!(game.world.block(at(5, 5)), Block::OakStairs, "{yaw}");
        assert_eq!(game.world.block_data(at(5, 5)), facing, "{yaw}");
    }
}

#[test]
fn stairs_clicked_on_the_upper_half_of_a_wall_are_upside_down() {
    let mut game = game_with_floor();
    game.world.set_block(at(6, 5), Block::Stone);
    let camera = level_at(
        Vec3::new(6.0, Y as f32 + 0.8, 5.5),
        Vec3::new(3.5, Y as f32 + 3.0, 5.5),
    );
    click(&mut game, "cobblestone_stairs", camera, &mut 0.0);
    assert_eq!(game.world.block(at(5, 5)), Block::CobblestoneStairs);
    assert_eq!(game.world.block_data(at(5, 5)), FACING_POS_X | STAIRS_TOP);
}

#[test]
fn wool_and_carpet_keep_the_item_color() {
    let mut game = game_with_floor();
    let mut time = 0.0;
    click(&mut game, "red_wool", aimed_down(5.5, 5.5), &mut time);
    assert_eq!(game.world.block(at(5, 5)), Block::Wool);
    assert_eq!(game.world.block_data(at(5, 5)), 14);
    click(&mut game, "blue_carpet", aimed_down(8.5, 5.5), &mut time);
    assert_eq!(game.world.block(at(8, 5)), Block::Carpet);
    assert_eq!(game.world.block_data(at(8, 5)), 11);
}

#[test]
fn carpet_needs_a_block_under_it() {
    let mut game = game_with_floor();
    assert!(
        !game
            .world
            .can_place_with_data(at(5, 5) + IVec3::Y * 3, Block::Carpet, 0)
    );
    assert!(game.world.can_place_with_data(at(5, 5), Block::Carpet, 0));
    let mut time = 0.0;
    click(&mut game, "white_carpet", aimed_down(5.5, 5.5), &mut time);
    game.world
        .set_block_supported(at(5, 5) - IVec3::Y, Block::Air);
    assert_eq!(game.world.block(at(5, 5)), Block::Air);
}

#[test]
fn the_selection_box_follows_the_slab() {
    let mut game = game_with_floor();
    game.world
        .set_block_with_data(at(5, 5), Block::OakSlab, SLAB_BOTTOM);
    game.set_camera_state(&[5.5, 15.0, 5.5, 0.0, -std::f32::consts::FRAC_PI_2, 1.0])
        .unwrap();
    game.interact(0.0);
    let (pos, (min, max)) = game.target_block().unwrap();
    assert_eq!(pos, at(5, 5));
    assert_eq!((min, max), (Vec3::ZERO, Vec3::new(1.0, 0.5, 1.0)));
}

#[test]
fn a_player_standing_on_a_slab_blocks_the_double() {
    let mut game = game_with_floor();
    let mut time = 0.0;
    click(&mut game, "oak_slab", aimed_down(5.5, 5.5), &mut time);
    let _ = game.set_camera_state(&[5.5, Y as f32 + 0.5 + 1.62, 5.5, 0.0, -1.5, 1.0]);
    time += 0.3;
    game.mouse_button(2, true);
    game.interact(time);
    game.mouse_button(2, false);
    assert_eq!(game.world.block_data(at(5, 5)), SLAB_BOTTOM);
}

#[test]
fn the_selection_box_of_a_stair_is_the_whole_cube() {
    let mut game = game_with_floor();
    game.world
        .set_block_with_data(at(5, 5), Block::OakStairs, 0);
    game.set_camera_state(&[5.5, 15.0, 5.5, 0.0, -std::f32::consts::FRAC_PI_2, 1.0])
        .unwrap();
    game.interact(0.0);
    let (_, bounds) = game.target_block().unwrap();
    assert_eq!(bounds, (Vec3::ZERO, Vec3::ONE));
}
