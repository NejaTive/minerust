use glam::{IVec2, IVec3};

use crate::block::Block;
use crate::column::Column;
use crate::section::SECTION_SIZE;
use crate::terrain::{Biome, Terrain, WATER_LEVEL};

/// Наибольший радиус кроны по x и z. Колонка ищет стволы и в полосе такой ширины у
/// соседей: их кроны могут нависать над ней.
pub const CROWN_RADIUS: i32 = 2;
/// Стволы не ближе этого друг к другу по каждой из осей x, z (расстояние Чебышёва).
pub const MIN_SPACING: i32 = 3;
/// Доля столбцов — кандидатов на дерево. В лесу после прореживания по `MIN_SPACING`
/// остаётся ~8 деревьев на колонку, на равнине — одно на несколько колонок.
const FOREST_DENSITY: f32 = 0.04;
const PLAINS_DENSITY: f32 = 0.002;
const MIN_TRUNK: i32 = 4;
const MAX_TRUNK: i32 = 6;

// Соли хеша: у каждого решения о дереве свой независимый случайный поток.
const SALT_SPOT: u64 = 1;
const SALT_TRUNK: u64 = 2;
const SALT_CORNERS: u64 = 3;

/// Дерево как у дуба в Minecraft: ствол и крона из четырёх слоёв листвы.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Tree {
    /// Нижнее бревно, мировые координаты; под ним трава.
    pub base: IVec3,
    /// Брёвен в стволе, `MIN_TRUNK..=MAX_TRUNK`.
    pub height: i32,
    /// Случайные биты: какие углы кроны срезаны.
    corners: u32,
}

impl Tree {
    pub fn logs(&self) -> impl Iterator<Item = IVec3> + '_ {
        (0..self.height).map(|dy| self.base + IVec3::Y * dy)
    }

    /// Листва в мировых координатах. Два нижних слоя (от третьего бревна сверху) —
    /// радиус 2, два верхних — радиус 1, самый верхний над стволом. Углы слоёв срезаны
    /// случайно, у верхнего всегда: он крестом. Внутри кроны позиции совпадают со
    /// стволом — их не ставит [`plant`].
    pub fn leaves(&self) -> impl Iterator<Item = IVec3> + '_ {
        let top = self.base.y + self.height - 1;
        let layers: [(i32, i32); 4] = [(top - 2, 2), (top - 1, 2), (top, 1), (top + 1, 1)];
        layers
            .into_iter()
            .enumerate()
            .flat_map(move |(layer, (y, r))| {
                (-r..=r).flat_map(move |dz| (-r..=r).map(move |dx| (layer, y, r, dx, dz)))
            })
            .filter(|&(layer, _, r, dx, dz)| {
                let corner = dx.abs() == r && dz.abs() == r;
                if !corner {
                    return true;
                }
                let bit = layer * 4 + usize::from(dx > 0) * 2 + usize::from(dz > 0);
                layer != 3 && self.corners & (1 << bit) != 0
            })
            .map(|(_, y, _, dx, dz)| IVec3::new(self.base.x + dx, y, self.base.z + dz))
    }
}

/// Дерево со стволом в столбце (x, z), если оно там растёт. Зависит только от сида и
/// координат, поэтому любая колонка получает один и тот же ответ.
pub fn tree_at(terrain: &Terrain, x: i32, z: i32) -> Option<Tree> {
    let own = candidate(terrain, x, z)?;
    // Кандидат растёт, если среди кандидатов в квадрате 5×5 вокруг он первый по
    // приоритету: из двух близких деревьев каждое не может быть первым, так что
    // расстояние держится без общего состояния и порядка обхода.
    let reach = MIN_SPACING - 1;
    for dz in -reach..=reach {
        for dx in -reach..=reach {
            if (dx, dz) == (0, 0) {
                continue;
            }
            let (nx, nz) = (x + dx, z + dz);
            if candidate(terrain, nx, nz).is_some_and(|other| (other, nx, nz) < (own, x, z)) {
                return None;
            }
        }
    }
    let seed = terrain.seed();
    let trunk = hash(seed, x, z, SALT_TRUNK) % (MAX_TRUNK - MIN_TRUNK + 1) as u32;
    Some(Tree {
        base: IVec3::new(x, terrain.surface_height(x, z) + 1, z),
        height: MIN_TRUNK + trunk as i32,
        corners: hash(seed, x, z, SALT_CORNERS),
    })
}

/// Приоритет кандидата на дерево (меньше — главнее) или `None`, если в столбце дерева
/// быть не может: по хешу, потому что сверху песок (пустыня, пляж) или вода.
fn candidate(terrain: &Terrain, x: i32, z: i32) -> Option<u32> {
    let h = hash(terrain.seed(), x, z, SALT_SPOT);
    // 24 старших бита — равномерное число 0..1, точно представимое в f32.
    let roll = (h >> 8) as f32 / (1 << 24) as f32;
    // Шум биомов дороже хеша: отсекаем заведомо пустые столбцы до него.
    if roll >= FOREST_DENSITY.max(PLAINS_DENSITY) {
        return None;
    }
    let w = terrain.biome_weights(x, z);
    let density = w.forest * FOREST_DENSITY + w.plains * PLAINS_DENSITY;
    if w.dominant() == Biome::Desert || roll >= density {
        return None;
    }
    // Ниже этой высоты поверхность — пляж или дно.
    (terrain.surface(x, z, w) > WATER_LEVEL + 1).then_some(h)
}

/// Ставит в колонку `key` её часть деревьев: и своих, и соседских, чьи кроны
/// заходят в неё. Сначала все брёвна, потом листва только в воздух — так результат не
/// зависит от порядка деревьев, а листва не затирает ни брёвна, ни землю.
pub fn plant(terrain: &Terrain, key: IVec2, column: &mut Column) {
    let origin = key * SECTION_SIZE;
    let range = |start: i32| start - CROWN_RADIUS..start + SECTION_SIZE + CROWN_RADIUS;
    let trees: Vec<Tree> = range(origin.y)
        .flat_map(|z| range(origin.x).map(move |x| (x, z)))
        .filter_map(|(x, z)| tree_at(terrain, x, z))
        .collect();
    let origin = IVec3::new(origin.x, 0, origin.y);
    for log in trees.iter().flat_map(Tree::logs) {
        let local = log - origin;
        if Column::contains(local) {
            column.set(local, Block::Log);
        }
    }
    for leaf in trees.iter().flat_map(Tree::leaves) {
        let local = leaf - origin;
        if Column::contains(local) && column.get(local) == Block::Air {
            column.set(local, Block::Leaves);
        }
    }
}

/// Хеш (сид, x, z, соль) через splitmix64: детерминированный и без зависимостей.
fn hash(seed: i32, x: i32, z: i32, salt: u64) -> u32 {
    let point = (u64::from(x as u32) << 32) | u64::from(z as u32);
    let stream = splitmix64((u64::from(seed as u32) << 8) | salt);
    (splitmix64(point ^ stream) >> 32) as u32
}

fn splitmix64(mut v: u64) -> u64 {
    v = v.wrapping_add(0x9e37_79b9_7f4a_7c15);
    v = (v ^ (v >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    v = (v ^ (v >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    v ^ (v >> 31)
}

/// Дерево в лесу, крона которого заходит в соседнюю колонку по x: для тестов стыков.
#[cfg(test)]
pub(crate) fn border_tree(terrain: &Terrain) -> Tree {
    (0..4096)
        .flat_map(|z| (0..64).map(move |i| (i * SECTION_SIZE + i % 2 * 15, z)))
        .find_map(|(x, z)| tree_at(terrain, x, z))
        .expect("no tree on a column border")
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::column::WORLD_HEIGHT;

    #[test]
    fn same_seed_same_trees() {
        let (a, b) = (Terrain::new(42), Terrain::new(42));
        for z in 0..200 {
            for x in 0..200 {
                assert_eq!(tree_at(&a, x, z), tree_at(&b, x, z));
            }
        }
        let other = Terrain::new(43);
        let differs = (0..200)
            .flat_map(|z| (0..200).map(move |x| (x, z)))
            .filter(|&(x, z)| tree_at(&a, x, z) != tree_at(&other, x, z))
            .count();
        assert!(differs > 0);
    }

    #[test]
    fn trees_keep_their_distance_and_shape() {
        let terrain = Terrain::new(1337);
        let mut trunks = HashSet::new();
        for z in -512..512 {
            for x in -512..512 {
                if let Some(tree) = tree_at(&terrain, x, z) {
                    assert!((MIN_TRUNK..=MAX_TRUNK).contains(&tree.height));
                    assert_eq!(tree.base.y, terrain.surface_height(x, z) + 1);
                    let leaves: Vec<IVec3> = tree.leaves().collect();
                    let spread = leaves
                        .iter()
                        .map(|l| (l.x - x).abs().max((l.z - z).abs()))
                        .max();
                    assert_eq!(spread, Some(CROWN_RADIUS));
                    // Над стволом лист, крона выше третьего бревна сверху не опускается.
                    let top = tree.base.y + tree.height - 1;
                    assert!(leaves.contains(&IVec3::new(x, top + 1, z)));
                    assert!(leaves.iter().all(|l| (top - 2..=top + 1).contains(&l.y)));
                    trunks.insert((x, z));
                }
            }
        }
        assert!(trunks.len() > 500, "only {} trees", trunks.len());
        for &(x, z) in &trunks {
            for dz in -(MIN_SPACING - 1)..MIN_SPACING {
                for dx in -(MIN_SPACING - 1)..MIN_SPACING {
                    if (dx, dz) != (0, 0) {
                        assert!(!trunks.contains(&(x + dx, z + dz)), "{x}, {z} +{dx}, {dz}");
                    }
                }
            }
        }
    }

    #[test]
    fn no_trees_in_water_or_on_beaches() {
        let terrain = Terrain::new(1337);
        let mut low = 0;
        for z in -1024..1024 {
            for x in -1024..1024 {
                if terrain.surface_height(x, z) <= WATER_LEVEL + 1 {
                    low += 1;
                    assert_eq!(tree_at(&terrain, x, z), None, "{x}, {z}");
                }
            }
        }
        assert!(low > 10_000, "only {low} low columns");
    }

    #[test]
    fn forest_is_denser_than_plains_and_desert_is_empty() {
        // Площадь — только суша: в воде и на пляжах деревьев нет в любом биоме.
        let terrain = Terrain::new(1337);
        let mut area = [0; 3];
        let mut trees = [0; 3];
        for z in -1024..1024 {
            for x in -1024..1024 {
                if terrain.surface_height(x, z) <= WATER_LEVEL + 1 {
                    continue;
                }
                let biome = terrain.biome_at(x, z) as usize;
                area[biome] += 1;
                if tree_at(&terrain, x, z).is_some() {
                    trees[biome] += 1;
                }
            }
        }
        let [plains, desert, forest] = [0, 1, 2].map(|b| trees[b] as f32 / area[b] as f32);
        assert!(area.iter().all(|&a| a > 10_000), "{area:?}");
        assert_eq!(trees[Biome::Desert as usize], 0, "{desert}");
        assert!(forest > 0.02 && plains > 0.0005, "{forest} {plains}");
        assert!(forest > plains * 5.0, "{forest} {plains}");
    }

    #[test]
    fn crowns_cross_columns_seamlessly() {
        let terrain = Terrain::new(1337);
        let tree = border_tree(&terrain);
        let center = IVec2::new(
            tree.base.x.div_euclid(SECTION_SIZE),
            tree.base.z.div_euclid(SECTION_SIZE),
        );
        let leaf_columns: HashSet<_> = tree
            .leaves()
            .map(|l| (l.x.div_euclid(SECTION_SIZE), l.z.div_euclid(SECTION_SIZE)))
            .collect();
        assert!(leaf_columns.len() > 1, "{tree:?} does not cross a border");

        // «Окно» 3×3 колонок одним массивом: земля, затем деревья со стволами в окне
        // целиком, с записью за границы колонок.
        let size = 3 * SECTION_SIZE;
        let min = (center - IVec2::ONE) * SECTION_SIZE;
        let index = |p: IVec3| ((p.x - min.x) + size * (p.z - min.y) + size * size * p.y) as usize;
        let inside = |p: IVec3| {
            (min.x..min.x + size).contains(&p.x)
                && (min.y..min.y + size).contains(&p.z)
                && (0..WORLD_HEIGHT).contains(&p.y)
        };
        let mut window = vec![Block::Air; (size * size * WORLD_HEIGHT) as usize];
        for cz in -1..=1 {
            for cx in -1..=1 {
                let key = center + IVec2::new(cx, cz);
                let ground = terrain.generate_ground(key);
                for y in 0..WORLD_HEIGHT {
                    for z in 0..SECTION_SIZE {
                        for x in 0..SECTION_SIZE {
                            let local = IVec3::new(x, y, z);
                            let world = local + IVec3::new(key.x, 0, key.y) * SECTION_SIZE;
                            window[index(world)] = ground.get(local);
                        }
                    }
                }
            }
        }
        let trees: Vec<Tree> = (min.y..min.y + size)
            .flat_map(|z| (min.x..min.x + size).map(move |x| (x, z)))
            .filter_map(|(x, z)| tree_at(&terrain, x, z))
            .collect();
        for log in trees.iter().flat_map(Tree::logs) {
            if inside(log) {
                window[index(log)] = Block::Log;
            }
        }
        for leaf in trees.iter().flat_map(Tree::leaves) {
            if inside(leaf) && window[index(leaf)] == Block::Air {
                window[index(leaf)] = Block::Leaves;
            }
        }

        // Колонки по отдельности совпадают с окном везде, куда не дотягиваются деревья
        // из-за его края.
        let mut compared = 0;
        for cz in -1..=1 {
            for cx in -1..=1 {
                let key = center + IVec2::new(cx, cz);
                let column = terrain.generate(key);
                for y in 0..WORLD_HEIGHT {
                    for z in 0..SECTION_SIZE {
                        for x in 0..SECTION_SIZE {
                            let local = IVec3::new(x, y, z);
                            let world = local + IVec3::new(key.x, 0, key.y) * SECTION_SIZE;
                            let from_edge = [world.x - min.x, world.z - min.y]
                                .map(|c| c.min(size - 1 - c))
                                .into_iter()
                                .min()
                                .unwrap();
                            if from_edge >= CROWN_RADIUS {
                                assert_eq!(column.get(local), window[index(world)], "{world}");
                                compared += 1;
                            }
                        }
                    }
                }
            }
        }
        assert!(compared > (SECTION_SIZE * SECTION_SIZE * WORLD_HEIGHT * 4) as usize);
    }

    #[test]
    fn leaves_do_not_replace_logs_or_ground() {
        let terrain = Terrain::new(1337);
        let tree = border_tree(&terrain);
        let key = IVec2::new(
            tree.base.x.div_euclid(SECTION_SIZE),
            tree.base.z.div_euclid(SECTION_SIZE),
        );
        let ground = terrain.generate_ground(key);
        let column = terrain.generate(key);
        let origin = IVec3::new(key.x, 0, key.y) * SECTION_SIZE;
        for log in tree.logs() {
            assert_eq!(column.get(log - origin), Block::Log);
        }
        assert_eq!(column.get(tree.base - origin - IVec3::Y), Block::Grass);
        for y in 0..WORLD_HEIGHT {
            for z in 0..SECTION_SIZE {
                for x in 0..SECTION_SIZE {
                    let p = IVec3::new(x, y, z);
                    if ground.get(p) != Block::Air {
                        assert_eq!(column.get(p), ground.get(p), "{p}");
                    }
                }
            }
        }
    }
}
