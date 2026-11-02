use super::*;

#[test]
fn tile_count_and_slots_follow_tile_order() {
    assert_eq!(TILE_COUNT, 28);
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
