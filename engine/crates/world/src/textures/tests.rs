use super::*;
use crate::tile::{TORCH_PIXELS, TORCH_TOP_ROW};

const WATER_ALPHA: u8 = 180;

#[test]
fn standard_layout_matches_the_old_one() {
    let set = TextureSet::standard();
    assert_eq!(set.size(), 16);
    assert_eq!(set.layer_count(), 58);
    let mut next = 0;
    for tile in Tile::ALL {
        assert_eq!(set.first_layer(tile), next, "{tile:?}");
        assert_eq!(set.tile(tile).frames, tile.frames());
        assert_eq!(
            set.tile(tile).layers.len(),
            set.frame_bytes() * tile.frames() as usize
        );
        next += tile.frames();
    }
    assert_eq!(
        set.first_layer(Tile::Water) + Tile::Water.frames(),
        set.first_layer(Tile::Glowstone)
    );
}

#[test]
fn animation_table_has_one_entry_per_tile() {
    let set = TextureSet::standard();
    let table = set.animation_table();
    assert_eq!(table.len(), Tile::ALL.len());
    let entry = |tile: Tile| table[tile.layer() as usize];
    assert_eq!(entry(Tile::Water)[0], [16.0, 10.0, 1.0, 0.0]);
    assert_eq!(entry(Tile::Lava)[0], [16.0, 5.0, 0.3, EMISSIVE_FLAG]);
    assert_eq!(entry(Tile::Stone)[0], [1.0, 0.0, 0.0, 0.0]);
    assert_eq!(entry(Tile::Leaves)[0], [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(entry(Tile::TallGrass)[0], [1.0, 0.0, 0.0, 2.0]);
    assert_eq!(entry(Tile::Torch)[0], [1.0, 0.0, 0.0, EMISSIVE_FLAG]);
    assert_eq!(entry(Tile::Glowstone)[0][3], EMISSIVE_FLAG);
    for tile in Tile::ALL {
        assert_eq!(entry(tile)[1][0], set.first_layer(tile) as f32, "{tile:?}");
        assert_eq!(entry(tile)[1][1], 0.0);
        assert_eq!(entry(tile)[2], [1.0, 1.0, 1.0, 0.0], "{tile:?}");
    }
}

#[test]
fn tile_pixels_is_the_first_frame() {
    let set = TextureSet::standard();
    for tile in [Tile::Water, Tile::Lava] {
        let frames = &set.tile(tile).layers;
        assert_eq!(frames.len(), 16 * set.frame_bytes());
        assert_eq!(set.tile_pixels(tile), frames[..set.frame_bytes()]);
    }
}

#[test]
fn animation_loops_without_a_jump() {
    let set = TextureSet::standard();
    for tile in [Tile::Water, Tile::Lava] {
        let frames = &set.tile(tile).layers;
        let size = set.frame_bytes();
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
    let px = TextureSet::standard().tile_pixels(Tile::Torch);
    for y in 0..16 {
        for x in 0..16 {
            let alpha = px[((y * 16 + x) * 4 + 3) as usize];
            let inside = (TORCH_PIXELS.0..TORCH_PIXELS.1).contains(&x) && y >= TORCH_TOP_ROW;
            assert_eq!(alpha == 255, inside, "({x}, {y}) alpha {alpha}");
        }
    }
}

#[test]
fn cutout_tiles_have_holes_and_opaque_pixels() {
    let set = TextureSet::standard();
    for tile in Tile::ALL {
        let px = set.tile_pixels(tile);
        let opaque = px.chunks(4).filter(|p| p[3] == 255).count();
        let clear = px.chunks(4).filter(|p| p[3] == 0).count();
        if tile.is_cutout() {
            assert!(opaque > 0 && clear > 0, "{tile:?}: {opaque}/{clear}");
        } else if tile == Tile::Water {
            assert!(px.chunks(4).all(|p| p[3] == WATER_ALPHA), "{tile:?}");
        } else if tile == Tile::Ice {
            assert!(px.chunks(4).all(|p| (100..255).contains(&p[3])), "{tile:?}");
        } else {
            assert!(px.chunks(4).all(|p| p[3] == 255), "{tile:?}");
        }
    }
}

#[test]
fn icon_pixels_apply_the_tint() {
    let mut tiles: Vec<TileTexture> = Tile::ALL
        .iter()
        .map(|_| TileTexture::still([200, 100, 50, 255].repeat(16 * 16)))
        .collect();
    tiles[Tile::Leaves.layer() as usize].tint = [0.5, 1.0, 0.0];
    let set = TextureSet::from_tiles(16, tiles);
    assert_eq!(&set.icon_pixels(Tile::Leaves)[..4], [100, 100, 0, 255]);
    assert_eq!(&set.icon_pixels(Tile::Stone)[..4], [200, 100, 50, 255]);
}

#[test]
fn scale_nearest_repeats_pixels() {
    let src = [1, 0, 0, 255, 2, 0, 0, 255, 3, 0, 0, 255, 4, 0, 0, 255];
    let out = scale_nearest(&src, 2, 4);
    let reds: Vec<u8> = out.chunks(4).map(|p| p[0]).collect();
    assert_eq!(reds, [1, 1, 2, 2, 1, 1, 2, 2, 3, 3, 4, 4, 3, 3, 4, 4]);
    assert_eq!(scale_nearest(&src, 2, 2), src);
}
