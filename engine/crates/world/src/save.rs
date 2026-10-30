use std::borrow::Cow;
use std::fmt;

use glam::IVec3;

use crate::block::Block;
use crate::column::{COLUMN_SECTIONS, Column, WORLD_HEIGHT};
use crate::section::{SECTION_SIZE, SECTION_VOLUME};

pub const FORMAT_VERSION: u8 = 2;

type Migration = fn(&[u8]) -> Result<Vec<u8>, SaveError>;

const MIGRATIONS: &[Migration] = &[v1_to_v2];

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

const _: () = assert!(COLUMN_SECTIONS <= 8);

const _: () = assert!(COLUMN_VOLUME <= u16::MAX as usize);

#[derive(Debug, PartialEq, Eq)]
pub enum SaveError {
    BadMagic,
    UnsupportedVersion(u8),
    UnknownBlock(u8),
    Truncated,
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
    for (sy, data) in section_data {
        let data: Box<[u8; SECTION_VOLUME]> = Box::new(data.try_into().expect("section data len"));
        column.set_section_data(sy, Some(data));
    }
    Ok(column)
}

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
