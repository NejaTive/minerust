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

#[test]
fn section_without_data_allocates_no_array() {
    let mut section = Section::new();
    section.set(IVec3::new(1, 2, 3), Block::Log);
    section.set_with_data(IVec3::new(4, 5, 6), Block::Torch, 0);
    assert!(section.data().is_none());
    assert_eq!(section.get_data(IVec3::new(1, 2, 3)), 0);
    // Клон тоже без массива: сгенерированные колонки дублируются при выгрузке.
    assert!(section.clone().data().is_none());
}

#[test]
fn data_is_written_and_reset_with_the_block() {
    let mut section = Section::new();
    let pos = IVec3::new(7, 7, 7);
    section.set_with_data(pos, Block::Log, 2);
    assert_eq!(section.get_data(pos), 2);
    assert_eq!(section.get_data(IVec3::new(6, 7, 7)), 0);
    section.set(pos, Block::Stone);
    assert_eq!(section.get_data(pos), 0);
}

#[test]
fn zero_data_array_is_dropped() {
    let mut section = Section::new();
    section.set_data(Some(Box::new([0; SECTION_VOLUME])));
    assert!(section.data().is_none());
}
