use super::*;

const SEED: i32 = 1337;
const MESA_SEED: i32 = 99;

fn columns_of(terrain: &Terrain, biome: Biome, count: usize) -> Vec<ColumnPos> {
    (-200..200)
        .flat_map(|z| (-200..200).map(move |x| ColumnPos::new(x, z)))
        .filter(|k| terrain.sample_biome(k.origin().x + 8, k.origin().z + 8) == biome)
        .take(count)
        .collect()
}

fn mesa_columns(terrain: &Terrain, biome: Biome, count: usize) -> Vec<ColumnPos> {
    let at = |k: ColumnPos| terrain.sample_biome(k.origin().x + 8, k.origin().z + 8);
    let mut found: Vec<ColumnPos> = Vec::new();
    for z in (-600..600).step_by(4) {
        for x in (-600..600).step_by(4) {
            let hit = ColumnPos::new(x, z);
            if found.len() >= count {
                return found;
            }
            if found.contains(&hit) || at(hit) != biome {
                continue;
            }
            for dz in -4..4 {
                for dx in -4..4 {
                    let k = ColumnPos::new(x + dx, z + dz);
                    if at(k) == biome && !found.contains(&k) {
                        found.push(k);
                    }
                }
            }
        }
    }
    assert!(!found.is_empty(), "no {biome:?} near the origin");
    found
}

fn cells() -> impl Iterator<Item = IVec3> {
    (0..WORLD_HEIGHT)
        .flat_map(|y| (0..16).flat_map(move |z| (0..16).map(move |x| [x, y, z].into())))
}

fn find(column: &Column, block: Block) -> Vec<IVec3> {
    cells().filter(|&p| column.get(p) == block).collect()
}

fn same_columns(a: &Column, b: &Column) -> bool {
    cells().all(|p| a.get(p) == b.get(p))
}

#[test]
fn decoration_is_deterministic() {
    let (a, b) = (Terrain::new(SEED), Terrain::new(SEED));
    for key in columns_of(&a, Biome::Desert, 4) {
        assert!(same_columns(&a.generate(key), &b.generate(key)));
    }
}

#[test]
fn desert_has_cacti_on_sand_with_free_sides() {
    let terrain = Terrain::new(SEED);
    let mut total = 0;
    for key in columns_of(&terrain, Biome::Desert, 10) {
        let column = terrain.generate(key);
        for pos in find(&column, Block::Cactus) {
            total += 1;
            let mut base = pos;
            while column.get(base - IVec3::Y) == Block::Cactus {
                base -= IVec3::Y;
            }
            assert!(column.get(base - IVec3::Y).is_sand(), "{key} {pos}");
            assert!(
                pos.y - base.y < 3,
                "stack of {} at {key} {pos}",
                pos.y - base.y + 1
            );
            for dir in HORIZONTAL {
                let side = pos + dir;
                assert!(
                    !Column::contains(side) || !column.get(side).is_solid(),
                    "{key} {pos}"
                );
            }
        }
    }
    assert!(total > 20, "{total} cacti");
}

#[test]
fn desert_has_dead_bushes_on_allowed_ground() {
    let terrain = Terrain::new(SEED);
    let mut total = 0;
    for key in columns_of(&terrain, Biome::Desert, 20) {
        let column = terrain.generate(key);
        for pos in find(&column, Block::DeadBush) {
            total += 1;
            assert!(
                Block::DeadBush.stands_on(column.get(pos - IVec3::Y)),
                "{key} {pos}"
            );
        }
    }
    assert!(total > 5, "{total} dead bushes");
}

#[test]
fn mesa_has_many_dead_bushes_and_few_cacti_on_red_sand() {
    let terrain = Terrain::new(MESA_SEED);
    let (mut bushes, mut cacti) = (0, 0);
    for key in mesa_columns(&terrain, Biome::Mesa, 12) {
        let column = terrain.generate(key);
        bushes += find(&column, Block::DeadBush).len();
        for pos in find(&column, Block::Cactus) {
            if !terrain
                .biome_at(key.origin().x + pos.x, key.origin().z + pos.z)
                .is_mesa()
            {
                continue;
            }
            cacti += 1;
            let mut base = pos;
            while column.get(base - IVec3::Y) == Block::Cactus {
                base -= IVec3::Y;
            }
            assert_eq!(column.get(base - IVec3::Y), Block::RedSand);
        }
    }
    assert!(bushes > 20, "{bushes} dead bushes");
    assert!(cacti > 0, "{cacti} cacti");
}

#[test]
fn grassland_has_no_cacti_or_dead_bushes() {
    let terrain = Terrain::new(SEED);
    for key in columns_of(&terrain, Biome::Plains, 8) {
        let column = terrain.generate(key);
        assert!(find(&column, Block::Cactus).is_empty());
        assert!(find(&column, Block::DeadBush).is_empty());
    }
}

#[test]
fn reeds_stand_at_the_waterline_two_to_four_high() {
    let terrain = Terrain::new(SEED);
    let mut total = 0;
    for z in -40..40 {
        for x in -40..40 {
            let key = ColumnPos::new(x, z);
            let column = terrain.generate(key);
            for pos in find(&column, Block::Reeds) {
                total += 1;
                let mut base = pos;
                while column.get(base - IVec3::Y) == Block::Reeds {
                    base -= IVec3::Y;
                }
                assert_eq!(base.y, WATER_LEVEL + 1, "{key} {pos}");
                let ground = base - IVec3::Y;
                assert!(Block::Reeds.stands_on(column.get(ground)), "{key} {pos}");
                assert!(pos.y - base.y < 4, "{key} {pos}");
                let wet = HORIZONTAL.iter().any(|&d| {
                    let side = ground + d;
                    if Column::contains(side) {
                        column.get(side) == Block::Water
                    } else {
                        true
                    }
                });
                assert!(wet, "{key} {pos}");
            }
        }
    }
    assert!(total > 0, "no reeds in 6400 columns");
}

#[test]
fn mesa_has_colored_bands_under_red_sand() {
    let terrain = Terrain::new(MESA_SEED);
    let mut kinds = std::collections::BTreeSet::new();
    for key in mesa_columns(&terrain, Biome::Mesa, 12) {
        let column = terrain.generate_ground(key);
        for p in cells().filter(|p| column.get(*p).is_terracotta()) {
            kinds.insert(column.get(p) as u8);
        }
        let o = key.origin();
        let surface = terrain.surface_height(o.x + 8, o.z + 8);
        if surface <= WATER_LEVEL {
            continue;
        }
        let top = column.get([8, surface, 8].into());
        assert!(
            top == Block::RedSand || top.is_terracotta() || top == Block::Grass,
            "{top:?}"
        );
    }
    assert!(kinds.len() >= 4, "{kinds:?}");
    assert!(
        kinds.contains(&(Block::Terracotta as u8))
            && kinds.contains(&(Block::OrangeTerracotta as u8))
    );
}

#[test]
fn bryce_pillars_rise_above_the_plain() {
    let terrain = Terrain::new(MESA_SEED);
    let keys = mesa_columns(&terrain, Biome::MesaBryce, 30);
    let tops: Vec<i32> = keys
        .iter()
        .flat_map(|k| {
            let o = k.origin();
            (0..16).flat_map(move |z| (0..16).map(move |x| (o.x + x, o.z + z)))
        })
        .map(|(x, z)| terrain.surface_height(x, z))
        .collect();
    assert!(
        tops.iter().max().copied().unwrap_or(0) > 80,
        "{:?}",
        tops.iter().max()
    );
}

#[test]
fn forest_plateau_has_oaks() {
    let terrain = Terrain::new(MESA_SEED);
    let logs: usize = mesa_columns(&terrain, Biome::MesaPlateauF, 30)
        .into_iter()
        .map(|key| find(&terrain.generate(key), Block::Log).len())
        .sum();
    assert!(logs > 0);
}
