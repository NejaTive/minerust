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

#[test]
fn each_egg_summons_its_own_mob_with_its_own_colors() {
    let entities = builtin_entities();
    let kinds: Vec<MobKind> = Item::ALL.iter().map(|item| item.def().spawns).collect();
    assert_eq!(kinds, MobKind::ALL);
    let icons: Vec<Vec<u8>> = Item::ALL
        .iter()
        .map(|item| item.icon_pixels(&entities))
        .collect();
    for (i, a) in icons.iter().enumerate() {
        assert!(icons[i + 1..].iter().all(|b| a != b));
    }
}

#[test]
fn parrot_and_squid_eggs_use_vanilla_colors() {
    assert_eq!(Item::ParrotSpawnEgg.def().egg_primary, [0x0D, 0xA7, 0x0B]);
    assert_eq!(Item::ParrotSpawnEgg.def().egg_secondary, [0xFF, 0x00, 0x00]);
    assert_eq!(Item::SquidSpawnEgg.def().egg_primary, [0x22, 0x3B, 0x4D]);
    assert_eq!(Item::SquidSpawnEgg.def().egg_secondary, [0x70, 0x88, 0x99]);
    assert_eq!(Item::ParrotSpawnEgg.def().label, "яйцо призыва попугая");
    assert_eq!(Item::SquidSpawnEgg.def().label, "яйцо призыва кальмара");
}
