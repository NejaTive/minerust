use std::collections::HashSet;

use super::*;

#[test]
fn ids_round_trip_and_are_unique() {
    let ids: HashSet<u8> = Biome::ALL.iter().map(|b| b.id()).collect();
    assert_eq!(ids.len(), Biome::ALL.len());
    for &biome in Biome::ALL {
        assert_eq!(Biome::from_id(i32::from(biome.id())), Some(biome));
    }
    assert_eq!(Biome::from_id(8), None);
    assert_eq!(Biome::from_id(9), None);
}

#[test]
fn every_overworld_biome_of_1_12_is_present() {
    assert_eq!(Biome::ALL.len(), 59);
    assert_eq!(Biome::Ocean.id(), 0);
    assert_eq!(Biome::DeepOcean.id(), 24);
    assert_eq!(Biome::MesaPlateauM.id(), 167);
}

#[test]
fn names_are_unique_snake_case() {
    let names: HashSet<&str> = Biome::ALL.iter().map(|b| b.name()).collect();
    assert_eq!(names.len(), Biome::ALL.len());
    assert!(
        names
            .iter()
            .all(|n| n.chars().all(|c| c.is_ascii_lowercase() || c == '_'))
    );
}

#[test]
fn mutations_are_base_id_plus_128() {
    let mutated: Vec<_> = Biome::ALL
        .iter()
        .filter_map(|b| b.mutation().map(|m| (*b, m)))
        .collect();
    assert_eq!(mutated.len(), 21);
    for (base, mutation) in mutated {
        assert_eq!(
            i32::from(mutation.id()),
            i32::from(base.id()) + 128,
            "{base:?}"
        );
        assert!(mutation.is_mutation() && !base.is_mutation());
    }
}

#[test]
fn water_biomes_lie_below_sea_and_mountains_above() {
    for &b in Biome::ALL {
        if b.is_ocean() || b.is_river() {
            assert!(b.base_height() < 0.0, "{b:?}");
        }
    }
    assert!(Biome::DeepOcean.base_height() < Biome::Ocean.base_height());
    assert!(Biome::ExtremeHills.base_height() > 0.9);
    assert!(Biome::MesaPlateau.base_height() > 1.0);
}

#[test]
fn snowy_biomes_are_the_frozen_ones() {
    for biome in [
        Biome::IcePlains,
        Biome::IceMountains,
        Biome::IceSpikes,
        Biome::ColdTaiga,
        Biome::FrozenOcean,
        Biome::FrozenRiver,
        Biome::ColdBeach,
    ] {
        assert!(biome.is_snowy(), "{biome:?}");
    }
    for biome in [
        Biome::Plains,
        Biome::Taiga,
        Biome::Desert,
        Biome::ExtremeHills,
    ] {
        assert!(!biome.is_snowy(), "{biome:?}");
    }
}

#[test]
fn surface_blocks_use_existing_blocks_only() {
    assert_eq!(Biome::Desert.surface_blocks().top, Block::Sand);
    assert_eq!(Biome::Beach.surface_blocks().filler, Block::Sand);
    assert_eq!(Biome::StoneBeach.surface_blocks().top, Block::Stone);
    assert_eq!(Biome::Plains.surface_blocks().top, Block::Grass);
    assert_eq!(Biome::Mesa.surface_blocks().top, Block::Sand);
}
