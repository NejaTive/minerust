use super::*;

#[test]
fn column_rects_tile_the_torus_without_wrapping_inside_a_column() {
    assert_eq!(BIOME_MAP_SIZE % SECTION_SIZE as u32, 0);
    assert_eq!(column_origin(ColumnPos::new(0, 0)), (0, 0));
    assert_eq!(column_origin(ColumnPos::new(1, 2)), (16, 32));
    assert_eq!(
        column_origin(ColumnPos::new(-1, -1)),
        (BIOME_MAP_SIZE - 16, BIOME_MAP_SIZE - 16)
    );
    assert_eq!(column_origin(ColumnPos::new(64, 65)), (0, 16));
}

#[test]
fn map_covers_the_largest_render_distance() {
    let widest_blocks = (2 * 16 + 1) * SECTION_SIZE as u32;
    assert!(BIOME_MAP_SIZE >= widest_blocks);
}
