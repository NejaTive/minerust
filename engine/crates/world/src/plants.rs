use crate::block::Block;
use crate::column::{Column, WORLD_HEIGHT};
use crate::pos::ColumnPos;
use crate::section::SECTION_SIZE;
use crate::terrain::{Biome, Terrain, WATER_LEVEL};
use crate::trees::hash;

/// Доля подходящих клеток с высокой травой. Густая поросль забила бы вид на землю и
/// дала бы по кресту на каждом втором блоке: в мешере это +8 вершин на клетку, а ~12%
/// читается как луг и почти не меняет время меша колонки.
const GRASS_DENSITY: f32 = 0.12;
/// Доля клеток с цветком (мак или одуванчик поровну): редкие яркие пятна, а не ковёр.
const FLOWER_DENSITY: f32 = 0.01;

// Соль хеша отдельная от деревьев: решение о растении не коррелирует с решением о дереве.
const SALT_PLANT: u64 = 11;
const SALT_KIND: u64 = 12;

/// Растение в клетке над поверхностью столбца (x, z) мировых координат, если оно там
/// растёт. Зависит только от сида и координат. Биом и высота проверяются тут, а
/// не блок под ногами: это дешёвый фильтр до обхода колонки.
fn plant_at(terrain: &Terrain, x: i32, z: i32) -> Option<Block> {
    let roll = (hash(terrain.seed(), x, z, SALT_PLANT) >> 8) as f32 / (1 << 24) as f32;
    // Шум биомов дороже хеша: отсекаем пустые клетки до него.
    if roll >= GRASS_DENSITY + FLOWER_DENSITY {
        return None;
    }
    if !matches!(terrain.biome_at(x, z), Biome::Plains | Biome::Forest) {
        return None;
    }
    if roll < FLOWER_DENSITY {
        let poppy = hash(terrain.seed(), x, z, SALT_KIND) & 1 == 0;
        return Some(if poppy {
            Block::Poppy
        } else {
            Block::Dandelion
        });
    }
    Some(Block::TallGrass)
}

/// Ставит в колонку `key` траву и цветы на верхний блок `Grass` с воздухом над ним в
/// равнинах и лесу. Работает только внутри колонки: решение по клетке зависит от сида
/// и мировых координат, так что шов с соседями не нужен. Пустыня, пляж (там песок) и
/// дно озёр отпадают сами: растёт только на траве выше воды.
pub fn plant(terrain: &Terrain, key: ColumnPos, column: &mut Column) {
    let origin = key.origin();
    for lz in 0..SECTION_SIZE {
        for lx in 0..SECTION_SIZE {
            let Some(top) = (0..WORLD_HEIGHT)
                .rev()
                .find(|&y| column.get([lx, y, lz].into()) != Block::Air)
            else {
                continue;
            };
            let ground = glam::IVec3::new(lx, top, lz);
            let above = ground + glam::IVec3::Y;
            if column.get(ground) != Block::Grass
                || top + 1 >= WORLD_HEIGHT
                || top <= WATER_LEVEL
                || column.get(above) != Block::Air
            {
                continue;
            }
            if let Some(block) = plant_at(terrain, origin.x + lx, origin.z + lz) {
                column.set(above, block);
            }
        }
    }
}

#[cfg(test)]
mod tests;
