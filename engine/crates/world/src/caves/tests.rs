use glam::IVec2;

use super::*;
use crate::terrain::{Terrain, lake_key};
use crate::trees::tree_at;

impl Caves {
    /// Вырезать ли блок по мировым координатам, без учёта ограничений (дно, вода,
    /// входы) и ослабления у поверхности — как на большой глубине.
    pub fn is_cave(&self, pos: IVec3) -> bool {
        hollow(self.sample(pos), 1.0)
    }

    /// Шумы в любой точке: билинейно по x, z в двух слоях узлов вокруг неё, потом
    /// линейно по y. Те же узлы и тот же порядок арифметики, что в [`Caves::carve`],
    /// поэтому результат совпадает побитно.
    fn sample(&self, pos: IVec3) -> Sample {
        let cell = pos.div_euclid(IVec3::splat(CELL));
        let t = pos - cell * CELL;
        let layer = |dy| {
            let at = |dx, dz| self.node((cell + IVec3::new(dx, dy, dz)) * CELL);
            bilinear(
                [at(0, 0), at(1, 0), at(0, 1), at(1, 1)],
                fraction(t.x),
                fraction(t.z),
            )
        };
        lerp(layer(0), layer(1), fraction(t.y))
    }
}

fn local(pos: IVec3) -> IVec3 {
    IVec3::new(pos.x.rem_euclid(16), pos.y, pos.z.rem_euclid(16))
}

fn keys(center: ColumnPos, radius: i32) -> impl Iterator<Item = ColumnPos> {
    (-radius..=radius).flat_map(move |z| (-radius..=radius).map(move |x| center + IVec2::new(x, z)))
}

/// Колонка без деревьев до и после пещер.
fn ground_and_carved(terrain: &Terrain, key: ColumnPos) -> (Column, Column) {
    (terrain.generate_ground(key), terrain.generate_carved(key))
}

/// Мировые координаты блоков колонки `key`.
fn blocks(key: ColumnPos) -> impl Iterator<Item = IVec3> {
    let origin = key.origin();
    (0..WORLD_HEIGHT).flat_map(move |y| {
        (0..16).flat_map(move |z| (0..16).map(move |x| IVec3::new(origin.x + x, y, origin.z + z)))
    })
}

#[test]
fn same_seed_same_caves_other_seed_other() {
    let (a, b, other) = (Caves::new(42), Caves::new(42), Caves::new(43));
    let points: Vec<IVec3> = (0..4000)
        .map(|i| IVec3::new(i * 7 - 9000, 4 + i % 120, i * -13 + 500))
        .collect();
    assert!(points.iter().all(|&p| a.is_cave(p) == b.is_cave(p)));
    let differs = points
        .iter()
        .filter(|&&p| a.is_cave(p) != other.is_cave(p))
        .count();
    let caves = points.iter().filter(|&&p| a.is_cave(p)).count();
    assert!(caves > 100, "{caves} caves");
    assert!(differs > caves / 2, "{differs} of {caves}");

    let terrain = Terrain::new(42);
    for key in [ColumnPos::new(-3, 7), ColumnPos::new(100, -40)] {
        let (_, x) = ground_and_carved(&terrain, key);
        let (_, y) = ground_and_carved(&Terrain::new(42), key);
        assert!(blocks(key).all(|p| x.get(local(p)) == y.get(local(p))));
    }
}

#[test]
fn interpolation_hits_nodes_and_stays_between_them() {
    let caves = Caves::new(7);
    for i in -20..20 {
        let node = IVec3::new(i * 4, 8 + (i & 15) * 4, i * -12);
        assert_eq!(caves.sample(node), caves.node(node));
    }
    for i in 0..2000 {
        let pos = IVec3::new(i * 5 - 3000, i % 128, i * 3 - 77);
        let cell = pos.div_euclid(IVec3::splat(CELL));
        let corners: [Sample; 8] = std::array::from_fn(|c| {
            let offset = IVec3::new((c & 1) as i32, (c >> 1 & 1) as i32, (c >> 2) as i32);
            caves.node((cell + offset) * CELL)
        });
        let value = caves.sample(pos);
        for n in 0..3 {
            let (min, max) = corners.iter().fold((f32::MAX, f32::MIN), |(lo, hi), c| {
                (lo.min(c[n]), hi.max(c[n]))
            });
            assert!(
                (min - 1e-6..=max + 1e-6).contains(&value[n]),
                "{pos}: {value:?}"
            );
        }
    }
}

#[test]
fn carve_matches_direct_evaluation_across_borders() {
    // Сетка в колонке и прямое вычисление по мировым координатам должны совпасть
    // везде, в том числе в столбцах x = 15 и x = 0 соседних колонок.
    let terrain = Terrain::new(1337);
    let caves = &terrain.caves;
    let surface = |x, z| terrain.surface_height(x, z);
    let has_tree = |x, z| tree_at(&terrain, x, z).is_some();
    let mut carved_total = 0;
    for key in [
        ColumnPos::new(2, -5),
        ColumnPos::new(3, -5),
        ColumnPos::new(-1, 0),
        ColumnPos::new(0, 0),
    ] {
        let (ground, carved) = ground_and_carved(&terrain, key);
        for p in blocks(key) {
            let l = caves.limits(p.x, p.z, surface, has_tree);
            let expected = p.y > FLOOR
                && p.y <= l.top
                && carvable(ground.get(local(p)))
                && hollow(caves.sample(p), l.density(p.y));
            let got = carved.get(local(p)) != ground.get(local(p));
            assert_eq!(got, expected, "{p}");
            carved_total += usize::from(got);
        }
    }
    assert!(carved_total > 1000, "{carved_total} carved");
}

#[test]
fn carved_share_of_underground_is_moderate() {
    for seed in [1337, 42] {
        let terrain = Terrain::new(seed);
        let (mut solid, mut hollow) = (0, 0);
        for key in keys(ColumnPos::new(5, -3), 3) {
            let (ground, carved) = ground_and_carved(&terrain, key);
            for p in blocks(key).filter(|p| p.y > FLOOR) {
                let p = local(p);
                if carvable(ground.get(p)) {
                    solid += 1;
                    hollow += usize::from(carved.get(p) == Block::Air);
                }
            }
        }
        let share = hollow as f32 / solid as f32;
        assert!((0.05..=0.20).contains(&share), "seed {seed}: {share}");
    }
}

#[test]
fn bottom_layers_stay_stone() {
    let terrain = Terrain::new(1337);
    for key in keys(ColumnPos::new(-4, 9), 2) {
        let column = terrain.generate(key);
        for p in blocks(key).filter(|p| p.y <= FLOOR) {
            assert_eq!(column.get(local(p)), Block::Stone, "{p}");
        }
    }
}

#[test]
fn no_air_under_water() {
    let terrain = Terrain::new(1337);
    let lake = lake_key(&terrain);
    let mut water = 0;
    for key in keys(lake, 2) {
        let column = terrain.generate(key);
        for p in blocks(key).map(local) {
            if column.get(p) != Block::Water {
                continue;
            }
            water += 1;
            // Дно и ещё WET_ROOF блоков под ним целы.
            let floor = (0..p.y)
                .rev()
                .find(|&y| column.get(p.with_y(y)) != Block::Water);
            let floor = floor.expect("water down to the bottom");
            for y in floor - WET_ROOF..=floor {
                assert!(column.get(p.with_y(y)).is_solid(), "{key} {p} y {y}");
            }
        }
    }
    assert!(water > 1000, "{water} water blocks");
}

#[test]
fn trees_stand_on_ground() {
    let terrain = Terrain::new(1337);
    let mut trees = 0;
    for key in keys(ColumnPos::new(0, 0), 6) {
        let column = terrain.generate(key);
        let origin = key.origin();
        for z in origin.z..origin.z + 16 {
            for x in origin.x..origin.x + 16 {
                let Some(tree) = tree_at(&terrain, x, z) else {
                    continue;
                };
                trees += 1;
                for dy in 1..=ROOF {
                    let p = local(tree.base - IVec3::Y * dy);
                    assert!(column.get(p).is_solid(), "tree at {x}, {z}, dy {dy}");
                }
            }
        }
    }
    assert!(trees > 10, "{trees} trees");
}

#[test]
fn tree_closes_entrance_roof() {
    let terrain = Terrain::new(1337);
    let caves = &terrain.caves;
    let (x, z) = (0..100_000)
        .map(|i| (i * 7, i * -3))
        .find(|&(x, z)| {
            caves.entrance(x, z) > 0.5 && terrain.surface_height(x, z) > WATER_LEVEL + 8
        })
        .expect("no dry entrance");
    let surface = |x, z| terrain.surface_height(x, z);
    let h = surface(x, z);
    assert_eq!(caves.limits(x, z, surface, |_, _| false).top, h);
    assert_eq!(caves.limits(x, z, surface, |_, _| true).top, h - ROOF);
}

#[test]
fn roof_is_opened_only_in_entrances() {
    let terrain = Terrain::new(1337);
    let caves = &terrain.caves;
    let mut openings = 0;
    for key in keys(ColumnPos::new(0, 0), 8) {
        let (ground, carved) = ground_and_carved(&terrain, key);
        let origin = key.origin();
        for z in origin.z..origin.z + 16 {
            for x in origin.x..origin.x + 16 {
                let h = terrain.surface_height(x, z);
                let opened = (h - ROOF + 1..=h).any(|y| {
                    let p = local(IVec3::new(x, y, z));
                    carved.get(p) != ground.get(p)
                });
                if opened {
                    assert!(caves.entrance(x, z) > 0.0, "{x}, {z}");
                    openings += 1;
                }
            }
        }
    }
    // Входы редкие, но на 17×17 колонок встречаются.
    assert!(openings > 0, "no entrances");
    assert!(openings < 17 * 17 * 256 / 50, "{openings} opened columns");
}
