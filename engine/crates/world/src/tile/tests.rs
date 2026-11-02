use super::*;
use crate::textures::TextureSet;

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

#[test]
fn every_tile_has_a_png_and_no_extra_files() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/textures");
    let mut files: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    files.sort();
    let mut expected: Vec<String> = Tile::ALL
        .iter()
        .map(|t| format!("{}.png", t.file_name()))
        .collect();
    expected.sort();
    assert_eq!(files, expected);
    let _ = TextureSet::standard();
}
