use glam::IVec3;

use crate::block::Block;
use crate::column::Column;
use crate::pos::ColumnPos;
use crate::world::World;

fn world() -> World {
    let mut world = World::new();
    world.insert(ColumnPos::ZERO, Column::new());
    world
}

#[test]
fn a_flower_goes_into_an_empty_pot_once() {
    let mut world = world();
    let pos = IVec3::new(3, 10, 3);
    world.set_block(pos, Block::FlowerPot);
    assert!(world.plant_in_pot(pos, Block::Poppy));
    assert_eq!(Block::pot_plant(world.block_data(pos)), Some(Block::Poppy));
    assert!(!world.plant_in_pot(pos, Block::Dandelion));
    assert_eq!(Block::pot_plant(world.block_data(pos)), Some(Block::Poppy));
}

#[test]
fn only_listed_plants_go_into_a_pot() {
    let mut world = world();
    let pos = IVec3::new(3, 10, 3);
    world.set_block(pos, Block::FlowerPot);
    assert!(!world.plant_in_pot(pos, Block::Stone));
    assert!(!world.plant_in_pot(pos, Block::TallGrass));
    assert_eq!(world.block_data(pos), 0);
    assert!(!world.plant_in_pot(pos + IVec3::X, Block::Cactus));
}
