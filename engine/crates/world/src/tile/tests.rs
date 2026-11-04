use super::*;

#[test]
fn tile_count_and_slots_follow_tile_order() {
    assert_eq!(TILE_COUNT, 153);
    for (i, tile) in Tile::ALL.iter().enumerate() {
        assert_eq!(tile.layer() as usize, i, "{tile:?}");
    }
}

#[test]
fn flags_are_part_of_tile_not_of_texture() {
    assert_eq!(Tile::Leaves.sway(), 1.0);
    assert_eq!(Tile::TallGrass.sway(), 2.0);
    assert!(Tile::Torch.emissive());
    assert!(!Tile::Stone.emissive());
}

#[test]
fn new_leaves_sway_and_all_cutout_tiles_are_marked() {
    for tile in [Tile::JungleLeaves, Tile::AcaciaLeaves, Tile::DarkOakLeaves] {
        assert_eq!(tile.sway(), 1.0);
        assert!(tile.is_cutout());
    }
    assert!(Tile::Vine.is_cutout());
    assert_eq!(Tile::Vine.sway(), 0.0);
    assert!(!Tile::JungleLogSide.is_cutout() && !Tile::MelonTop.is_cutout());
}

#[test]
fn lily_and_mushrooms_are_cutout_and_do_not_sway() {
    for tile in [Tile::Waterlily, Tile::BrownMushroom, Tile::RedMushroom] {
        assert!(tile.is_cutout(), "{tile:?}");
        assert_eq!(tile.sway(), 0.0);
    }
    assert!(!Tile::MyceliumTop.is_cutout() && !Tile::Clay.is_cutout());
}
