use std::borrow::Cow;
use std::fmt;

use glam::IVec3;

use crate::block::Block;
use crate::column::{Column, WORLD_HEIGHT};
use crate::section::SECTION_SIZE;

/// Версия формата колонки, которую пишет [`encode_column`]. Её же фронт пишет в
/// метаданные мира. Колонки версий `1..=FORMAT_VERSION` читаются (старые мигрируют),
/// остальные — [`SaveError::UnsupportedVersion`].
///
/// Как добавить версию: поднять `FORMAT_VERSION`, дописать в конец [`MIGRATIONS`]
/// функцию «тело версии N → тело версии N+1» (заголовок она не видит), поменять
/// кодирование и тест на прежние байты оставить как есть.
pub const FORMAT_VERSION: u8 = 1;

/// Шаг миграции: тело колонки (байты после заголовка) одной версии → тело следующей.
type Migration = fn(&[u8]) -> Result<Vec<u8>, SaveError>;

/// `MIGRATIONS[n - 1]` поднимает версию n до n + 1. Пока версия одна — шагов нет.
const MIGRATIONS: &[Migration] = &[];

const _: () = assert!(MIGRATIONS.len() + 1 == FORMAT_VERSION as usize);

const MAGIC: &[u8; 3] = b"MRC";
const COLUMN_VOLUME: usize = (SECTION_SIZE * SECTION_SIZE * WORLD_HEIGHT) as usize;
const RUN_SIZE: usize = 3;

// Серия не длиннее колонки, поэтому длина всегда влезает в u16 и серии не дробятся.
const _: () = assert!(COLUMN_VOLUME <= u16::MAX as usize);

/// Почему байты не читаются как колонка.
#[derive(Debug, PartialEq, Eq)]
pub enum SaveError {
    BadMagic,
    UnsupportedVersion(u8),
    UnknownBlock(u8),
    /// Данные кончились посреди серии или серии не покрыли колонку.
    Truncated,
    /// Серии вышли за колонку или после неё остались байты.
    Overflow,
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveError::BadMagic => write!(f, "not a column: bad magic"),
            SaveError::UnsupportedVersion(v) => write!(f, "unsupported column format {v}"),
            SaveError::UnknownBlock(b) => write!(f, "unknown block {b}"),
            SaveError::Truncated => write!(f, "column data is truncated"),
            SaveError::Overflow => write!(f, "column data is too long"),
        }
    }
}

impl std::error::Error for SaveError {}

/// Колонка → байты: магия `b"MRC"`, версия, затем RLE-пары (блок u8, длина u16 LE)
/// в порядке x, z снаружи, y внутри (вертикальные столбы дают длинные серии).
pub fn encode_column(column: &Column) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(64);
    bytes.extend_from_slice(MAGIC);
    bytes.push(FORMAT_VERSION);
    let mut push_run = |block: Block, len: usize| {
        bytes.push(block as u8);
        bytes.extend_from_slice(&(len as u16).to_le_bytes());
    };
    let mut run = (column.get(position(0)), 0);
    for i in 0..COLUMN_VOLUME {
        let block = column.get(position(i));
        if block != run.0 {
            push_run(run.0, run.1);
            run = (block, 0);
        }
        run.1 += 1;
    }
    push_run(run.0, run.1);
    bytes
}

/// Обратное к [`encode_column`]. Ошибка, если магия или версия не те, блок неизвестен
/// или сумма длин серий не равна объёму колонки.
pub fn decode_column(bytes: &[u8]) -> Result<Column, SaveError> {
    let body = bytes.strip_prefix(MAGIC).ok_or(SaveError::BadMagic)?;
    let (&version, body) = body.split_first().ok_or(SaveError::Truncated)?;
    let body = upgrade(version, body, MIGRATIONS)?;
    let body = &*body;
    let mut column = Column::new();
    let mut index = 0;
    for run in body.chunks(RUN_SIZE) {
        if index == COLUMN_VOLUME {
            return Err(SaveError::Overflow);
        }
        let &[block, lo, hi] = run else {
            return Err(SaveError::Truncated);
        };
        let block = Block::try_from(block).map_err(SaveError::UnknownBlock)?;
        let end = index + usize::from(u16::from_le_bytes([lo, hi]));
        if end > COLUMN_VOLUME {
            return Err(SaveError::Overflow);
        }
        // Новая колонка — сплошной воздух, его не переписываем.
        if block != Block::Air {
            for i in index..end {
                column.set(position(i), block);
            }
        }
        index = end;
    }
    if index < COLUMN_VOLUME {
        return Err(SaveError::Truncated);
    }
    Ok(column)
}

/// Тело колонки версии `version` → тело последней версии (`steps.len() + 1`). Тело
/// текущей версии не копируется.
fn upgrade<'a>(
    version: u8,
    body: &'a [u8],
    steps: &[Migration],
) -> Result<Cow<'a, [u8]>, SaveError> {
    let first = usize::from(version);
    if first == 0 || first > steps.len() + 1 {
        return Err(SaveError::UnsupportedVersion(version));
    }
    let mut body = Cow::Borrowed(body);
    for step in &steps[first - 1..] {
        body = Cow::Owned(step(&body)?);
    }
    Ok(body)
}

/// Номер блока в порядке сериализации → локальные координаты колонки.
fn position(i: usize) -> IVec3 {
    let i = i as i32;
    let size = SECTION_SIZE;
    IVec3::new(
        i / (size * WORLD_HEIGHT),
        i % WORLD_HEIGHT,
        i / WORLD_HEIGHT % size,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pos::ColumnPos;
    use crate::terrain::Terrain;

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
        // y внутри: соседние номера — соседние блоки одного вертикального столба.
        assert_eq!(position(1), IVec3::new(0, 1, 0));
        assert_eq!(position(WORLD_HEIGHT as usize), IVec3::new(0, 0, 1));
    }

    #[test]
    fn empty_and_solid_are_one_run() {
        assert_eq!(round_trip(&Column::new()).len(), 4 + RUN_SIZE);
        assert_eq!(round_trip(&filled(Block::Stone)).len(), 4 + RUN_SIZE);
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
            // Рельеф даёт ~4 серии на столб (камень, земля, трава, воздух): ~3 КБ из 32;
            // пещеры добавляют по паре серий в столбах, где проходят.
            assert!(bytes.len() < 8192, "{key}: {} bytes", bytes.len());
        }
    }

    #[test]
    fn random_blocks_round_trip() {
        // xorshift: случайность без зависимостей и одинаковая при каждом запуске.
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
        // Колонка в байтах, как её писала версия с блоками 0–3: камень, земля, трава,
        // воздух в каждом столбе. Формат не менялся, новые номера просто стали допустимы.
        // Версия записана литералом: тест должен читать именно v1 и после подъёма FORMAT_VERSION.
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
        // Заглушка цепочки: каждая версия дописывает в тело свой номер. Так видно, какие
        // шаги и в каком порядке прошли; первая настоящая миграция заменит эти функции.
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
        // Ошибка шага не теряется.
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
        bytes[4] = 200;
        assert_eq!(
            decode_column(&bytes).err(),
            Some(SaveError::UnknownBlock(200))
        );
    }

    #[test]
    fn rejects_truncated() {
        let bytes = round_trip(&Terrain::new(7).generate(ColumnPos::ZERO));
        // Посреди серии и ровно без последней серии.
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

        // Пустая серия после полной колонки — тоже лишние данные.
        let mut bytes = encode_column(&Column::new());
        bytes.extend_from_slice(&[0, 0, 0]);
        assert_eq!(decode_column(&bytes).err(), Some(SaveError::Overflow));

        // Серия длиннее остатка колонки.
        let mut bytes = encode_column(&Column::new());
        bytes[5..7].copy_from_slice(&(COLUMN_VOLUME as u16 + 1).to_le_bytes());
        assert_eq!(decode_column(&bytes).err(), Some(SaveError::Overflow));
    }
}
