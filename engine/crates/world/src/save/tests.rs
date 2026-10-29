use super::*;
use crate::pos::ColumnPos;
use crate::terrain::Terrain;

const HEADER: usize = 5;

const BLOCKS: [Block; 13] = [
    Block::Air,
    Block::Grass,
    Block::Dirt,
    Block::Stone,
    Block::Sand,
    Block::Log,
    Block::Leaves,
    Block::Water,
    Block::Glowstone,
    Block::TallGrass,
    Block::Poppy,
    Block::Dandelion,
    Block::Torch,
];

fn assert_same(a: &Column, b: &Column) {
    for i in 0..COLUMN_VOLUME {
        assert_eq!(a.get(position(i)), b.get(position(i)), "{}", position(i));
        assert_eq!(
            a.get_data(position(i)),
            b.get_data(position(i)),
            "data {}",
            position(i)
        );
    }
}

fn round_trip(column: &Column) -> Vec<u8> {
    let bytes = encode_column(column);
    assert_same(&decode_column(&bytes).expect("decodes"), column);
    bytes
}

fn filled(block: Block) -> Column {
    let mut column = Column::new();
    for i in 0..COLUMN_VOLUME {
        column.set(position(i), block);
    }
    column
}

#[test]
fn position_covers_column_once() {
    let mut seen = std::collections::HashSet::new();
    for i in 0..COLUMN_VOLUME {
        let p = position(i);
        assert!(Column::contains(p), "{p}");
        assert!(seen.insert(p.to_array()));
    }
    assert_eq!(position(1), IVec3::new(0, 1, 0));
    assert_eq!(position(WORLD_HEIGHT as usize), IVec3::new(0, 0, 1));
}

#[test]
fn empty_and_solid_are_one_run() {
    assert_eq!(round_trip(&Column::new()).len(), HEADER + RUN_SIZE);
    assert_eq!(round_trip(&filled(Block::Stone)).len(), HEADER + RUN_SIZE);
}

#[test]
fn generated_column_is_small() {
    let terrain = Terrain::new(1337);
    for key in [
        ColumnPos::ZERO,
        ColumnPos::new(-7, 12),
        ColumnPos::new(40, -3),
    ] {
        let bytes = round_trip(&terrain.generate(key));
        assert!(bytes.len() < 8192, "{key}: {} bytes", bytes.len());
    }
}

#[test]
fn random_blocks_round_trip() {
    let mut state = 0x2545_f491_u32;
    let mut column = Column::new();
    for i in 0..COLUMN_VOLUME {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        column.set(position(i), BLOCKS[(state as usize) % BLOCKS.len()]);
    }
    round_trip(&column);
}

#[test]
fn reads_columns_saved_before_new_blocks() {
    let mut bytes = b"MRC".to_vec();
    bytes.push(1);
    let columns = SECTION_SIZE * SECTION_SIZE;
    for _ in 0..columns {
        for (block, len) in [(3u8, 60u16), (2, 3), (1, 1), (0, 64)] {
            bytes.push(block);
            bytes.extend_from_slice(&len.to_le_bytes());
        }
    }
    let column = decode_column(&bytes).expect("old column decodes");
    let at = |y| column.get(IVec3::new(5, y, 9));
    assert_eq!(
        (at(0), at(59), at(60), at(63), at(64)),
        (
            Block::Stone,
            Block::Stone,
            Block::Dirt,
            Block::Grass,
            Block::Air
        )
    );
}

#[test]
fn new_blocks_round_trip() {
    let mut column = Column::new();
    for (i, &block) in BLOCKS.iter().enumerate() {
        column.set(IVec3::new(i as i32, 70, 3), block);
    }
    round_trip(&column);
    let terrain = Terrain::new(1337);
    round_trip(&terrain.generate(ColumnPos::new(3, -4)));
    let lake = terrain.generate(crate::terrain::lake_key(&terrain));
    assert!((0..COLUMN_VOLUME).any(|i| lake.get(position(i)) == Block::Water));
    round_trip(&lake);
}

#[test]
fn rejects_bad_header() {
    let mut bytes = encode_column(&Column::new());
    bytes[0] = b'X';
    assert_eq!(decode_column(&bytes).err(), Some(SaveError::BadMagic));
    assert_eq!(decode_column(b"MR").err(), Some(SaveError::BadMagic));
    assert_eq!(decode_column(b"MRC").err(), Some(SaveError::Truncated));

    let mut bytes = encode_column(&Column::new());
    bytes[3] = FORMAT_VERSION + 1;
    assert_eq!(
        decode_column(&bytes).err(),
        Some(SaveError::UnsupportedVersion(FORMAT_VERSION + 1))
    );
}

#[test]
fn rejects_version_zero() {
    let mut bytes = encode_column(&Column::new());
    bytes[3] = 0;
    assert_eq!(
        decode_column(&bytes).err(),
        Some(SaveError::UnsupportedVersion(0))
    );
}

#[test]
fn upgrade_applies_steps_from_the_saved_version() {
    fn v1_to_v2(body: &[u8]) -> Result<Vec<u8>, SaveError> {
        Ok([body, &[2]].concat())
    }
    fn v2_to_v3(body: &[u8]) -> Result<Vec<u8>, SaveError> {
        Ok([body, &[3]].concat())
    }
    fn broken(_: &[u8]) -> Result<Vec<u8>, SaveError> {
        Err(SaveError::Truncated)
    }
    let steps: [Migration; 2] = [v1_to_v2, v2_to_v3];
    let upgraded = |version| upgrade(version, &[9], &steps).map(Cow::into_owned);
    assert_eq!(upgraded(1), Ok(vec![9, 2, 3]));
    assert_eq!(upgraded(2), Ok(vec![9, 3]));
    assert_eq!(upgraded(3), Ok(vec![9]));
    assert_eq!(upgraded(0), Err(SaveError::UnsupportedVersion(0)));
    assert_eq!(upgraded(4), Err(SaveError::UnsupportedVersion(4)));
    let steps: [Migration; 1] = [broken];
    assert_eq!(upgrade(1, &[], &steps).err(), Some(SaveError::Truncated));
}

#[test]
fn current_version_body_is_not_copied() {
    let body = [1, 2, 3];
    assert!(matches!(
        upgrade(FORMAT_VERSION, &body, MIGRATIONS),
        Ok(Cow::Borrowed(_))
    ));
}

#[test]
fn rejects_unknown_block() {
    let mut bytes = encode_column(&filled(Block::Dirt));
    bytes[HEADER] = 200;
    assert_eq!(
        decode_column(&bytes).err(),
        Some(SaveError::UnknownBlock(200))
    );
}

#[test]
fn rejects_truncated() {
    let bytes = round_trip(&Terrain::new(7).generate(ColumnPos::ZERO));
    assert_eq!(
        decode_column(&bytes[..bytes.len() - 1]).err(),
        Some(SaveError::Truncated)
    );
    assert_eq!(
        decode_column(&bytes[..bytes.len() - RUN_SIZE]).err(),
        Some(SaveError::Truncated)
    );
    assert_eq!(decode_column(&bytes[..4]).err(), Some(SaveError::Truncated));
}

#[test]
fn rejects_extra_data() {
    let mut bytes = encode_column(&Column::new());
    bytes.push(0);
    assert_eq!(decode_column(&bytes).err(), Some(SaveError::Overflow));

    let mut bytes = encode_column(&Column::new());
    bytes.extend_from_slice(&[0, 0, 0]);
    assert_eq!(decode_column(&bytes).err(), Some(SaveError::Overflow));

    let mut bytes = encode_column(&Column::new());
    bytes[HEADER + 1..HEADER + 3].copy_from_slice(&(COLUMN_VOLUME as u16 + 1).to_le_bytes());
    assert_eq!(decode_column(&bytes).err(), Some(SaveError::Overflow));
}

#[test]
fn v1_column_migrates_with_zero_data() {
    let mut bytes = b"MRC".to_vec();
    bytes.push(1);
    bytes.push(0);
    bytes.extend_from_slice(&(COLUMN_VOLUME as u16).to_le_bytes());
    let column = decode_column(&bytes).expect("v1 decodes");
    assert!(column.section(0).data().is_none());
    assert_eq!(encode_column(&column).len(), bytes.len() + 1);
}

#[test]
fn column_without_data_costs_one_flag_byte_over_v1() {
    let terrain = Terrain::new(1337);
    for key in [ColumnPos::ZERO, ColumnPos::new(-7, 12)] {
        let column = terrain.generate(key);
        let v2 = encode_column(&column);
        let migrated = v1_to_v2(&v2[4 + 1..]).expect("migrates");
        assert_eq!(migrated, v2[4..]);
        assert_eq!(v2[4], 0);
    }
}

#[test]
fn block_data_round_trips() {
    let mut column = Column::new();
    column.set_with_data(IVec3::new(2, 70, 3), Block::Log, 1);
    column.set_with_data(IVec3::new(15, 127, 15), Block::Log, 2);
    column.set_with_data(IVec3::new(0, 0, 0), Block::Torch, 4);
    column.set(IVec3::new(5, 70, 5), Block::Log);
    let bytes = round_trip(&column);
    let back = decode_column(&bytes).expect("decodes");
    assert_eq!(back.get_data(IVec3::new(2, 70, 3)), 1);
    assert_eq!(back.get_data(IVec3::new(15, 127, 15)), 2);
    assert_eq!(back.get_data(IVec3::new(0, 0, 0)), 4);
    assert_eq!(back.get_data(IVec3::new(5, 70, 5)), 0);
    assert_eq!(bytes[4], 1 | 1 << 4 | 1 << 7);
    assert_eq!(
        bytes.len(),
        encode_column(&Column::new()).len() - RUN_SIZE + 3 * SECTION_VOLUME + runs_len(&bytes)
    );
}

fn runs_len(bytes: &[u8]) -> usize {
    bytes.len() - HEADER - bytes[4].count_ones() as usize * SECTION_VOLUME
}

#[test]
fn zeroed_data_is_not_saved() {
    let mut column = Column::new();
    column.set_with_data(IVec3::new(1, 1, 1), Block::Log, 2);
    column.set(IVec3::new(1, 1, 1), Block::Air);
    assert_eq!(encode_column(&column), encode_column(&Column::new()));
}

#[test]
fn rejects_truncated_section_data() {
    let mut column = Column::new();
    column.set_with_data(IVec3::new(1, 1, 1), Block::Log, 2);
    let bytes = encode_column(&column);
    assert_eq!(
        decode_column(&bytes[..HEADER + 100]).err(),
        Some(SaveError::Truncated)
    );
    let mut bytes = encode_column(&Column::new());
    bytes[4] = 0xFF;
    assert_eq!(decode_column(&bytes).err(), Some(SaveError::Truncated));
}
