use std::collections::HashMap;

use glam::IVec3;

use super::cell::{Cell, DOWN, NEG_X, NEG_Z, Selector, UP, X, Z};
use crate::bed::COLOR_SHIFT;
use crate::block::{Block, COLOR_COUNT, DataKind};

#[derive(Clone, Debug)]
pub enum Entry {
    Keep,
    Cell(Cell),
    Random(Selector),
    /// Клетка с шансом «один из N»; иначе клетка не трогается.
    Chance(u32, Cell),
}

/// Слои клеток из текстового файла: формат описан в `.claude/plan/structure-api.md`.
#[derive(Debug)]
pub struct Template {
    size: IVec3,
    entries: Vec<Entry>,
}

impl Template {
    pub fn size(&self) -> IVec3 {
        self.size
    }

    pub fn entry(&self, x: i32, y: i32, z: i32) -> &Entry {
        let index = (y * self.size.z + z) * self.size.x + x;
        &self.entries[index as usize]
    }

    /// Разбирает шаблон; ошибка — `panic!` с именем файла и номером строки.
    pub fn parse(file: &str, text: &str) -> Template {
        Parser {
            file,
            line: 0,
            size: None,
            legend: HashMap::new(),
            entries: Vec::new(),
            layers: 0,
            rows: 0,
        }
        .run(text)
    }
}

struct Parser<'a> {
    file: &'a str,
    line: usize,
    size: Option<IVec3>,
    legend: HashMap<char, Entry>,
    entries: Vec<Entry>,
    layers: i32,
    rows: i32,
}

impl Parser<'_> {
    fn fail(&self, message: &str) -> ! {
        panic!("{}:{}: {message}", self.file, self.line);
    }

    fn run(mut self, text: &str) -> Template {
        let mut in_legend = false;
        for (index, raw) in text.lines().enumerate() {
            self.line = index + 1;
            let line = raw.split(" #").next().unwrap().trim_end();
            if line.is_empty() {
                continue;
            }
            let mut words = line.split_whitespace();
            match (words.next().unwrap(), self.size) {
                ("size", None) => {
                    let dims: Vec<i32> = words.map(|w| self.number(w)).collect();
                    let [x, y, z] = dims[..] else {
                        self.fail("size wants three numbers");
                    };
                    self.size = Some(IVec3::new(x, y, z));
                }
                (_, None) => self.fail("expected `size X Y Z` first"),
                ("legend", Some(_)) => in_legend = true,
                ("y", Some(_)) if line[1..].trim().parse::<i32>().is_ok() => {
                    in_legend = false;
                    let layer = self.number(words.next().unwrap_or(""));
                    if layer != self.layers {
                        self.fail(&format!("expected layer y {}", self.layers));
                    }
                    self.layers += 1;
                    self.rows = 0;
                }
                _ if in_legend => self.legend_line(line),
                _ if self.layers > 0 => self.row(line),
                _ => self.fail("expected `legend` or `y N`"),
            }
        }
        let size = self.size.unwrap_or_else(|| self.fail("empty template"));
        if self.layers != size.y || self.rows != size.z {
            self.fail("template ends before every layer is complete");
        }
        Template {
            size,
            entries: self.entries,
        }
    }

    fn number(&self, word: &str) -> i32 {
        word.parse()
            .unwrap_or_else(|_| self.fail(&format!("`{word}` is not a number")))
    }

    fn byte(&self, word: &str) -> u8 {
        u8::try_from(self.number(word)).unwrap_or_else(|_| self.fail("value out of range"))
    }

    fn legend_line(&mut self, line: &str) {
        let (symbol, spec) = line.split_once(' ').unwrap_or((line, ""));
        let mut chars = symbol.chars();
        let (Some(symbol), None) = (chars.next(), chars.next()) else {
            self.fail("legend symbol must be one character");
        };
        let entry = self.entry_of(spec.trim());
        if self.legend.insert(symbol, entry).is_some() {
            self.fail(&format!("symbol `{symbol}` is defined twice"));
        }
    }

    fn entry_of(&self, spec: &str) -> Entry {
        let mut words = spec.split_whitespace();
        match words.next() {
            Some("keep") => Entry::Keep,
            Some("random") => {
                let weighted: Vec<(u32, Cell)> = words
                    .map(|word| {
                        let (weight, name) = word
                            .split_once(':')
                            .unwrap_or_else(|| self.fail("random wants `weight:block`"));
                        let weight = weight
                            .parse()
                            .unwrap_or_else(|_| self.fail("bad random weight"));
                        (weight, Cell::of(self.block(name)))
                    })
                    .collect();
                if weighted.iter().all(|(weight, _)| *weight == 0) {
                    self.fail("random without weights");
                }
                Entry::Random(Selector::new(&weighted))
            }
            Some(name) => self.cell_of(name, words),
            None => self.fail("empty legend entry"),
        }
    }

    fn cell_of<'w>(&self, name: &str, options: impl Iterator<Item = &'w str>) -> Entry {
        let block = self.block(name);
        let (mut data, mut front, mut chance) = (0u8, None, None);
        for option in options {
            let (key, value) = option
                .split_once('=')
                .unwrap_or_else(|| self.fail(&format!("bad option `{option}`")));
            match key {
                "front" => front = Some(self.front(value)),
                "data" => data |= self.byte(value),
                "color" if block.data_kind() == DataKind::Bed => {
                    data |= (self.byte(value) % COLOR_COUNT) << COLOR_SHIFT;
                }
                "color" => data |= self.byte(value),
                "chance" => chance = Some(self.number(value) as u32),
                _ => self.fail(&format!("unknown option `{key}`")),
            }
        }
        let cell = Cell { block, data, front };
        match chance {
            Some(one_in) if one_in > 0 => Entry::Chance(one_in, cell),
            Some(_) => self.fail("chance is `one in N`, N > 0"),
            None => Entry::Cell(cell),
        }
    }

    fn front(&self, value: &str) -> IVec3 {
        match value {
            "+x" => X,
            "-x" => NEG_X,
            "+z" => Z,
            "-z" => NEG_Z,
            "up" => UP,
            "down" => DOWN,
            _ => self.fail(&format!("unknown front `{value}`")),
        }
    }

    fn block(&self, name: &str) -> Block {
        Block::ALL
            .into_iter()
            .find(|block| block.def().name == name)
            .unwrap_or_else(|| self.fail(&format!("unknown block `{name}`")))
    }

    fn row(&mut self, line: &str) {
        let size = self.size.unwrap();
        if self.rows == size.z {
            self.fail("too many rows in the layer");
        }
        if line.chars().count() as i32 != size.x {
            self.fail(&format!("row must be {} symbols wide", size.x));
        }
        for symbol in line.chars() {
            let entry = match symbol {
                '.' => Entry::Cell(Cell::of(Block::Air)),
                '_' => Entry::Keep,
                _ => self
                    .legend
                    .get(&symbol)
                    .cloned()
                    .unwrap_or_else(|| self.fail(&format!("unknown symbol `{symbol}`"))),
            };
            self.entries.push(entry);
        }
        self.rows += 1;
    }
}
