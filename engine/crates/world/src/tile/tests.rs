use super::*;

const WATER_ALPHA: u8 = 180;

#[test]
fn layer_count_for_current_tiles() {
    assert_eq!(Tile::ALL.len(), 25);
    assert_eq!(LAYER_COUNT, 55);
}

#[test]
fn layers_follow_tile_order_and_frame_counts() {
    let mut next = 0;
    for tile in Tile::ALL {
        assert_eq!(tile.layer(), next, "{tile:?}");
        next += tile.frames();
    }
    assert_eq!(next, LAYER_COUNT);
    assert_eq!(
        Tile::Water.layer() + Tile::Water.frames(),
        Tile::Glowstone.layer()
    );
}

#[test]
fn animation_table_has_one_entry_per_layer() {
    let table = animation_table();
    assert_eq!(table.len(), LAYER_COUNT as usize);
    for tile in Tile::ALL {
        for frame in 0..tile.frames() {
            let entry = table[(tile.layer() + frame) as usize];
            assert_eq!(
                entry,
                [tile.frames() as f32, tile.fps(), tile.scroll(), 0.0]
            );
        }
    }
    assert_eq!(table[Tile::Water.layer() as usize], [16.0, 10.0, 1.0, 0.0]);
    assert_eq!(table[Tile::Lava.layer() as usize], [16.0, 5.0, 0.3, 0.0]);
    assert_eq!(table[Tile::Stone.layer() as usize], [1.0, 0.0, 0.0, 0.0]);
}

#[test]
fn tile_pixels_is_the_first_frame() {
    for tile in [Tile::Water, Tile::Lava] {
        let frames = tile_frames(tile);
        assert_eq!(frames.len(), (16 * TILE_SIZE * TILE_SIZE * 4) as usize);
        assert_eq!(
            tile_pixels(tile),
            frames[..(TILE_SIZE * TILE_SIZE * 4) as usize]
        );
    }
}

#[test]
fn animation_loops_without_a_jump() {
    for tile in [Tile::Water, Tile::Lava] {
        let frames = tile_frames(tile);
        let size = (TILE_SIZE * TILE_SIZE * 4) as usize;
        let diff = |a: usize, b: usize| -> usize {
            frames[a * size..(a + 1) * size]
                .iter()
                .zip(&frames[b * size..(b + 1) * size])
                .map(|(x, y)| usize::from(x.abs_diff(*y)))
                .sum()
        };
        let seam = diff(15, 0);
        let typical = (0..15).map(|i| diff(i, i + 1)).max().unwrap();
        assert!(
            seam <= typical * 2,
            "{tile:?}: seam {seam}, typical {typical}"
        );
    }
}

#[test]
fn torch_png_is_opaque_exactly_in_torch_geometry() {
    let px = tile_pixels(Tile::Torch);
    for y in 0..TILE_SIZE {
        for x in 0..TILE_SIZE {
            let alpha = px[((y * TILE_SIZE + x) * 4 + 3) as usize];
            let inside = (TORCH_PIXELS.0..TORCH_PIXELS.1).contains(&x) && y >= TORCH_TOP_ROW;
            assert_eq!(alpha == 255, inside, "({x}, {y}) alpha {alpha}");
            assert!(alpha == 0 || alpha == 255);
        }
    }
}

#[test]
fn every_tile_has_a_16x16_png_and_no_extra_files() {
    for tile in Tile::ALL {
        assert_eq!(
            tile_pixels(tile).len(),
            (TILE_SIZE * TILE_SIZE * 4) as usize
        );
    }
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/textures");
    let mut files: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    files.sort();
    let mut expected: Vec<String> = Tile::ALL
        .iter()
        .map(|t| format!("{}.png", t.file_name()))
        .collect();
    expected.sort();
    assert_eq!(files, expected);
    assert_eq!(files.len(), Tile::ALL.len());
}

#[test]
fn cutout_tiles_have_holes_and_opaque_pixels() {
    for tile in Tile::ALL {
        let px = tile_pixels(tile);
        let opaque = px.chunks(4).filter(|p| p[3] == 255).count();
        let clear = px.chunks(4).filter(|p| p[3] == 0).count();
        if tile.is_cutout() {
            assert!(
                opaque > 0 && clear > 0,
                "{tile:?}: opaque {opaque}, clear {clear}"
            );
        } else if tile == Tile::Water {
            assert!(px.chunks(4).all(|p| p[3] == WATER_ALPHA), "{tile:?}");
        } else {
            assert!(px.chunks(4).all(|p| p[3] == 255), "{tile:?}");
        }
    }
}
