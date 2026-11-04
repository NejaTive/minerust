use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::path::PathBuf;

use glam::IVec3;

use super::super::*;
use crate::block::{Block, DataKind, facing_dir_of};

const SENTINEL: Block = Block::Bedrock;
const SYMBOLS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
const FRONT_MASK: u8 = 0b11;

fn local_size(piece: &Piece) -> IVec3 {
    let size = piece.bounds.size();
    match piece.facing {
        Facing::North | Facing::South => size,
        Facing::East | Facing::West => IVec3::new(size.z, size.y, size.x),
    }
}

fn has_front(block: Block) -> bool {
    matches!(
        block.data_kind(),
        DataKind::Stairs
            | DataKind::Ladder
            | DataKind::Furnace
            | DataKind::Trapdoor
            | DataKind::Bed
            | DataKind::Chest
    )
}

fn front_name(dir: IVec3) -> &'static str {
    match dir {
        IVec3::X => "+x",
        IVec3::NEG_X => "-x",
        IVec3::Z => "+z",
        _ => "-z",
    }
}

fn place_all(piece: &Piece, below: i32) -> HashMap<ColumnPos, Column> {
    let b = piece.bounds;
    let (low, high) = (ColumnPos::of_block(b.min), ColumnPos::of_block(b.max));
    let mut columns = HashMap::new();
    for z in low.z()..=high.z() {
        for x in low.x()..=high.x() {
            let key = ColumnPos::new(x, z);
            let mut column = Column::new();
            for y in b.min.y - below..=b.max.y {
                for bz in b.min.z.max(key.origin().z)..=b.max.z.min(key.origin().z + 15) {
                    for bx in b.min.x.max(key.origin().x)..=b.max.x.min(key.origin().x + 15) {
                        column.set(IVec3::new(bx, y, bz) - key.origin(), SENTINEL);
                    }
                }
            }
            let mut writer = PieceWriter::new(&mut column, key, piece);
            piece.place(&mut writer);
            columns.insert(key, column);
        }
    }
    columns
}

/// Часть в формате шаблона: оси локальные, повёрнутые блоки приведены к `front` части.
pub(super) fn dump(piece: &Piece) -> String {
    dump_below(piece, 0)
}

/// Дамп с `below` слоями ниже локального нуля (слои с отрицательным `y`).
pub(super) fn dump_below(piece: &Piece, below: i32) -> String {
    let columns = place_all(piece, below);
    let mut scratch = Column::new();
    let probe = PieceWriter::new(&mut scratch, ColumnPos::ZERO, piece);
    let size = local_size(piece) + IVec3::new(0, below, 0);
    let mut symbols: Vec<(String, char)> = Vec::new();
    let mut layers = String::new();
    for y in -below..size.y - below {
        writeln!(layers, "y {y}").unwrap();
        for z in 0..size.z {
            for x in 0..size.x {
                let at = probe.world(x, y, z);
                let key = ColumnPos::of_block(at);
                let column = &columns[&key];
                let (block, data) = (
                    column.get(at - key.origin()),
                    column.get_data(at - key.origin()),
                );
                let spec = if block == SENTINEL {
                    "keep".to_string()
                } else {
                    spec_of(&probe, block, data)
                };
                let ch = match spec.as_str() {
                    "air" => '.',
                    "keep" => '_',
                    _ => match symbols.iter().find(|(s, _)| *s == spec) {
                        Some((_, ch)) => *ch,
                        None => {
                            let ch = SYMBOLS.chars().nth(symbols.len()).expect("too many tiles");
                            symbols.push((spec, ch));
                            ch
                        }
                    },
                };
                layers.push(ch);
            }
            layers.push('\n');
        }
    }
    let mut out = format!(
        "size {} {} {}\nlegend\n. air\n_ keep\n",
        size.x, size.y, size.z
    );
    for (spec, ch) in &symbols {
        writeln!(out, "{ch} {spec}").unwrap();
    }
    out + &layers
}

fn spec_of(probe: &PieceWriter, block: Block, data: u8) -> String {
    let name = block.def().name;
    assert!(!name.is_empty(), "block {block:?} has no name");
    let mut spec = name.to_string();
    let mut rest = data;
    if has_front(block) {
        let world = facing_dir_of(data & FRONT_MASK);
        let local = [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z]
            .into_iter()
            .find(|&dir| probe.world_dir(dir) == world)
            .unwrap();
        write!(spec, " front={}", front_name(local)).unwrap();
        rest &= !FRONT_MASK;
    }
    if rest != 0 {
        write!(spec, " data={rest}").unwrap();
    }
    spec
}

fn snapshot_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("src/structure/snapshots/{name}.txt"))
}

pub(super) fn assert_snapshot(name: &str, actual: &str) {
    let path = snapshot_path(name);
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        fs::write(&path, actual).unwrap();
        return;
    }
    let expected = fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("нет снимка {name}: UPDATE_SNAPSHOTS=1 cargo test"));
    assert!(expected == actual, "снимок {name} разошёлся\n{actual}");
}
