use glam::IVec3;

use crate::block::Block;

pub const SECTION_SIZE: i32 = 16;
pub const SECTION_VOLUME: usize = 4096;

/// Секция 16³. Индекс блока: x + 16·z + 256·y.
#[derive(Clone)]
pub struct Section {
    // В куче: на шаге 4 секций будут тысячи, и их перемещают по значению.
    blocks: Box<[Block; SECTION_VOLUME]>,
    /// Сколько блоков излучает свет: обход 4096 блоков на каждую подгрузку колонки
    /// заметно дороже счётчика, а светящихся блоков почти нет.
    emitters: u16,
    /// Байт данных на блок (см. [`crate::block::DataKind`]), тот же индекс. Массива нет,
    /// пока никто не записал ненулевой байт: почти все секции без данных, и 4 КиБ на
    /// каждую были бы лишними; `None` читается как нули.
    data: Option<Box<[u8; SECTION_VOLUME]>>,
}

impl Section {
    /// Секция целиком из воздуха.
    pub fn new() -> Self {
        Self {
            blocks: Box::new([Block::Air; SECTION_VOLUME]),
            emitters: 0,
            data: None,
        }
    }

    /// Лежат ли локальные координаты внутри секции (0..16 по каждой оси).
    pub fn contains(local: IVec3) -> bool {
        local.cmpge(IVec3::ZERO).all() && local.cmplt(IVec3::splat(SECTION_SIZE)).all()
    }

    /// Блок по локальным координатам; за пределами 0..16 — panic.
    pub fn get(&self, local: IVec3) -> Block {
        self.blocks[Self::index(local)]
    }

    /// Есть ли в секции излучающий свет блок.
    pub fn emits_light(&self) -> bool {
        self.emitters > 0
    }

    /// Блок по индексу `x + 16·z + 256·y` (0..SECTION_VOLUME): для горячих циклов,
    /// где индекс уже посчитан; за пределами — panic.
    pub fn get_index(&self, index: usize) -> Block {
        self.blocks[index]
    }

    /// Блоки строки по x при заданных y и z (0..16): 16 подряд идущих блоков, чтобы
    /// кэш мешера копировал их срезом, а не по одному.
    pub fn row(&self, y: i32, z: i32) -> &[Block] {
        let start = (SECTION_SIZE * z + SECTION_SIZE * SECTION_SIZE * y) as usize;
        &self.blocks[start..start + SECTION_SIZE as usize]
    }

    /// Байты данных строки по x при заданных y и z (0..16), как [`Section::row`]; `None` —
    /// все нули.
    pub fn data_row(&self, y: i32, z: i32) -> Option<&[u8]> {
        let start = (SECTION_SIZE * z + SECTION_SIZE * SECTION_SIZE * y) as usize;
        self.data
            .as_ref()
            .map(|data| &data[start..start + SECTION_SIZE as usize])
    }

    /// Нет ли в секции ничего, кроме воздуха.
    pub fn is_air(&self) -> bool {
        self.blocks.iter().all(|&b| b == Block::Air)
    }

    /// Байт данных блока по локальным координатам; за пределами 0..16 — panic.
    pub fn get_data(&self, local: IVec3) -> u8 {
        let index = Self::index(local);
        self.data.as_ref().map_or(0, |data| data[index])
    }

    /// Все байты данных секции; `None` — все нули. Мешеру и сохранению.
    pub fn data(&self) -> Option<&[u8; SECTION_VOLUME]> {
        self.data.as_deref()
    }

    /// Подменяет байты данных целиком (загрузка сохранения). Нулевой массив не хранится.
    pub fn set_data(&mut self, data: Option<Box<[u8; SECTION_VOLUME]>>) {
        self.data = data.filter(|data| data.iter().any(|&b| b != 0));
    }

    /// Ставит блок с нулевыми данными; за пределами 0..16 — panic.
    pub fn set(&mut self, local: IVec3, block: Block) {
        self.set_with_data(local, block, 0);
    }

    /// Ставит блок и байт данных. Данные прежнего блока не переживают замену: байт
    /// значит разное у разных блоков.
    pub fn set_with_data(&mut self, local: IVec3, block: Block, data: u8) {
        let index = Self::index(local);
        match &mut self.data {
            Some(bytes) => bytes[index] = data,
            // Массив заводим только под ненулевой байт.
            None if data != 0 => self.data.insert(Box::new([0; SECTION_VOLUME]))[index] = data,
            None => {}
        }
        let slot = &mut self.blocks[index];
        self.emitters -= u16::from(slot.emission() > 0);
        self.emitters += u16::from(block.emission() > 0);
        *slot = block;
    }

    fn index(local: IVec3) -> usize {
        // Без этой проверки, например, x = 16 молча попал бы в соседнюю строку по z.
        assert!(
            Self::contains(local),
            "local coords out of section: {local}"
        );
        (local.x + SECTION_SIZE * local.z + SECTION_SIZE * SECTION_SIZE * local.y) as usize
    }
}

impl Default for Section {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
