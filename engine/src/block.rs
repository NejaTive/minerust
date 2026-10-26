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

    /// Блок, который игрок может выбрать, по имени из интерфейса: `"grass"`, `"dirt"`,
    /// `"stone"`. Воздух не выбирается: для него есть ломание.
    pub fn from_name(name: &str) -> Option<Block> {
        match name {
            "grass" => Some(Block::Grass),
            "dirt" => Some(Block::Dirt),
            "stone" => Some(Block::Stone),
            _ => None,
        }
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

/// Обратное к `block as u8`: так блоки читаются из сохранения. Ошибка — сам незнакомый
/// байт. Номера вариантов — часть формата сохранения: новые блоки добавлять в конец.
impl TryFrom<u8> for Block {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, u8> {
        Ok(match value {
            0 => Block::Air,
            1 => Block::Grass,
            2 => Block::Dirt,
            3 => Block::Stone,
            other => return Err(other),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_round_trip() {
        for block in [Block::Air, Block::Grass, Block::Dirt, Block::Stone] {
            assert_eq!(Block::try_from(block as u8), Ok(block));
        }
        assert_eq!(Block::try_from(4), Err(4));
        assert_eq!(Block::try_from(255), Err(255));
    }

    #[test]
    fn grass_has_distinct_top_side_bottom() {
        assert_eq!(Block::Grass.tile(Facing::Top), Some(Tile::GrassTop));
        assert_eq!(Block::Grass.tile(Facing::Side), Some(Tile::GrassSide));
        assert_eq!(Block::Grass.tile(Facing::Bottom), Some(Tile::Dirt));
        assert_eq!(Block::Air.tile(Facing::Top), None);
    }

    #[test]
    fn selectable_by_name() {
        assert_eq!(Block::from_name("grass"), Some(Block::Grass));
        assert_eq!(Block::from_name("dirt"), Some(Block::Dirt));
        assert_eq!(Block::from_name("stone"), Some(Block::Stone));
        assert_eq!(Block::from_name("air"), None);
        assert_eq!(Block::from_name("Stone"), None);
    }
}
