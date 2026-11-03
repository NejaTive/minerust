use super::*;

fn ramp(size: u32) -> Colormap {
    let rgba = (0..size * size)
        .flat_map(|i| {
            [
                (i % size * 255 / (size - 1)) as u8,
                (i / size * 255 / (size - 1)) as u8,
                7,
                255,
            ]
        })
        .collect();
    Colormap::from_image(&DecodedImage {
        width: size,
        height: size,
        rgba,
    })
    .unwrap()
}

fn hex(c: [f32; 3]) -> u32 {
    c.map(|v| (v * 255.0).round() as u32)
        .iter()
        .fold(0, |acc, &v| acc << 8 | v)
}

#[test]
fn colormap_lookup_follows_vanilla_formula() {
    let map = ramp(256);
    assert_eq!(map.at(1.0, 0.0), [0, 255, 7]);
    assert_eq!(map.at(1.0, 1.0), [0, 0, 7]);
    assert_eq!(map.at(0.0, 1.0), [255, 255, 7]);
    let [r, g, _] = map.at(0.5, 0.5);
    assert!(r.abs_diff(128) <= 2 && g.abs_diff(191) <= 2, "{r} {g}");
}

#[test]
fn colormap_clamps_climate_and_scales_to_any_size() {
    let big = ramp(256);
    let small = ramp(16);
    assert_eq!(big.at(2.0, 0.0), big.at(1.0, 0.0));
    assert_eq!(big.at(-0.5, 0.4), big.at(0.0, 0.4));
    let [r, g, _] = small.at(0.0, 1.0);
    assert_eq!([r, g], [255, 255]);
    let (a, b) = (big.at(0.3, 0.6), small.at(0.3, 0.6));
    assert!(a[0].abs_diff(b[0]) < 18 && a[1].abs_diff(b[1]) < 18);
}

#[test]
fn constants_are_close_to_vanilla_biome_colors() {
    let palette = BiomePalette::constants();
    let close = |got: [f32; 3], want: u32| {
        let want = [(want >> 16) & 255, (want >> 8) & 255, want & 255];
        got.iter()
            .zip(want)
            .all(|(g, w)| ((g * 255.0).round() - w as f32).abs() <= 24.0)
    };
    assert!(
        close(palette.grass(Biome::Plains), 0x91BD59),
        "{:x}",
        hex(palette.grass(Biome::Plains))
    );
    assert!(close(palette.grass(Biome::Desert), 0xBFB755));
    assert!(close(palette.grass(Biome::Forest), 0x79C05A));
    assert!(close(palette.grass(Biome::IcePlains), 0x80B497));
    assert!(close(palette.foliage(Biome::Plains), 0x77AB2F));
}

#[test]
fn special_biomes_use_fixed_colors() {
    let palette = BiomePalette::constants();
    assert_eq!(hex(palette.grass(Biome::Swampland)), 0x6A7039);
    assert_eq!(hex(palette.foliage(Biome::SwamplandM)), 0x6A7039);
    assert_eq!(hex(palette.grass(Biome::Mesa)), 0x90814D);
    assert_eq!(hex(palette.foliage(Biome::MesaBryce)), 0x9E814D);
    let roofed = palette.grass(Biome::RoofedForest);
    let forest = palette.grass(Biome::Forest);
    assert!(roofed[1] < forest[1], "dark forest grass is darker");
}

#[test]
fn missing_colormap_falls_back_to_constants_per_kind() {
    let foliage = ramp(256);
    let palette = BiomePalette::from_colormaps(None, Some(&foliage));
    assert_eq!(
        palette.grass(Biome::Plains),
        BiomePalette::constants().grass(Biome::Plains)
    );
    let [r, g, b] = foliage.at(2.0, 0.0);
    assert_eq!(
        palette.foliage(Biome::Desert),
        [r, g, b].map(|c| f32::from(c) / 255.0)
    );
    assert_eq!(
        BiomePalette::from_colormaps(None, None),
        BiomePalette::constants()
    );
}

#[test]
fn column_bytes_are_flat_inside_one_biome_and_blend_at_borders() {
    let palette = BiomePalette::constants();
    let side = 16 + 2 * BLEND_RADIUS as usize;
    let flat = palette.column_bytes(&vec![Biome::Plains; side * side]);
    assert_eq!(flat.len(), 2 * 16 * 16 * 4);
    let plains = palette
        .grass(Biome::Plains)
        .map(|c| (c * 255.0).round() as u8);
    assert_eq!(flat[..4], [plains[0], plains[1], plains[2], 255]);
    assert!(
        flat.as_chunks::<4>().0[..256]
            .iter()
            .all(|p| p[..3] == plains)
    );

    let mut biomes = vec![Biome::Plains; side * side];
    for z in 0..side {
        for x in side / 2..side {
            biomes[z * side + x] = Biome::Desert;
        }
    }
    let mixed = palette.column_bytes(&biomes);
    let at = |x: usize| mixed[x * 4];
    let desert = palette
        .grass(Biome::Desert)
        .map(|c| (c * 255.0).round() as u8);
    assert_eq!(at(0), plains[0]);
    assert_eq!(at(15), desert[0]);
    let border = at(side / 2 - BLEND_RADIUS as usize - 1);
    assert!(
        border > plains[0].min(desert[0]) && border < plains[0].max(desert[0])
            || plains[0] == desert[0]
    );
}
