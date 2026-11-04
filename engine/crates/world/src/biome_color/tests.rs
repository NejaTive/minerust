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
    let flat = palette.column_bytes(&vec![Biome::Plains; side * side], 0, 0);
    assert_eq!(flat.len(), 3 * 16 * 16 * 4);
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
    let mixed = palette.column_bytes(&biomes, 0, 0);
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

fn rgb8(c: [f32; 3]) -> [u8; 3] {
    c.map(|v| (v * 255.0).round() as u8)
}

#[test]
fn water_is_white_except_in_swamps() {
    let palette = BiomePalette::constants();
    assert_eq!(rgb8(palette.water(Biome::Swampland)), [0xE0, 0xFF, 0xAE]);
    assert_eq!(rgb8(palette.water(Biome::SwamplandM)), [0xE0, 0xFF, 0xAE]);
    for &biome in Biome::ALL.iter().filter(|b| !b.is_swamp()) {
        assert_eq!(palette.water(biome), [1.0; 3], "{biome:?}");
    }
    assert_eq!(palette.plains(BiomeTint::Water), [1.0; 3]);
}

#[test]
fn water_layer_blends_swamp_into_neighbours() {
    let palette = BiomePalette::constants();
    let side = 16 + 2 * BLEND_RADIUS as usize;
    let mut biomes = vec![Biome::Plains; side * side];
    for z in 0..side {
        for x in side / 2..side {
            biomes[z * side + x] = Biome::Swampland;
        }
    }
    let bytes = palette.column_bytes(&biomes, 0, 0);
    let water = &bytes[2 * 16 * 16 * 4..];
    assert_eq!(water[..3], [255; 3]);
    assert_eq!(water[15 * 4..15 * 4 + 3], [0xE0, 0xFF, 0xAE]);
    let mid = water[(side / 2 - BLEND_RADIUS as usize - 1) * 4];
    assert!(mid > 0xE0 && mid < 255);
}

#[test]
fn swamp_grass_takes_both_values_with_dark_share_near_a_third() {
    let (mut dark, mut total) = (0, 0);
    for z in (-1500..1500).step_by(3) {
        for x in (-1500..1500).step_by(3) {
            let color = swamp_grass(x, z);
            assert!(color == [0x4C, 0x76, 0x3C] || color == [0x6A, 0x70, 0x39]);
            dark += i32::from(color == [0x4C, 0x76, 0x3C]);
            total += 1;
        }
    }
    let share = f64::from(dark) / f64::from(total);
    assert!((0.28..0.40).contains(&share), "dark share {share}");
}

#[test]
fn swamp_grass_patches_span_tens_of_blocks() {
    let run = (0..2000)
        .map(|x| swamp_grass(x, 17))
        .collect::<Vec<_>>()
        .chunk_by(|a, b| a == b)
        .map(<[_]>::len)
        .collect::<Vec<_>>();
    let mean = run.iter().sum::<usize>() as f64 / run.len() as f64;
    assert!((10.0..100.0).contains(&mean), "mean run {mean}");
}

#[test]
fn swamp_grass_is_continuous_across_column_borders() {
    let palette = BiomePalette::constants();
    let side = 16 + 2 * BLEND_RADIUS as usize;
    let swamp = vec![Biome::Swampland; side * side];
    let left = palette.column_bytes(&swamp, -16, 40);
    let right = palette.column_bytes(&swamp, 0, 40);
    let wide: Vec<[u8; 3]> = (-16i32..16)
        .map(|x| {
            let bytes = if x < 0 { &left } else { &right };
            let i = (x.rem_euclid(16) * 4) as usize;
            [bytes[i], bytes[i + 1], bytes[i + 2]]
        })
        .collect();
    for (k, x) in (-16..16).enumerate() {
        let around: Vec<[u8; 3]> = (x - BLEND_RADIUS..=x + BLEND_RADIUS)
            .flat_map(|wx| {
                (40 - BLEND_RADIUS..=40 + BLEND_RADIUS).map(move |wz| swamp_grass(wx, wz))
            })
            .collect();
        let n = around.len() as u32;
        let avg: [u8; 3] = std::array::from_fn(|c| {
            ((around.iter().map(|p| u32::from(p[c])).sum::<u32>() + n / 2) / n) as u8
        });
        assert_eq!(wide[k], avg, "x = {x}");
    }
}
