use mesh::templates::*;

#[test]
fn only_fluid_side_templates_are_flagged_for_scrolling() {
    let bytes = table_bytes();
    let flagged: Vec<usize> = (0..TEMPLATE_COUNT)
        .filter(|i| f32::from_le_bytes(bytes[i * 32 + 20..i * 32 + 24].try_into().unwrap()) == 1.0)
        .collect();
    let expected: Vec<usize> = (WATER_SIDE..WATER_BOTTOM)
        .chain(WATER_SIDE_SLOPED..TEMPLATE_COUNT as u8)
        .map(usize::from)
        .collect();
    assert_eq!(flagged, expected);
}
