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
mod tests;
