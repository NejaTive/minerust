use glam::IVec3;
use world::block::Block;
use world::column::Column;
use world::item::Item;
use world::mob::{MobKind, RABBIT_GOLD, RABBIT_SPLOTCHED, RABBIT_WHITE};
use world::pos::ColumnPos;
use world::terrain::Biome;

use super::*;
use crate::raycast::Hit;
use crate::spawning::{CREATURE_CAP, cap};

const FLOOR: i32 = 10;
const SPAN: i32 = 6;
const NIGHT: f32 = 0.75;
const NOON: f32 = 0.25;
const CYCLES: usize = 600;

fn biome_of(game: &Game, key: ColumnPos) -> Biome {
    let origin = key.origin();
    game.terrain.sample_biome(origin.x + 8, origin.z + 8)
}

fn keys_in(seed: i32, biomes: &[Biome], count: usize) -> Vec<ColumnPos> {
    let game = Game::new(seed, None).unwrap();
    (-120..120)
        .flat_map(|x| (-120..120).map(move |z| ColumnPos::new(x, z)))
        .filter(|&key| biomes.contains(&biome_of(&game, key)))
        .step_by(97)
        .take(count)
        .collect()
}

fn seed_with(biomes: &[Biome]) -> i32 {
    (1..60)
        .find(|&seed| !keys_in(seed, biomes, 1).is_empty())
        .expect("a seed with these biomes nearby")
}

fn load_generated(game: &mut Game, key: ColumnPos) {
    let column = game.terrain.generate(key);
    game.world.insert(key, column);
}

fn farm_game(floor: Block) -> Game {
    let mut game = Game::new(1, None).unwrap();
    for x in -SPAN..=SPAN {
        for z in -SPAN..=SPAN {
            let mut column = Column::new();
            column.populated = true;
            for lx in 0..16 {
                for lz in 0..16 {
                    column.set(IVec3::new(lx, FLOOR, lz), floor);
                }
            }
            let key = ColumnPos::new(x, z);
            game.world.insert(key, column);
            game.light.add_column(&game.world, key);
        }
    }
    game.set_camera_state(&[0.5, 40.0, 0.5, 0.0, 0.0, 1.0])
        .unwrap();
    game.set_time_of_day(NOON);
    game
}

fn cycle_many(game: &mut Game) {
    for _ in 0..CYCLES {
        game.spawn_cycle();
    }
}

fn cell_at(x: i32, z: i32) -> IVec3 {
    IVec3::new(x, FLOOR + 1, z)
}

fn cow_at(game: &mut Game, cell: IVec3) -> Mob {
    game.new_mob(MobKind::Cow, 0, cell, &mut Rng::new(5))
}

#[test]
fn herds_appear_on_grass_in_plains_but_not_in_desert_or_tundra() {
    let mut game = Game::new(21, None).unwrap();
    let mut herds = 0;
    for key in keys_in(21, &[Biome::Plains], 120) {
        load_generated(&mut game, key);
        let records = game.generation_herds(key);
        for record in &records {
            let below = record.pos.floor().as_ivec3() - IVec3::Y;
            assert_eq!(game.world.block(below), Block::Grass);
            assert_eq!(ColumnPos::of_block(record.pos.floor().as_ivec3()), key);
        }
        herds += records.len();
    }
    assert!(herds > 0);
    for key in keys_in(21, &[Biome::Desert, Biome::Ocean, Biome::River], 40) {
        load_generated(&mut game, key);
        assert!(game.generation_herds(key).is_empty());
    }
}

#[test]
fn generation_herds_are_deterministic() {
    let mut game = Game::new(21, None).unwrap();
    let key = keys_in(21, &[Biome::Plains], 1)[0];
    load_generated(&mut game, key);
    assert_eq!(game.generation_herds(key), game.generation_herds(key));
}

#[test]
fn reloading_a_saved_column_does_not_respawn_the_herd() {
    let mut game = Game::new(21, None).unwrap();
    let key = keys_in(21, &[Biome::Plains], 400)
        .into_iter()
        .find(|&key| {
            load_generated(&mut game, key);
            !game.generation_herds(key).is_empty()
        })
        .expect("a column with a herd");
    game.set_camera_state(&[
        key.origin().x as f32,
        150.0,
        key.origin().z as f32,
        0.0,
        0.0,
        1.0,
    ])
    .unwrap();
    game.world.remove(key);
    load_generated(&mut game, key);
    game.adopt_mobs();
    let herd = game.mob_count();
    assert!(herd > 0);
    assert!(game.world.column(key).unwrap().populated);

    let saved = world::save::encode_column(game.world.edited_column(key).unwrap());
    game.world.remove(key);
    game.adopt_mobs();
    assert_eq!(game.mob_count(), 0);
    game.import_column(key, world::save::decode_column(&saved).unwrap());
    let restored = game.world.take_edited(key).unwrap();
    game.world.insert(key, restored);
    game.adopt_mobs();
    assert_eq!(game.mob_count(), herd);
}

#[test]
fn cycle_spawns_animals_on_grass_within_the_cap() {
    let mut game = farm_game(Block::Grass);
    cycle_many(&mut game);
    let limit = cap((2 * SPAN as usize + 1).pow(2));
    assert!(limit > 0 && limit <= CREATURE_CAP);
    assert!(game.mob_count() > 0);
    assert!(game.mob_count() <= limit);
    for mob in game.mobs() {
        assert_eq!(mob.body.feet.y, (FLOOR + 1) as f32);
        assert!(mob.body.feet.distance(game.player().feet) >= MIN_PLAYER_DISTANCE);
    }
}

#[test]
fn nothing_spawns_without_grass_or_in_the_dark() {
    let mut bare = farm_game(Block::Stone);
    cycle_many(&mut bare);
    assert_eq!(bare.mob_count(), 0);

    let mut night = farm_game(Block::Grass);
    night.set_time_of_day(NIGHT);
    cycle_many(&mut night);
    assert_eq!(night.mob_count(), 0);
}

#[test]
fn spawn_spot_needs_distance_and_room() {
    let mut game = farm_game(Block::Grass);
    game.set_camera_state(&[8.5, 12.7, 8.5, 0.0, 0.0, 1.0])
        .unwrap();
    let near = cell_at(8, 8 + 10);
    let far = cell_at(8, 8 + 30);
    let mob = cow_at(&mut game, near);
    assert!(!game.may_spawn_here(&mob, near));
    let mob = cow_at(&mut game, far);
    assert!(game.may_spawn_here(&mob, far));

    game.world
        .set_block(IVec3::new(far.x, far.y + 1, far.z), Block::Stone);
    let mob = cow_at(&mut game, far);
    assert!(!game.may_spawn_here(&mob, far));
}

#[test]
fn generation_puts_gold_rabbits_on_desert_sand() {
    let mut game = Game::new(21, None).unwrap();
    let mut rabbits = 0;
    for key in keys_in(21, &[Biome::Desert], 200) {
        load_generated(&mut game, key);
        for record in game.generation_herds(key) {
            assert_eq!(record.kind, MobKind::Rabbit);
            assert_eq!(record.variant, RABBIT_GOLD);
            let below = record.pos.floor().as_ivec3() - IVec3::Y;
            assert!(game.world.block(below).collision_height() >= 1.0);
            assert!(!game.world.block(below).is_grass());
            rabbits += 1;
        }
    }
    assert!(rabbits > 0);
}

#[test]
fn generated_herds_keep_one_rabbit_colour_per_group() {
    let seed = seed_with(&[Biome::IcePlains]);
    let mut game = Game::new(seed, None).unwrap();
    let mut snowy = 0;
    for key in keys_in(seed, &[Biome::IcePlains], 200) {
        load_generated(&mut game, key);
        for record in game.generation_herds(key) {
            if record.kind == MobKind::Rabbit {
                assert!([RABBIT_WHITE, RABBIT_SPLOTCHED].contains(&record.variant));
                snowy += 1;
            }
        }
    }
    assert!(snowy > 0);
}

#[test]
fn generation_never_spawns_on_liquid_or_in_barren_biomes() {
    let mut game = Game::new(21, None).unwrap();
    for key in keys_in(21, &[Biome::Ocean, Biome::River, Biome::Beach], 60) {
        load_generated(&mut game, key);
        assert!(game.generation_herds(key).is_empty());
    }
}

#[test]
fn ocelots_spawn_on_leaves_above_sea_level_only() {
    let mut game = farm_game(Block::Grass);
    game.set_camera_state(&[8.5, 12.7, 8.5, 0.0, 0.0, 1.0])
        .unwrap();
    let high = IVec3::new(8, OCELOT_MIN_Y, 38);
    let low = IVec3::new(8, OCELOT_MIN_Y - 1, 38);
    for cell in [high, low] {
        game.world.set_block(cell - IVec3::Y, Block::Leaves);
    }
    let ocelot = |game: &Game, cell| game.new_mob(MobKind::Ocelot, 0, cell, &mut Rng::new(5));
    assert!(game.may_spawn_here(&ocelot(&game, high), high));
    assert!(!game.may_spawn_here(&ocelot(&game, low), low));
    let cow = game.new_mob(MobKind::Cow, 0, high, &mut Rng::new(5));
    assert!(!game.may_spawn_here(&cow, high));
}

#[test]
fn egg_rabbit_takes_the_colour_of_the_biome_it_is_summoned_in() {
    for (biome, allowed) in [
        (Biome::Desert, vec![RABBIT_GOLD]),
        (Biome::IcePlains, vec![RABBIT_WHITE, RABBIT_SPLOTCHED]),
    ] {
        let seed = seed_with(&[biome]);
        let mut game = Game::new(seed, None).unwrap();
        let key = keys_in(seed, &[biome], 1)[0];
        load_generated(&mut game, key);
        let origin = key.origin();
        let cell = IVec3::new(origin.x + 8, 200, origin.z + 8);
        for _ in 0..20 {
            game.mobs.clear();
            let hit = Hit {
                block: cell - IVec3::Y,
                normal: IVec3::Y,
                distance: 1.0,
            };
            game.use_item(Item::RabbitSpawnEgg, hit);
            assert_eq!(game.mobs.len(), 1);
            assert!(allowed.contains(&game.mobs[0].variant), "{biome:?}");
        }
    }
}
