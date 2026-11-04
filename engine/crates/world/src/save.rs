use std::borrow::Cow;
use std::fmt;

use glam::{IVec3, Vec3};

use crate::block::Block;
use crate::column::{COLUMN_SECTIONS, Column, WORLD_HEIGHT};
use crate::mob::{MobKind, MobRecord};
use crate::section::{SECTION_SIZE, SECTION_VOLUME};

pub const FORMAT_VERSION: u8 = 6;

type Migration = fn(&[u8]) -> Result<Vec<u8>, SaveError>;

const MIGRATIONS: &[Migration] = &[v1_to_v2, v2_to_v3, v3_to_v4, v4_to_v5, v5_to_v6];

fn v1_to_v2(body: &[u8]) -> Result<Vec<u8>, SaveError> {
    let mut out = Vec::with_capacity(body.len() + 1);
    out.push(0);
    out.extend_from_slice(body);
    Ok(out)
}

fn v2_to_v3(body: &[u8]) -> Result<Vec<u8>, SaveError> {
    let (&flags, rest) = body.split_first().ok_or(SaveError::Truncated)?;
    let mut out = Vec::with_capacity(body.len() + 2);
    out.push(flags);
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(rest);
    Ok(out)
}

fn v3_to_v4(body: &[u8]) -> Result<Vec<u8>, SaveError> {
    let (&flags, rest) = body.split_first().ok_or(SaveError::Truncated)?;
    let mut out = Vec::with_capacity(body.len() + 1);
    out.extend_from_slice(&[flags, 1]);
    out.extend_from_slice(rest);
    Ok(out)
}

fn v4_to_v5(_: &[u8]) -> Result<Vec<u8>, SaveError> {
    Err(SaveError::UnsupportedVersion(4))
}

fn v5_to_v6(_: &[u8]) -> Result<Vec<u8>, SaveError> {
    Err(SaveError::UnsupportedVersion(5))
}

const _: () = assert!(MIGRATIONS.len() + 1 == FORMAT_VERSION as usize);

const MAGIC: &[u8; 3] = b"MRC";
const COLUMN_VOLUME: usize = (SECTION_SIZE * SECTION_SIZE * WORLD_HEIGHT) as usize;
const RUN_SIZE: usize = 3;
const MOB_SIZE: usize = 18;

const _: () = assert!(COLUMN_SECTIONS <= 16);

#[derive(Debug, PartialEq, Eq)]
pub enum SaveError {
    BadMagic,
    UnsupportedVersion(u8),
    UnknownBlock(u8),
    Truncated,
    Overflow,
    UnknownMob(u8),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveError::BadMagic => write!(f, "not a column: bad magic"),
            SaveError::UnsupportedVersion(v) => write!(f, "unsupported column format {v}"),
            SaveError::UnknownBlock(b) => write!(f, "unknown block {b}"),
            SaveError::UnknownMob(m) => write!(f, "unknown mob {m}"),
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
    let flags = with_data
        .iter()
        .fold(0u16, |flags, &(sy, _)| flags | 1 << sy);
    bytes.extend_from_slice(&flags.to_le_bytes());
    bytes.push(u8::from(column.populated));
    bytes.extend_from_slice(&(column.mobs.len() as u16).to_le_bytes());
    for mob in &column.mobs {
        bytes.push(mob.kind as u8);
        for value in [mob.pos.x, mob.pos.y, mob.pos.z, mob.yaw] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.push(mob.variant);
    }
    for (_, data) in with_data {
        bytes.extend_from_slice(data);
    }
    let mut push_run = |block: Block, len: usize| {
        bytes.push(block as u8);
        bytes.extend_from_slice(&((len - 1) as u16).to_le_bytes());
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
    let (flags, body) = body.split_at_checked(2).ok_or(SaveError::Truncated)?;
    let flags = u16::from_le_bytes([flags[0], flags[1]]);
    let (&populated, mut body) = body.split_first().ok_or(SaveError::Truncated)?;
    let mut column = Column::new();
    column.populated = populated != 0;
    let (count, rest) = body.split_at_checked(2).ok_or(SaveError::Truncated)?;
    let count = usize::from(u16::from_le_bytes([count[0], count[1]]));
    let (mobs, rest) = rest
        .split_at_checked(count * MOB_SIZE)
        .ok_or(SaveError::Truncated)?;
    body = rest;
    for mob in mobs.as_chunks::<MOB_SIZE>().0 {
        let kind = MobKind::try_from(mob[0]).map_err(SaveError::UnknownMob)?;
        let f =
            |i: usize| f32::from_le_bytes(mob[1 + 4 * i..5 + 4 * i].try_into().expect("4 bytes"));
        column.mobs.push(MobRecord {
            kind,
            pos: Vec3::new(f(0), f(1), f(2)),
            yaw: f(3),
            variant: mob[17],
        });
    }
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
        let end = index + usize::from(u16::from_le_bytes([lo, hi])) + 1;
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
