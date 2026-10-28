use super::*;

/// Альфа во всех пикселях `water.png`: дно на глубине пары блоков ещё видно.
const WATER_ALPHA: u8 = 180;

#[test]
fn layer_count_for_current_tiles() {
    assert_eq!(Tile::ALL.len(), 14);
    assert_eq!(LAYER_COUNT, 14);
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
    // tile_pixels паникует с именем файла, если размер или формат не подходят.
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
