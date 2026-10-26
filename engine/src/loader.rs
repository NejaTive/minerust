use std::collections::HashSet;

use glam::IVec2;

use crate::column::Column;
use crate::world::World;

/// Дальность прорисовки, колонок. Расстояние везде евклидово между ключами колонок.
pub const RADIUS: i32 = 8;
/// Данные грузим на колонку дальше прорисовки: колонке на краю нужны соседи для меша.
const GEN_RADIUS: i32 = RADIUS + 1;
/// Меш удаляем за RADIUS + 1, данные — за RADIUS + 2: на границе колонки не мигают
/// туда-обратно при шаге игрока, и у колонки с мешем (≤ RADIUS + 1) соседи
/// (≤ RADIUS + 2 по неравенству треугольника) никогда не выгружаются.
const UNMESH_RADIUS: i32 = RADIUS + 1;
const UNLOAD_RADIUS: i32 = RADIUS + 2;

/// Соседи по граням: мешеру нужны только они, диагональные колонки он не трогает.
const NEIGHBORS: [IVec2; 4] = [IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y];

/// Сколько работы разрешено за один вызов [`Loader::update`].
#[derive(Clone, Copy)]
pub struct Budget {
    /// Генераций колонок.
    pub generate: usize,
    /// Колонок, отданных на меширование.
    pub mesh: usize,
}

/// Что изменилось в мешах за шаг; строить и удалять их — дело вызывающего.
#[derive(Default)]
pub struct Changes {
    /// Колонки, которым нужно построить меш. Загрузчик уже считает их мешированными.
    pub mesh: Vec<IVec2>,
    /// Колонки, чьи меши пора удалить.
    pub unmesh: Vec<IVec2>,
}

/// Решает, какие колонки генерировать, мешировать и выгружать вокруг игрока.
/// Без GPU: данные колонок лежат в [`World`], меши — у вызывающего.
pub struct Loader {
    center: IVec2,
    /// Смещения в круге GEN_RADIUS, ближние первыми.
    offsets: Vec<IVec2>,
    meshed: HashSet<IVec2>,
}

impl Loader {
    /// `center` — ключ колонки, в которой стоит игрок.
    pub fn new(center: IVec2) -> Self {
        let r = GEN_RADIUS;
        let mut offsets: Vec<IVec2> = (-r..=r)
            .flat_map(|z| (-r..=r).map(move |x| IVec2::new(x, z)))
            .filter(|o| o.length_squared() <= r * r)
            .collect();
        // Вторичный ключ — ради одинакового порядка на любой платформе.
        offsets.sort_by_key(|o| (o.length_squared(), o.y, o.x));
        Self {
            center,
            offsets,
            meshed: HashSet::new(),
        }
    }

    /// Синхронно грузит круг `radius` (и кольцо соседей за ним) и возвращает все
    /// колонки круга для меширования — чтобы первый кадр не был пустым.
    pub fn warm_up(
        &mut self,
        world: &mut World,
        generate: impl FnMut(IVec2) -> Column,
        radius: i32,
    ) -> Vec<IVec2> {
        assert!(radius <= RADIUS, "warm-up radius {radius} exceeds RADIUS");
        // Смещения отсортированы по расстоянию, так что первые `circle_len(r)` из них — ровно круг r.
        let budget = Budget {
            generate: self.circle_len(radius + 1),
            mesh: usize::MAX,
        };
        self.step(world, generate, budget, radius).mesh
    }

    /// Шаг на кадр: снимает меши и выгружает данные далёких колонок, генерирует
    /// ближние незагруженные и выбирает, какие мешировать, — в пределах `budget`.
    pub fn update(
        &mut self,
        center: IVec2,
        world: &mut World,
        generate: impl FnMut(IVec2) -> Column,
        budget: Budget,
    ) -> Changes {
        self.center = center;
        self.step(world, generate, budget, RADIUS)
    }

    /// Колонок в радиусе прорисовки, которые ещё ждут меша.
    pub fn pending(&self) -> usize {
        self.offsets
            .iter()
            .filter(|o| o.length_squared() <= RADIUS * RADIUS)
            .filter(|&&o| !self.meshed.contains(&(self.center + o)))
            .count()
    }

    fn step(
        &mut self,
        world: &mut World,
        mut generate: impl FnMut(IVec2) -> Column,
        budget: Budget,
        mesh_radius: i32,
    ) -> Changes {
        let mut changes = Changes::default();
        let center = self.center;
        let dist2 = |key: IVec2| (key - center).length_squared();

        // Меши снимаются раньше данных и с меньшим радиусом — на этом держится
        // инвариант про соседей мешированной колонки.
        self.meshed.retain(|&key| {
            let keep = dist2(key) <= UNMESH_RADIUS * UNMESH_RADIUS;
            if !keep {
                changes.unmesh.push(key);
            }
            keep
        });
        let far: Vec<IVec2> = world
            .keys()
            .filter(|&key| dist2(key) > UNLOAD_RADIUS * UNLOAD_RADIUS)
            .collect();
        for key in far {
            world.remove(key);
        }

        let missing: Vec<IVec2> = self
            .offsets
            .iter()
            .map(|&o| center + o)
            .filter(|&key| world.column(key).is_none())
            .take(budget.generate)
            .collect();
        for key in missing {
            world.insert(key, generate(key));
        }

        let loaded = |key: IVec2| world.column(key).is_some();
        changes.mesh = self
            .offsets
            .iter()
            .filter(|o| o.length_squared() <= mesh_radius * mesh_radius)
            .map(|&o| center + o)
            .filter(|key| !self.meshed.contains(key))
            .filter(|&key| loaded(key) && NEIGHBORS.iter().all(|&n| loaded(key + n)))
            .take(budget.mesh)
            .collect();
        self.meshed.extend(&changes.mesh);
        changes
    }

    fn circle_len(&self, radius: i32) -> usize {
        self.offsets
            .iter()
            .filter(|o| o.length_squared() <= radius * radius)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNLIMITED: Budget = Budget {
        generate: usize::MAX,
        mesh: usize::MAX,
    };

    fn air(_: IVec2) -> Column {
        Column::new()
    }

    fn circle_points(r: i32) -> usize {
        (-r..=r)
            .flat_map(|z| (-r..=r).map(move |x| IVec2::new(x, z)))
            .filter(|o| o.length_squared() <= r * r)
            .count()
    }

    fn dist2(a: IVec2, b: IVec2) -> i32 {
        (a - b).length_squared()
    }

    /// Гоняет загрузчик в `center`, пока он не перестанет что-то делать.
    fn settle(loader: &mut Loader, world: &mut World, center: IVec2, meshed: &mut HashSet<IVec2>) {
        loop {
            let changes = loader.update(center, world, air, UNLIMITED);
            for key in &changes.unmesh {
                assert!(meshed.remove(key));
            }
            if changes.mesh.is_empty() && changes.unmesh.is_empty() {
                return;
            }
            meshed.extend(changes.mesh);
        }
    }

    #[test]
    fn loads_and_meshes_circles() {
        let center = IVec2::new(-5, 3);
        let mut loader = Loader::new(center);
        let mut world = World::new();
        let mut meshed = HashSet::new();
        settle(&mut loader, &mut world, center, &mut meshed);

        let r = GEN_RADIUS;
        for z in -r - 1..=r + 1 {
            for x in -r - 1..=r + 1 {
                let key = center + IVec2::new(x, z);
                let d2 = dist2(key, center);
                assert_eq!(world.column(key).is_some(), d2 <= r * r, "{key}");
                assert_eq!(meshed.contains(&key), d2 <= RADIUS * RADIUS, "{key}");
            }
        }
        assert_eq!(loader.pending(), 0);
    }

    #[test]
    fn nearest_first_within_budget() {
        let mut loader = Loader::new(IVec2::ZERO);
        let mut world = World::new();
        let mut generated = Vec::new();
        let budget = Budget {
            generate: 5,
            mesh: 2,
        };
        let changes = loader.update(
            IVec2::ZERO,
            &mut world,
            |key| {
                generated.push(key);
                Column::new()
            },
            budget,
        );
        // Центр и 4 соседа — ровно круг радиуса 1, но меш строить ещё нельзя:
        // у соседей нет своих соседей.
        assert_eq!(generated.len(), 5);
        assert_eq!(generated[0], IVec2::ZERO);
        assert!(generated.iter().all(|&k| k.length_squared() <= 1));
        assert_eq!(changes.mesh, vec![IVec2::ZERO]);

        for _ in 0..200 {
            let changes = loader.update(IVec2::ZERO, &mut world, air, budget);
            assert!(changes.mesh.len() <= budget.mesh);
            let d2: Vec<i32> = changes.mesh.iter().map(|k| k.length_squared()).collect();
            assert!(d2.is_sorted(), "{:?}", changes.mesh);
        }
        assert_eq!(loader.pending(), 0);
    }

    #[test]
    fn generation_respects_budget() {
        let mut loader = Loader::new(IVec2::ZERO);
        let mut world = World::new();
        let budget = Budget {
            generate: 3,
            mesh: 1,
        };
        let mut previous = 0;
        for _ in 0..10 {
            loader.update(IVec2::ZERO, &mut world, air, budget);
            assert_eq!(world.len(), previous + 3);
            previous = world.len();
        }
    }

    #[test]
    fn column_without_all_neighbors_is_not_meshed() {
        let mut loader = Loader::new(IVec2::ZERO);
        let mut world = World::new();
        for key in [IVec2::ZERO, IVec2::X, IVec2::NEG_X, IVec2::Y] {
            world.insert(key, Column::new());
        }
        let no_generation = Budget {
            generate: 0,
            mesh: usize::MAX,
        };
        let changes = loader.update(IVec2::ZERO, &mut world, air, no_generation);
        assert!(changes.mesh.is_empty());

        world.insert(IVec2::NEG_Y, Column::new());
        let changes = loader.update(IVec2::ZERO, &mut world, air, no_generation);
        assert_eq!(changes.mesh, vec![IVec2::ZERO]);
    }

    #[test]
    fn unloads_only_beyond_margin() {
        let mut loader = Loader::new(IVec2::ZERO);
        let mut world = World::new();
        let mut meshed = HashSet::new();
        settle(&mut loader, &mut world, IVec2::ZERO, &mut meshed);
        let before: HashSet<IVec2> = world.keys().collect();

        // Сдвиг на 2 колонки: старые данные дальше RADIUS + 1 от нового центра,
        // но не дальше RADIUS + 2, должны остаться.
        let center = IVec2::new(2, 0);
        let changes = loader.update(center, &mut world, air, UNLIMITED);
        for key in &before {
            let d2 = dist2(*key, center);
            assert_eq!(
                world.column(*key).is_some(),
                d2 <= UNLOAD_RADIUS * UNLOAD_RADIUS
            );
        }
        assert!(
            before
                .iter()
                .any(|&k| dist2(k, center) > GEN_RADIUS * GEN_RADIUS && world.column(k).is_some())
        );
        for key in &changes.unmesh {
            assert!(dist2(*key, center) > UNMESH_RADIUS * UNMESH_RADIUS);
            assert!(meshed.remove(key));
        }
        meshed.extend(changes.mesh);

        // Далёкий прыжок: всё старое выгружается, загруженное ограничено кругом.
        let far = IVec2::new(1000, -1000);
        settle(&mut loader, &mut world, far, &mut meshed);
        assert!(
            world
                .keys()
                .all(|k| dist2(k, far) <= GEN_RADIUS * GEN_RADIUS)
        );
        assert_eq!(world.len(), loader.circle_len(GEN_RADIUS));
    }

    #[test]
    fn meshed_columns_keep_their_neighbors() {
        let mut loader = Loader::new(IVec2::ZERO);
        let mut world = World::new();
        let mut meshed: HashSet<IVec2> = HashSet::new();
        let budget = Budget {
            generate: 4,
            mesh: 2,
        };
        // Петляющий путь с шагами разной длины, включая прыжки через несколько колонок.
        let mut center = IVec2::ZERO;
        for i in 0..400 {
            let step = match i % 7 {
                0 => IVec2::new(3, 1),
                1 | 2 => IVec2::new(1, 0),
                3 => IVec2::new(0, -2),
                4 => IVec2::new(-1, 1),
                _ => IVec2::ZERO,
            };
            if i % 100 < 50 {
                center += step;
            } else {
                center -= step;
            }
            let changes = loader.update(center, &mut world, air, budget);
            for key in &changes.unmesh {
                assert!(meshed.remove(key));
            }
            meshed.extend(changes.mesh);
            for &key in &meshed {
                assert!(world.column(key).is_some(), "meshed {key} unloaded");
                for n in NEIGHBORS {
                    assert!(
                        world.column(key + n).is_some(),
                        "neighbor of {key} unloaded"
                    );
                }
            }
            assert!(world.len() <= circle_points(UNLOAD_RADIUS));
        }
    }

    #[test]
    fn warm_up_meshes_small_circle() {
        let mut loader = Loader::new(IVec2::new(7, 7));
        let mut world = World::new();
        let mesh = loader.warm_up(&mut world, air, 2);
        assert_eq!(mesh.len(), loader.circle_len(2));
        assert_eq!(world.len(), loader.circle_len(3));
        assert!(mesh.iter().all(|&k| dist2(k, IVec2::new(7, 7)) <= 4));
    }
}
