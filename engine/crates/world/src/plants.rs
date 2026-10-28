use glam::IVec2;

use crate::block::Block;
use crate::column::{Column, WORLD_HEIGHT};
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
pub fn plant(terrain: &Terrain, key: IVec2, column: &mut Column) {
    let origin = key * SECTION_SIZE;
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
            if let Some(block) = plant_at(terrain, origin.x + lx, origin.y + lz) {
                column.set(above, block);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Колонки с доминирующим в центре биомом `biome`, не больше `count`.
    fn columns_of(terrain: &Terrain, biome: Biome, count: usize) -> Vec<IVec2> {
        (-40..40)
            .flat_map(|z| (-40..40).map(move |x| IVec2::new(x, z)))
            .filter(|k| terrain.biome_at(k.x * 16 + 8, k.y * 16 + 8) == biome)
            .take(count)
            .collect()
    }

    fn count(column: &Column, pred: impl Fn(Block) -> bool) -> usize {
        (0..16)
            .flat_map(|z| (0..WORLD_HEIGHT).flat_map(move |y| (0..16).map(move |x| [x, y, z])))
            .filter(|&p| pred(column.get(p.into())))
            .count()
    }

    #[test]
    fn same_seed_same_plants() {
        let key = IVec2::new(3, -2);
        let a = Terrain::new(42).generate(key);
        let b = Terrain::new(42).generate(key);
        for y in 0..WORLD_HEIGHT {
            for z in 0..16 {
                for x in 0..16 {
                    assert_eq!(a.get([x, y, z].into()), b.get([x, y, z].into()));
                }
            }
        }
        assert!(count(&a, |b| b == Block::TallGrass) > 0 || count(&a, Block::needs_support) == 0);
    }

    #[test]
    fn plains_have_reasonable_grass_share() {
        let terrain = Terrain::new(7);
        let (mut grass, mut flowers, mut surface) = (0, 0, 0);
        for key in columns_of(&terrain, Biome::Plains, 12) {
            let column = terrain.generate(key);
            grass += count(&column, |b| b == Block::TallGrass);
            flowers += count(&column, |b| matches!(b, Block::Poppy | Block::Dandelion));
            // Подходящие клетки — это те, где что-то выросло, плюс свободная трава.
            for z in 0..16 {
                for x in 0..16 {
                    let top = (0..WORLD_HEIGHT)
                        .rev()
                        .find(|&y| column.get([x, y, z].into()).is_solid());
                    if top.is_some_and(|y| column.get([x, y, z].into()) == Block::Grass) {
                        surface += 1;
                    }
                }
            }
        }
        let share = grass as f32 / surface as f32;
        assert!((0.06..0.2).contains(&share), "grass share {share}");
        assert!(
            flowers > 0 && flowers < grass / 3,
            "{flowers} flowers, {grass} grass"
        );
    }

    #[test]
    fn desert_has_no_plants() {
        let terrain = Terrain::new(7);
        for key in columns_of(&terrain, Biome::Desert, 6) {
            let column = terrain.generate(key);
            for z in 0..16 {
                for x in 0..16 {
                    let has_plant =
                        (0..WORLD_HEIGHT).any(|y| column.get([x, y, z].into()).needs_support());
                    // Край пустыни может зайти в равнину, но в самой пустыне пусто.
                    let biome = terrain.biome_at(key.x * 16 + x, key.y * 16 + z);
                    assert!(!has_plant || biome != Biome::Desert, "{key} {x},{z}");
                }
            }
        }
    }

    #[test]
    fn plants_stand_on_dry_grass_only() {
        let terrain = Terrain::new(7);
        for key in columns_of(&terrain, Biome::Forest, 6)
            .into_iter()
            .chain(columns_of(&terrain, Biome::Plains, 6))
        {
            let column = terrain.generate(key);
            for y in 0..WORLD_HEIGHT {
                for z in 0..16 {
                    for x in 0..16 {
                        let block = column.get([x, y, z].into());
                        if block.needs_support() {
                            assert_eq!(column.get([x, y - 1, z].into()), Block::Grass);
                            assert!(y - 1 > WATER_LEVEL, "plant at y {y} near water");
                        }
                    }
                }
            }
        }
    }
}
