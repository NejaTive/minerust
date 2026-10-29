use super::*;

#[test]
fn set_then_get_lands_in_right_section() {
    let mut column = Column::new();
    let pos = IVec3::new(3, 37, 15);
    column.set(pos, Block::Stone);
    assert_eq!(column.get(pos), Block::Stone);
    assert_eq!(column.section(2).get(IVec3::new(3, 5, 15)), Block::Stone);
    assert_eq!(column.get(IVec3::new(3, 36, 15)), Block::Air);
}

#[test]
#[should_panic]
fn get_above_column_panics() {
    Column::new().get(IVec3::new(0, WORLD_HEIGHT, 0));
}

#[test]
#[should_panic]
fn get_below_column_panics() {
    Column::new().get(IVec3::new(0, -1, 0));
}
