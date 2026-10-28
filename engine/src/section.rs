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
}

impl Section {
    /// Секция целиком из воздуха.
    pub fn new() -> Self {
        Self {
            blocks: Box::new([Block::Air; SECTION_VOLUME]),
            emitters: 0,
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

    /// Нет ли в секции ничего, кроме воздуха.
    pub fn is_air(&self) -> bool {
        self.blocks.iter().all(|&b| b == Block::Air)
    }

    /// Ставит блок по локальным координатам; за пределами 0..16 — panic.
    pub fn set(&mut self, local: IVec3, block: Block) {
        let slot = &mut self.blocks[Self::index(local)];
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
mod tests {
    use super::*;

    #[test]
    fn set_then_get() {
        let mut section = Section::new();
        let pos = IVec3::new(15, 3, 7);
        section.set(pos, Block::Stone);
        assert_eq!(section.get(pos), Block::Stone);
        assert_eq!(section.get(IVec3::new(15, 3, 6)), Block::Air);
    }

    #[test]
    #[should_panic]
    fn get_out_of_bounds_panics() {
        Section::new().get(IVec3::new(16, 0, 0));
    }
}
