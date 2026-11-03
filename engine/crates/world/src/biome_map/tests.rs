use std::collections::HashSet;

use super::*;

fn sampled(map: &BiomeMap, spacing: i32, count: i32) -> HashSet<Biome> {
    let mut seen = HashSet::new();
    for gz in 0..count {
        for gx in 0..count {
            let (x, z) = (
                gx * spacing - count * spacing / 2,
                gz * spacing - count * spacing / 2,
            );
            seen.extend(map.blocks(x, z, 16, 16));
        }
    }
    seen
}

#[test]
fn same_seed_same_map() {
    let (a, b) = (BiomeMap::new(42), BiomeMap::new(42));
    for (x, z) in [(0, 0), (-1000, 37), (123_456, -98_765)] {
        assert_eq!(a.blocks(x, z, 16, 16), b.blocks(x, z, 16, 16));
    }
}

#[test]
fn other_seed_other_map() {
    let (a, b) = (BiomeMap::new(42), BiomeMap::new(43));
    assert_ne!(
        a.blocks(-2000, -2000, 64, 64),
        b.blocks(-2000, -2000, 64, 64)
    );
}

#[test]
fn overlapping_requests_agree() {
    let map = BiomeMap::new(7);
    let wide = map.blocks(-40, -23, 80, 80);
    let narrow = map.blocks(-8, 5, 16, 16);
    for z in 0..16usize {
        for x in 0..16usize {
            assert_eq!(narrow[z * 16 + x], wide[(z + 28) * 80 + x + 32]);
        }
    }
    assert_eq!(
        map.blocks(13, -9, 1, 1)[0],
        map.blocks(0, -20, 40, 40)[11 * 40 + 13]
    );
}

#[test]
fn cached_window_matches_direct_request() {
    let map = BiomeMap::new(5);
    let coarse = map.coarse(28 / 4 - 2, -12 / 4 - 2, 9, 9);
    assert_eq!(
        map.blocks_within(28, -12, 16, 16, &coarse),
        map.blocks(28, -12, 16, 16)
    );
}

#[test]
fn a_large_sample_finds_water_rivers_beaches_and_every_climate() {
    let map = BiomeMap::new(1337);
    let seen = sampled(&map, 400, 40);
    for biome in [
        Biome::Ocean,
        Biome::DeepOcean,
        Biome::River,
        Biome::Beach,
        Biome::Plains,
        Biome::Desert,
        Biome::Forest,
        Biome::Taiga,
        Biome::IcePlains,
        Biome::Swampland,
        Biome::ExtremeHills,
    ] {
        assert!(seen.contains(&biome), "no {biome:?} in {seen:?}");
    }
    assert!(seen.len() > 25, "{} biomes: {seen:?}", seen.len());
}

#[test]
fn hills_and_mutations_exist() {
    let map = BiomeMap::new(99);
    let seen = sampled(&map, 400, 50);
    assert!(seen.iter().any(|b| matches!(
        b,
        Biome::ForestHills | Biome::TaigaHills | Biome::DesertHills | Biome::IceMountains
    )));
    assert!(seen.iter().any(|b| b.is_mutation()), "{seen:?}");
}

#[test]
fn hot_and_frozen_land_never_touch() {
    let map = BiomeMap::new(2024);
    for z in (-6000..6000).step_by(700) {
        let row = map.blocks(-6000, z, 12_000, 1);
        for pair in row.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let hot = |b: Biome| b.is_desert() || b == Biome::Savanna;
            let frozen = |b: Biome| {
                matches!(
                    b,
                    Biome::IcePlains | Biome::IceMountains | Biome::IceSpikes | Biome::ColdTaiga
                )
            };
            assert!(
                !(hot(a) && frozen(b) || hot(b) && frozen(a)),
                "{a:?} next to {b:?} at z {z}"
            );
        }
    }
}

#[test]
fn coast_is_a_beach_or_a_stone_or_cold_shore() {
    let map = BiomeMap::new(1337);
    let row = map.blocks(-8000, 0, 16_000, 1);
    for pair in row.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if a == Biome::Ocean && b == Biome::Plains || a == Biome::Plains && b == Biome::Ocean {
            panic!("plains meets ocean without a shore");
        }
    }
}

#[test]
fn mesa_jungle_and_mega_taiga_are_found_across_seeds() {
    let (mut mesa, mut jungle, mut mega, mut total) = (0, 0, 0, 0);
    for seed in [1337, 42, 7, 5, 99, 2024, 11, 12] {
        let map = BiomeMap::new(seed);
        for gz in 0..128 {
            let row = map.blocks(-8192, gz * 128 - 8192, 16_384, 1);
            for biome in row.into_iter().step_by(128) {
                total += 1;
                mesa += i32::from(biome.is_mesa());
                jungle += i32::from(biome.is_jungle());
                mega += i32::from(matches!(
                    biome,
                    Biome::MegaTaiga
                        | Biome::MegaTaigaHills
                        | Biome::MegaSpruceTaiga
                        | Biome::MegaSpruceTaigaHills
                ));
            }
        }
    }
    for (name, count) in [("mesa", mesa), ("jungle", jungle), ("mega taiga", mega)] {
        assert!(count * 300 > total, "{name} {count} of {total}");
    }
}
