use super::*;
use crate::pack::builtin_entities;

#[test]
fn names_round_trip() {
    for item in Item::ALL {
        assert_eq!(Item::from_name(item.def().name), Some(item));
    }
    assert_eq!(Item::from_name("nope"), None);
}

#[test]
fn icon_is_tinted_from_both_layers() {
    let entities = builtin_entities();
    let pixels = Item::PigSpawnEgg.icon_pixels(&entities);
    let size = Item::icon_size(&entities);
    assert_eq!(pixels.len(), (size * size * 4) as usize);
    let opaque: Vec<&[u8; 4]> = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[3] == 255)
        .collect();
    assert!(!opaque.is_empty());
    assert!(opaque.iter().all(|p| p[0] >= p[2]));
}
