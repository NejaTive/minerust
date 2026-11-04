use super::*;

#[test]
fn horse_variant_packs_colour_and_markings() {
    for colour in 0..HORSE_COLOURS {
        for markings in 0..HORSE_MARKINGS {
            let variant = horse_variant(colour, markings);
            assert_eq!(horse_colour(variant), colour);
            assert_eq!(horse_markings(variant), markings);
        }
    }
}

#[test]
fn new_kinds_are_appended_after_the_old_ones() {
    assert_eq!(MobKind::Wolf as u8, 6);
    assert_eq!(MobKind::Horse as u8, 7);
    assert_eq!(MobKind::Donkey as u8, 8);
    assert_eq!(MobKind::Llama as u8, 9);
    assert_eq!(MobKind::try_from(9), Ok(MobKind::Llama));
    assert_eq!(MobKind::Rabbit as u8, 10);
    assert_eq!(MobKind::Ocelot as u8, 11);
    assert_eq!(MobKind::try_from(11), Ok(MobKind::Ocelot));
    assert_eq!(MobKind::try_from(12), Err(12));
}
