use super::*;

fn columns_of(terrain: &Terrain, biome: Biome, count: usize) -> Vec<ColumnPos> {
    (-40..40)
        .flat_map(|z| (-40..40).map(move |x| ColumnPos::new(x, z)))
        .filter(|k| terrain.biome_at(k.origin().x + 8, k.origin().z + 8) == biome)
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
    let key = ColumnPos::new(3, -2);
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
                let biome = terrain.biome_at(key.origin().x + x, key.origin().z + z);
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
