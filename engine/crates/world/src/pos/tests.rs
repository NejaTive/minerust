use super::*;

#[test]
fn block_to_keys_uses_floor_division() {
    assert_eq!(
        ColumnPos::of_block(IVec3::new(-1, 5, -16)),
        ColumnPos::new(-1, -1)
    );
    assert_eq!(
        ColumnPos::of_block(IVec3::new(-17, 5, 15)),
        ColumnPos::new(-2, 0)
    );
    assert_eq!(
        SectionPos::of_block(IVec3::new(5, 40, -7)),
        SectionPos::new(0, 2, -1)
    );
    assert_eq!(
        SectionPos::of_block(IVec3::new(5, 40, -7)).column(),
        ColumnPos::new(0, -1)
    );
}

#[test]
fn origins_are_world_corners() {
    assert_eq!(ColumnPos::new(-2, 3).origin(), IVec3::new(-32, 0, 48));
    assert_eq!(SectionPos::new(-2, 3, 1).origin(), IVec3::new(-32, 48, 16));
    assert_eq!(ColumnPos::new(1, -1).section(4), SectionPos::new(1, 4, -1));
}
