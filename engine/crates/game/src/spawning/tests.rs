use super::*;
use world::mob::{horse_colour, horse_markings};

fn kinds(biome: Biome) -> Vec<(MobKind, u32, u32, u32)> {
    animals(biome)
        .iter()
        .map(|e| (e.kind, e.weight, e.min, e.max))
        .collect()
}

const FARM: [(MobKind, u32, u32, u32); 4] = [
    (MobKind::Sheep, 12, 4, 4),
    (MobKind::Pig, 10, 4, 4),
    (MobKind::Chicken, 10, 4, 4),
    (MobKind::Cow, 8, 4, 4),
];

fn farm_and(wolf_weight: u32) -> Vec<(MobKind, u32, u32, u32)> {
    let mut list = FARM.to_vec();
    list.push((MobKind::Wolf, wolf_weight, 4, 4));
    list
}

#[test]
fn plain_biomes_keep_the_farm_animals() {
    for biome in [
        Biome::BirchForest,
        Biome::RoofedForest,
        Biome::FlowerForest,
        Biome::Jungle,
    ] {
        assert_eq!(kinds(biome), FARM, "{biome:?}");
    }
}

#[test]
fn mushroom_islands_only_have_mooshrooms() {
    for biome in [Biome::MushroomIsland, Biome::MushroomIslandShore] {
        assert_eq!(kinds(biome), [(MobKind::Mooshroom, 8, 4, 8)]);
    }
}

#[test]
fn snow_biomes_only_have_polar_bears() {
    for biome in [Biome::IcePlains, Biome::IceMountains, Biome::IceSpikes] {
        assert_eq!(kinds(biome), [(MobKind::PolarBear, 1, 1, 2)]);
    }
}

#[test]
fn normal_forests_add_wolves() {
    for biome in [Biome::Forest, Biome::ForestHills] {
        assert_eq!(kinds(biome), farm_and(5), "{biome:?}");
    }
}

#[test]
fn every_taiga_adds_wolves() {
    for biome in [
        Biome::Taiga,
        Biome::TaigaHills,
        Biome::TaigaM,
        Biome::ColdTaiga,
        Biome::ColdTaigaHills,
        Biome::ColdTaigaM,
        Biome::MegaTaiga,
        Biome::MegaTaigaHills,
        Biome::MegaSpruceTaiga,
        Biome::MegaSpruceTaigaHills,
    ] {
        assert_eq!(kinds(biome), farm_and(8), "{biome:?}");
    }
}

#[test]
fn barren_biomes_have_no_animals() {
    for biome in [
        Biome::Ocean,
        Biome::River,
        Biome::Desert,
        Biome::Mesa,
        Biome::Beach,
        Biome::ColdBeach,
        Biome::StoneBeach,
    ] {
        assert!(animals(biome).is_empty(), "{biome:?}");
    }
}

#[test]
fn group_size_stays_within_the_entry_range() {
    let mut rng = Rng::new(7);
    let entry = animals(Biome::MushroomIsland)[0];
    let sizes: Vec<u32> = (0..200).map(|_| group_size(&mut rng, entry)).collect();
    assert!(sizes.iter().all(|s| (4..=8).contains(s)));
    assert!(sizes.contains(&4) && sizes.contains(&8));
    assert!((0..20).all(|_| group_size(&mut rng, FARM_ANIMALS[0]) == 4));
}

#[test]
fn pick_follows_the_weights() {
    let mut rng = Rng::new(3);
    let list = animals(Biome::Taiga);
    let picks: Vec<MobKind> = (0..2000)
        .filter_map(|_| pick(&mut rng, list))
        .map(|e| e.kind)
        .collect();
    let share = |kind| picks.iter().filter(|&&k| k == kind).count();
    assert!(share(MobKind::Wolf) > 200 && share(MobKind::Wolf) < 450);
    assert!(share(MobKind::Sheep) > share(MobKind::Cow));
    assert!(pick(&mut rng, &[]).is_none());
}

fn with(extra: &[(MobKind, u32, u32, u32)]) -> Vec<(MobKind, u32, u32, u32)> {
    let mut list = FARM.to_vec();
    list.extend_from_slice(extra);
    list
}

#[test]
fn plains_add_horses_and_donkeys() {
    for biome in [Biome::Plains, Biome::SunflowerPlains] {
        assert_eq!(
            kinds(biome),
            with(&[(MobKind::Horse, 5, 2, 6), (MobKind::Donkey, 1, 1, 3)]),
            "{biome:?}"
        );
    }
}

#[test]
fn savannas_add_horses_and_donkeys() {
    let savanna = with(&[(MobKind::Horse, 1, 2, 6), (MobKind::Donkey, 1, 1, 1)]);
    for biome in [Biome::Savanna, Biome::SavannaM, Biome::SavannaPlateauM] {
        assert_eq!(kinds(biome), savanna, "{biome:?}");
    }
}

#[test]
fn savanna_plateau_also_has_llamas() {
    let plateau = with(&[
        (MobKind::Horse, 1, 2, 6),
        (MobKind::Donkey, 1, 1, 1),
        (MobKind::Llama, 8, 4, 4),
    ]);
    assert_eq!(kinds(Biome::SavannaPlateau), plateau);
}

#[test]
fn extreme_hills_have_llamas() {
    for biome in [
        Biome::ExtremeHills,
        Biome::ExtremeHillsEdge,
        Biome::ExtremeHillsPlus,
        Biome::ExtremeHillsM,
        Biome::ExtremeHillsPlusM,
    ] {
        assert_eq!(
            kinds(biome),
            with(&[(MobKind::Llama, 5, 4, 6)]),
            "{biome:?}"
        );
    }
}

#[test]
fn group_colour_is_shared_and_markings_are_personal() {
    let mut rng = Rng::new(11);
    for _ in 0..50 {
        let colour = group_colour(&mut rng, MobKind::Horse);
        assert!(colour < HORSE_COLOURS);
        let group: Vec<u8> = (0..6)
            .map(|_| variant_in_group(&mut rng, MobKind::Horse, colour))
            .collect();
        assert!(group.iter().all(|&v| horse_colour(v) == colour));
        assert!(group.iter().all(|&v| horse_markings(v) < HORSE_MARKINGS));
    }
    let markings: std::collections::HashSet<u8> = (0..200)
        .map(|_| horse_markings(variant_in_group(&mut rng, MobKind::Horse, 2)))
        .collect();
    assert_eq!(markings.len(), usize::from(HORSE_MARKINGS));
}

#[test]
fn llama_colour_is_the_variant_and_others_have_none() {
    let mut rng = Rng::new(5);
    let colours: std::collections::HashSet<u8> = (0..200)
        .map(|_| random_variant(&mut rng, MobKind::Llama))
        .collect();
    assert_eq!(colours, (0..LLAMA_COLOURS).collect());
    for kind in [MobKind::Donkey, MobKind::Pig, MobKind::Wolf] {
        assert_eq!(random_variant(&mut rng, kind), 0);
    }
    let horses: std::collections::HashSet<u8> = (0..500)
        .map(|_| horse_colour(random_variant(&mut rng, MobKind::Horse)))
        .collect();
    assert_eq!(horses.len(), usize::from(HORSE_COLOURS));
}
