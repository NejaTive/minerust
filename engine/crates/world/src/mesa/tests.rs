use super::*;

#[test]
fn bands_are_deterministic_and_use_all_stripe_colors() {
    let a = Mesa::new(5);
    assert_eq!(a.bands, Mesa::new(5).bands);
    assert!(a.bands.iter().all(|b| b.is_terracotta()));
    let kinds: std::collections::BTreeSet<u8> = (0..20)
        .flat_map(|seed| Mesa::new(seed).bands)
        .map(|b| b as u8)
        .collect();
    assert_eq!(kinds.len(), 7);
}

#[test]
fn band_repeats_every_sixty_four_blocks() {
    let mesa = Mesa::new(9);
    for y in 0..64 {
        assert_eq!(mesa.band(100, y), mesa.band(100, y + 64));
    }
}

#[test]
fn layers_start_with_red_sand_or_clay_and_end_in_stone() {
    let mesa = Mesa::new(9);
    let mut layers = [Block::Air; LAYER_SPAN];
    mesa.layers(Biome::Mesa, 80, 3, 4, &mut layers);
    assert!(layers[0].is_terracotta() || layers[0] == Block::RedSand);
    assert!(layers[LAYER_SPAN - 1] == Block::Stone || layers[LAYER_SPAN - 1].is_terracotta());
    assert!(layers.iter().all(|&b| b != Block::Air));
    let mut forest = [Block::Air; LAYER_SPAN];
    mesa.layers(Biome::MesaPlateauF, 120, 3, 4, &mut forest);
    assert!(matches!(forest[0], Block::Grass | Block::Dirt));
}
