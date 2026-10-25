use crate::texture::Tile;

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

/// Куда смотрит грань блока; от этого зависит её тайл.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Facing {
    Top,
    Bottom,
    Side,
}

impl Block {
    /// Закрывает ли блок грань соседа: грань к твёрдому блоку не рисуется.
    pub fn is_solid(self) -> bool {
        self != Block::Air
    }

    /// Тайл грани; `None` — у блока нет граней (воздух).
    pub fn tile(self, facing: Facing) -> Option<Tile> {
        Some(match (self, facing) {
            (Block::Air, _) => return None,
            (Block::Grass, Facing::Top) => Tile::GrassTop,
            (Block::Grass, Facing::Side) => Tile::GrassSide,
            (Block::Grass, Facing::Bottom) | (Block::Dirt, _) => Tile::Dirt,
            (Block::Stone, _) => Tile::Stone,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grass_has_distinct_top_side_bottom() {
        assert_eq!(Block::Grass.tile(Facing::Top), Some(Tile::GrassTop));
        assert_eq!(Block::Grass.tile(Facing::Side), Some(Tile::GrassSide));
        assert_eq!(Block::Grass.tile(Facing::Bottom), Some(Tile::Dirt));
        assert_eq!(Block::Air.tile(Facing::Top), None);
    }
}
