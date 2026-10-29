use super::*;

#[test]
fn set_then_get() {
    let mut section = Section::new();
    let pos = IVec3::new(15, 3, 7);
    section.set(pos, Block::Stone);
    assert_eq!(section.get(pos), Block::Stone);
    assert_eq!(section.get(IVec3::new(15, 3, 6)), Block::Air);
}

#[test]
#[should_panic]
fn get_out_of_bounds_panics() {
    Section::new().get(IVec3::new(16, 0, 0));
}
