use super::*;

fn gradient(size: u32) -> Colormap {
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
    let map = gradient(256);
    assert_eq!(map.at(1.0, 0.0), [0.0, 1.0, 7.0 / 255.0]);
    assert_eq!(map.at(1.0, 1.0), [0.0, 0.0, 7.0 / 255.0]);
    assert_eq!(map.at(0.0, 1.0), [1.0, 1.0, 7.0 / 255.0]);
    let [r, g, _] = map.at(0.5, 0.5);
    assert!(
        (r - 128.0 / 255.0).abs() < 0.01 && (g - 0.75).abs() < 0.01,
        "{r} {g}"
    );
}

#[test]
fn colormap_clamps_climate_and_scales_to_any_size() {
    let big = gradient(256);
    let small = gradient(16);
    assert_eq!(big.at(2.0, 0.0), big.at(1.0, 0.0));
    assert_eq!(big.at(-0.5, 0.4), big.at(0.0, 0.4));
    let [r, g, _] = small.at(0.0, 1.0);
    assert_eq!([r, g], [1.0, 1.0]);
    let (a, b) = (big.at(0.3, 0.6), small.at(0.3, 0.6));
    assert!((a[0] - b[0]).abs() < 0.07 && (a[1] - b[1]).abs() < 0.07);
}

#[test]
fn constants_are_vanilla_biome_colors() {
    let palette = BiomePalette::constants();
    assert_eq!(hex(palette.grass[0]), 0x91BD59);
    assert_eq!(hex(palette.grass[1]), 0xBFB755);
    assert_eq!(hex(palette.grass[2]), 0x79C05A);
    assert_eq!(hex(palette.grass[3]), 0x80B497);
    assert_eq!(hex(palette.foliage[0]), 0x77AB2F);
    assert_eq!(hex(palette.foliage[1]), 0xAEA42A);
    assert_eq!(hex(palette.foliage[2]), 0x59AE30);
    assert_eq!(hex(palette.foliage[3]), 0x60A17B);
}

#[test]
fn missing_colormap_falls_back_to_constants_per_kind() {
    let foliage = gradient(256);
    let palette = BiomePalette::from_colormaps(None, Some(&foliage));
    assert_eq!(palette.grass, BiomePalette::constants().grass);
    assert_eq!(palette.foliage[1], foliage.at(2.0, 0.0));
    assert_ne!(palette.foliage, BiomePalette::constants().foliage);
    assert_eq!(
        BiomePalette::from_colormaps(None, None),
        BiomePalette::constants()
    );
}

#[test]
fn blend_is_a_weighted_mean() {
    let colors = BiomePalette::constants().grass;
    assert_eq!(blend(&colors, [255, 0, 0, 0]), colors[0]);
    let half = blend(&colors, [128, 128, 0, 0]);
    for c in 0..3 {
        let expected = (colors[0][c] + colors[1][c]) / 2.0;
        assert!((half[c] - expected).abs() < 0.01);
    }
    assert_eq!(blend(&colors, [0; 4]), colors[0]);
    assert_eq!(
        blend(&colors, [10, 0, 0, 0]),
        colors[0],
        "weights are normalized"
    );
}

#[test]
fn weights_bytes_quantize_to_unit_range() {
    let w = BiomeWeights {
        plains: 0.5,
        desert: 0.25,
        forest: 0.25,
        taiga: 0.0,
        tundra: 0.0,
    };
    assert_eq!(weights_bytes(w), [128, 64, 64, 0]);
}

#[test]
fn weights_bytes_merge_taiga_and_tundra_into_cold() {
    let w = BiomeWeights {
        plains: 0.0,
        desert: 0.0,
        forest: 0.0,
        taiga: 0.25,
        tundra: 0.5,
    };
    assert_eq!(weights_bytes(w), [0, 0, 0, 191]);
}

#[test]
fn palette_bytes_are_eight_vec4() {
    assert_eq!(BiomePalette::constants().to_bytes().len(), 8 * 16);
}
