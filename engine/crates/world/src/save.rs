use std::borrow::Cow;
use std::fmt;

use glam::IVec3;

use crate::block::Block;
use crate::column::{COLUMN_SECTIONS, Column, WORLD_HEIGHT};
use crate::section::{SECTION_SIZE, SECTION_VOLUME};

/// Версия формата колонки, которую пишет [`encode_column`]. Её же фронт пишет в
/// метаданные мира. Колонки версий `1..=FORMAT_VERSION` читаются (старые мигрируют),
/// остальные — [`SaveError::UnsupportedVersion`].
///
/// Как добавить версию: поднять `FORMAT_VERSION`, дописать в конец [`MIGRATIONS`]
/// функцию «тело версии N → тело версии N+1» (заголовок она не видит), поменять
/// кодирование и тест на прежние байты оставить как есть.
pub const FORMAT_VERSION: u8 = 2;

/// Шаг миграции: тело колонки (байты после заголовка) одной версии → тело следующей.
type Migration = fn(&[u8]) -> Result<Vec<u8>, SaveError>;

/// `MIGRATIONS[n - 1]` поднимает версию n до n + 1.
const MIGRATIONS: &[Migration] = &[v1_to_v2];

/// В v2 перед сериями блоков встал байт флагов секций с данными; в v1 данных не было,
/// то есть флаги нулевые.
fn v1_to_v2(body: &[u8]) -> Result<Vec<u8>, SaveError> {
    let mut out = Vec::with_capacity(body.len() + 1);
    out.push(0);
    out.extend_from_slice(body);
    Ok(out)
}

const _: () = assert!(MIGRATIONS.len() + 1 == FORMAT_VERSION as usize);

const MAGIC: &[u8; 3] = b"MRC";
const COLUMN_VOLUME: usize = (SECTION_SIZE * SECTION_SIZE * WORLD_HEIGHT) as usize;
const RUN_SIZE: usize = 3;

// Флаги секций с данными лежат в одном байте.
const _: () = assert!(COLUMN_SECTIONS <= 8);

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

/// Колонка → байты: магия `b"MRC"`, версия, байт флагов (бит `sy` — у секции `sy` есть
/// ненулевые данные блоков), по 4096 байт данных на каждую секцию с флагом (в порядке
/// секции, индекс `x + 16·z + 256·y`), затем RLE-пары (блок u8, длина u16 LE) в порядке
/// x, z снаружи, y внутри (вертикальные столбы дают длинные серии). Колонка без данных
/// весит как в v1 плюс байт флагов.
pub fn encode_column(column: &Column) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(64);
    bytes.extend_from_slice(MAGIC);
    bytes.push(FORMAT_VERSION);
    let with_data: Vec<_> = (0..COLUMN_SECTIONS)
        .filter_map(|sy| {
            let data = column.section(sy).data()?;
            data.iter().any(|&b| b != 0).then_some((sy, data))
        })
        .collect();
    bytes.push(with_data.iter().fold(0, |flags, &(sy, _)| flags | 1 << sy));
    for (_, data) in with_data {
        bytes.extend_from_slice(data);
    }
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
    let (&flags, mut body) = body.split_first().ok_or(SaveError::Truncated)?;
    let mut column = Column::new();
    let mut section_data = Vec::new();
    for sy in (0..COLUMN_SECTIONS).filter(|sy| flags >> sy & 1 == 1) {
        let (data, rest) = body
            .split_at_checked(SECTION_VOLUME)
            .ok_or(SaveError::Truncated)?;
        body = rest;
        section_data.push((sy, data));
    }
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
    // После серий: `Column::set` в них обнуляет данные клетки.
    for (sy, data) in section_data {
        let data: Box<[u8; SECTION_VOLUME]> = Box::new(data.try_into().expect("section data len"));
        column.set_section_data(sy, Some(data));
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
