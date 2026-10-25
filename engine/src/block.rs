/// Тип блока. `repr(u8)`, чтобы секция занимала байт на блок.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Block {
    #[default]
    Air,
    Grass,
    Dirt,
    Stone,
}

impl Block {
    /// Закрывает ли блок грань соседа: грань к твёрдому блоку не рисуется.
    pub fn is_solid(self) -> bool {
        self != Block::Air
    }

    /// Базовый цвет в линейном RGB (поверхность sRGB сама переведёт при выводе).
    /// У воздуха граней нет, его цвет не используется.
    pub fn color(self) -> [f32; 3] {
        match self {
            Block::Air => [0.0; 3],
            Block::Grass => [0.30, 0.60, 0.20],
            Block::Dirt => [0.45, 0.30, 0.15],
            Block::Stone => [0.40, 0.40, 0.42],
        }
    }
}
