use super::*;
use crate::tile::{DEFAULT_TILE_SIZE, PIXEL_GRID, TORCH_PIXELS, TORCH_TOP_ROW};

const WATER_ALPHA: u8 = 180;

#[test]
fn builtin_set_has_every_tile_without_fallback() {
    let set = TextureSet::standard();
    assert_eq!(set.size(), DEFAULT_TILE_SIZE);
    let mut next = 0;
    for tile in Tile::ALL {
        let texture = set.tile(tile);
        assert_eq!(set.first_layer(tile), next, "{tile:?}");
        assert_eq!(
            texture.layers.len(),
            set.frame_bytes() * texture.frames as usize
        );
        next += texture.frames;
    }
    assert_eq!(set.layer_count(), next);
}

#[test]
fn builtin_animations_come_from_mcmeta() {
    let set = TextureSet::standard();
    for tile in [Tile::Water, Tile::Lava] {
        let texture = set.tile(tile);
        assert!(texture.frames > 1, "{tile:?}");
        assert!(texture.fps > 0.0, "{tile:?}");
        assert!(texture.interpolate, "{tile:?}");
    }
    assert_eq!(set.tile(Tile::Water).fps, 10.0);
    for tile in [Tile::Stone, Tile::Torch, Tile::Leaves] {
        assert_eq!(set.tile(tile).frames, 1, "{tile:?}");
    }
}

#[test]
fn animation_table_has_one_entry_per_tile() {
    let set = TextureSet::standard();
    let table = set.animation_table();
    assert_eq!(table.len(), Tile::ALL.len());
    let entry = |tile: Tile| table[tile.layer() as usize];
    assert_eq!(entry(Tile::Stone)[0], [1.0, 0.0, 0.0, 0.0]);
    assert_eq!(entry(Tile::Leaves)[0][3], 1.0);
    assert_eq!(entry(Tile::TallGrass)[0][3], 2.0);
    assert_eq!(entry(Tile::Torch)[0][3], EMISSIVE_FLAG);
    assert_eq!(entry(Tile::Lava)[0][3], EMISSIVE_FLAG);
    assert_eq!(entry(Tile::Glowstone)[0][3], EMISSIVE_FLAG);
    assert_eq!(entry(Tile::Stone)[2], [1.0, 1.0, 1.0, 0.0]);
    for tile in Tile::ALL {
        assert_eq!(entry(tile)[0][2], 0.0, "{tile:?} must not scroll");
        assert_eq!(entry(tile)[1][0], set.first_layer(tile) as f32, "{tile:?}");
    }
}

#[test]
fn tile_pixels_is_the_first_frame() {
    let set = TextureSet::standard();
    for tile in [Tile::Water, Tile::Lava] {
        let frames = &set.tile(tile).layers;
        assert!(frames.len() > set.frame_bytes());
        assert_eq!(set.tile_pixels(tile), frames[..set.frame_bytes()]);
    }
}

#[test]
fn torch_opaque_pixels_match_torch_geometry() {
    let set = TextureSet::standard();
    let px = set.tile_pixels(Tile::Torch);
    let size = set.size();
    let scale = size / PIXEL_GRID;
    for y in 0..size {
        for x in 0..size {
            let alpha = px[((y * size + x) * 4 + 3) as usize];
            if alpha == 0 {
                continue;
            }
            let (gx, gy) = (x / scale, y / scale);
            assert!(
                (TORCH_PIXELS.0..TORCH_PIXELS.1).contains(&gx) && gy >= TORCH_TOP_ROW,
                "opaque ({x}, {y}) is outside the torch geometry"
            );
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
            assert!(
                px.chunks(4).all(|p| p[3] <= WATER_ALPHA && p[3] > 0),
                "{tile:?}"
            );
        } else if tile == Tile::Ice {
            assert!(px.chunks(4).all(|p| p[3] > 0), "{tile:?}");
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
