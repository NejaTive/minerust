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
    Sand,
    Log,
    Leaves,
}

/// Куда смотрит грань блока; от этого зависит её тайл.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Facing {
    Top,
    Bottom,
    Side,
}

impl Block {
    /// Для коллизий, прицела и проверки установки: всё, кроме воздуха.
    pub fn is_solid(self) -> bool {
        self != Block::Air
    }

    /// Закрывает ли блок грань соседа и участвует ли в затенении углов: всё, кроме
    /// воздуха и листвы. Сквозь дырки листвы видно и грани за ней, и соседнюю листву.
    pub fn is_opaque(self) -> bool {
        !matches!(self, Block::Air | Block::Leaves)
    }

    /// Блок, который игрок может выбрать, по имени из интерфейса: `"grass"`, `"dirt"`,
    /// `"stone"`, `"sand"`, `"log"`, `"leaves"`. Воздух не выбирается: для него есть ломание.
    pub fn from_name(name: &str) -> Option<Block> {
        match name {
            "grass" => Some(Block::Grass),
            "dirt" => Some(Block::Dirt),
            "stone" => Some(Block::Stone),
            "sand" => Some(Block::Sand),
            "log" => Some(Block::Log),
            "leaves" => Some(Block::Leaves),
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
            (Block::Sand, _) => Tile::Sand,
            (Block::Log, Facing::Top | Facing::Bottom) => Tile::LogTop,
            (Block::Log, Facing::Side) => Tile::LogSide,
            (Block::Leaves, _) => Tile::Leaves,
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
            4 => Block::Sand,
            5 => Block::Log,
            6 => Block::Leaves,
            other => return Err(other),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Block; 7] = [
        Block::Air,
        Block::Grass,
        Block::Dirt,
        Block::Stone,
        Block::Sand,
        Block::Log,
        Block::Leaves,
    ];

    #[test]
    fn byte_round_trip() {
        for (i, block) in ALL.into_iter().enumerate() {
            assert_eq!(block as usize, i, "save numbers must not move");
            assert_eq!(Block::try_from(block as u8), Ok(block));
        }
        assert_eq!(Block::try_from(7), Err(7));
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
        assert_eq!(Block::from_name("sand"), Some(Block::Sand));
        assert_eq!(Block::from_name("log"), Some(Block::Log));
        assert_eq!(Block::from_name("leaves"), Some(Block::Leaves));
        assert_eq!(Block::from_name("air"), None);
        assert_eq!(Block::from_name("Stone"), None);
    }

    #[test]
    fn solid_and_opaque() {
        for block in ALL {
            assert_eq!(block.is_solid(), block != Block::Air, "{block:?}");
            let opaque = !matches!(block, Block::Air | Block::Leaves);
            assert_eq!(block.is_opaque(), opaque, "{block:?}");
        }
    }

    #[test]
    fn log_has_rings_on_ends_and_bark_on_sides() {
        assert_eq!(Block::Log.tile(Facing::Top), Some(Tile::LogTop));
        assert_eq!(Block::Log.tile(Facing::Bottom), Some(Tile::LogTop));
        assert_eq!(Block::Log.tile(Facing::Side), Some(Tile::LogSide));
        assert_eq!(Block::Sand.tile(Facing::Top), Some(Tile::Sand));
        assert_eq!(Block::Leaves.tile(Facing::Side), Some(Tile::Leaves));
    }
}
