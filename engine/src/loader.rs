use std::collections::HashSet;

use glam::IVec2;

use crate::column::Column;
use crate::world::World;

/// Пределы дальности прорисовки, колонок. Фронт (`src/settings.ts`) держит те же числа.
pub const MIN_RADIUS: i32 = 4;
pub const MAX_RADIUS: i32 = 16;
pub const DEFAULT_RADIUS: i32 = 8;

/// Соседи по граням: мешеру нужны только они, диагональные колонки он не трогает.
const NEIGHBORS: [IVec2; 4] = [IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y];

/// Результат одного [`Loader::advance`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Progress {
    /// Колонка сгенерирована и лежит в `World`.
    Generated(IVec2),
    /// Колонке нужно построить меш. Загрузчик уже считает её мешированной.
    Mesh(IVec2),
    /// Всё в радиусе загружено и мешировано.
    Done,
}

/// Решает, какие колонки генерировать, мешировать и выгружать вокруг игрока.
/// Без GPU и без часов: данные колонок лежат в [`World`], меши — у вызывающего,
/// и сколько шагов [`Loader::advance`] делать за кадр, решает он же.
///
/// Расстояние везде евклидово между ключами колонок. От дальности прорисовки
/// `radius` зависят остальные радиусы:
/// - данные грузим до `radius + 1`: колонке на краю нужны соседи для меша;
/// - меш удаляем дальше `radius + 1`, данные — дальше `radius + 2`: на границе колонки
///   не мигают туда-обратно при шаге игрока, и у колонки с мешем (≤ radius + 1) соседи
///   (≤ radius + 2 по неравенству треугольника) никогда не выгружаются.
///
/// После уменьшения радиуса запас на гистерезис не нужен: первый `update` снимает
/// меши дальше `radius`, а данные дальше `radius + 1`. Инвариант про соседей держится
/// и тут — меши снимаются раньше данных в том же `update`.
pub struct Loader {
    center: IVec2,
    radius: i32,
    /// Радиус уменьшился, и следующий `update` обрезает круги без запаса.
    shrunk: bool,
    /// Смещения в круге `gen_radius`, ближние первыми.
    offsets: Vec<IVec2>,
    meshed: HashSet<IVec2>,
    /// Индексы в `offsets`, до которых всё уже мешировано и загружено. Между вызовами
    /// `update` колонки только прибавляются, так что `advance` не просматривает
    /// готовое начало круга заново.
    meshed_prefix: usize,
    loaded_prefix: usize,
}

impl Loader {
    /// `center` — ключ колонки, в которой стоит игрок; дальность — [`DEFAULT_RADIUS`].
    pub fn new(center: IVec2) -> Self {
        let mut loader = Self {
            center,
            radius: DEFAULT_RADIUS,
            shrunk: false,
            offsets: Vec::new(),
            meshed: HashSet::new(),
            meshed_prefix: 0,
            loaded_prefix: 0,
        };
        loader.offsets = circle_offsets(loader.gen_radius());
        loader
    }

    pub fn radius(&self) -> i32 {
        self.radius
    }

    /// Меняет дальность прорисовки, зажимая её в [`MIN_RADIUS`]..=[`MAX_RADIUS`].
    /// Лишние меши и данные снимет ближайший [`Loader::update`].
    pub fn set_radius(&mut self, radius: i32) {
        let radius = radius.clamp(MIN_RADIUS, MAX_RADIUS);
        if radius != self.radius {
            self.shrunk |= radius < self.radius;
            self.radius = radius;
            self.offsets = circle_offsets(self.gen_radius());
            self.reset_prefixes();
        }
    }

    fn gen_radius(&self) -> i32 {
        self.radius + 1
    }

    fn unmesh_radius(&self) -> i32 {
        self.radius + 1
    }

    fn unload_radius(&self) -> i32 {
        self.radius + 2
    }

    /// Синхронно грузит круг `radius` (и кольцо соседей за ним) и возвращает все
    /// колонки круга для меширования — чтобы первый кадр не был пустым.
    pub fn warm_up(
        &mut self,
        world: &mut World,
        mut generate: impl FnMut(IVec2) -> Column,
        radius: i32,
    ) -> Vec<IVec2> {
        assert!(
            radius <= self.radius,
            "warm-up radius {radius} exceeds {}",
            self.radius
        );
        let center = self.center;
        let in_circle = |r: i32| move |o: &&IVec2| o.length_squared() <= r * r;
        for &o in self.offsets.iter().filter(in_circle(radius + 1)) {
            let key = center + o;
            if world.column(key).is_none() {
                world.insert(key, generate(key));
            }
        }
        let mesh: Vec<IVec2> = self
            .offsets
            .iter()
            .filter(in_circle(radius))
            .map(|&o| center + o)
            .filter(|key| !self.meshed.contains(key))
            .collect();
        self.meshed.extend(&mesh);
        mesh
    }

    /// Раз в кадр: переносит центр, снимает меши и выгружает данные далёких колонок.
    /// Возвращает колонки, чьи меши пора удалить.
    pub fn update(&mut self, center: IVec2, world: &mut World) -> Vec<IVec2> {
        self.center = center;
        self.reset_prefixes();
        let dist2 = |key: IVec2| (key - center).length_squared();

        // Меши снимаются раньше данных и с меньшим радиусом — на этом держится
        // инвариант про соседей мешированной колонки.
        let shrunk = std::mem::take(&mut self.shrunk);
        let unmesh_radius = self.unmesh_radius() - i32::from(shrunk);
        let mut unmesh = Vec::new();
        self.meshed.retain(|&key| {
            let keep = dist2(key) <= unmesh_radius * unmesh_radius;
            if !keep {
                unmesh.push(key);
            }
            keep
        });
        let unload_radius = self.unload_radius() - i32::from(shrunk);
        let far: Vec<IVec2> = world
            .keys()
            .filter(|&key| dist2(key) > unload_radius * unload_radius)
            .collect();
        for key in far {
            world.remove(key);
        }
        unmesh
    }

    /// Одна единица работы: меш ближайшей готовой колонки, а если таких нет —
    /// генерация ближайшей незагруженной. Меши идут первыми, чтобы рельеф вокруг
    /// игрока появлялся как можно раньше; генерация при этом не голодает, ведь
    /// без неё готовые к мешу колонки быстро кончаются.
    pub fn advance(
        &mut self,
        world: &mut World,
        generate: impl FnOnce(IVec2) -> Column,
    ) -> Progress {
        if let Some(key) = self.next_to_mesh(world) {
            self.meshed.insert(key);
            return Progress::Mesh(key);
        }
        if let Some(key) = self.next_to_generate(world) {
            world.insert(key, generate(key));
            return Progress::Generated(key);
        }
        Progress::Done
    }

    /// Колонок в радиусе прорисовки, которые ещё ждут меша.
    pub fn pending(&self) -> usize {
        self.offsets[..self.circle_len(self.radius)]
            .iter()
            .filter(|&&o| !self.meshed.contains(&(self.center + o)))
            .count()
    }

    fn next_to_mesh(&mut self, world: &World) -> Option<IVec2> {
        let center = self.center;
        let circle = &self.offsets[..self.circle_len(self.radius)];
        while circle
            .get(self.meshed_prefix)
            .is_some_and(|&o| self.meshed.contains(&(center + o)))
        {
            self.meshed_prefix += 1;
        }
        let loaded = |key: IVec2| world.column(key).is_some();
        circle[self.meshed_prefix..]
            .iter()
            .map(|&o| center + o)
            .filter(|key| !self.meshed.contains(key))
            .find(|&key| loaded(key) && NEIGHBORS.iter().all(|&n| loaded(key + n)))
    }

    fn next_to_generate(&mut self, world: &World) -> Option<IVec2> {
        let center = self.center;
        let missing = self.offsets[self.loaded_prefix..]
            .iter()
            .position(|&o| world.column(center + o).is_none());
        match missing {
            Some(i) => {
                self.loaded_prefix += i;
                Some(center + self.offsets[self.loaded_prefix])
            }
            None => {
                self.loaded_prefix = self.offsets.len();
                None
            }
        }
    }

    fn reset_prefixes(&mut self) {
        self.meshed_prefix = 0;
        self.loaded_prefix = 0;
    }

    /// Смещения отсортированы по расстоянию, так что круг `radius` — их начало этой длины.
    fn circle_len(&self, radius: i32) -> usize {
        self.offsets
            .partition_point(|o| o.length_squared() <= radius * radius)
    }
}

/// Смещения в круге `radius`, ближние первыми.
fn circle_offsets(radius: i32) -> Vec<IVec2> {
    let r = radius;
    let mut offsets: Vec<IVec2> = (-r..=r)
        .flat_map(|z| (-r..=r).map(move |x| IVec2::new(x, z)))
        .filter(|o| o.length_squared() <= r * r)
        .collect();
    // Вторичный ключ — ради одинакового порядка на любой платформе.
    offsets.sort_by_key(|o| (o.length_squared(), o.y, o.x));
    offsets
}

#[cfg(test)]
mod tests {
    use super::*;

    fn air(_: IVec2) -> Column {
        Column::new()
    }

    fn circle_points(r: i32) -> usize {
        circle_offsets(r).len()
    }

    fn dist2(a: IVec2, b: IVec2) -> i32 {
        (a - b).length_squared()
    }

    /// Один кадр движка: `update`, затем до `steps` шагов `advance`.
    /// Проверяет, что снимаются только мешированные колонки, и ведёт их учёт.
    fn frame(
        loader: &mut Loader,
        world: &mut World,
        center: IVec2,
        meshed: &mut HashSet<IVec2>,
        steps: usize,
    ) -> Vec<Progress> {
        for key in loader.update(center, world) {
            assert!(meshed.remove(&key), "unmesh of {key} without mesh");
        }
        let mut done = Vec::new();
        for _ in 0..steps {
            let progress = loader.advance(world, air);
            if let Progress::Mesh(key) = progress {
                assert!(meshed.insert(key), "{key} meshed twice");
            }
            if progress == Progress::Done {
                break;
            }
            done.push(progress);
        }
        done
    }

    /// Гоняет загрузчик в `center`, пока он не закончит всю работу.
    fn settle(loader: &mut Loader, world: &mut World, center: IVec2, meshed: &mut HashSet<IVec2>) {
        while !frame(loader, world, center, meshed, usize::MAX).is_empty() {}
    }

    /// Загружен ровно круг `radius + 1`, мешируется ровно круг `radius`.
    fn assert_circles(world: &World, meshed: &HashSet<IVec2>, center: IVec2, radius: i32) {
        let r = radius + 1;
        for z in -r - 2..=r + 2 {
            for x in -r - 2..=r + 2 {
                let key = center + IVec2::new(x, z);
                let d2 = dist2(key, center);
                assert_eq!(world.column(key).is_some(), d2 <= r * r, "{key}");
                assert_eq!(meshed.contains(&key), d2 <= radius * radius, "{key}");
            }
        }
    }

    fn assert_neighbors_loaded(world: &World, meshed: &HashSet<IVec2>) {
        for &key in meshed {
            assert!(world.column(key).is_some(), "meshed {key} unloaded");
            for n in NEIGHBORS {
                assert!(
                    world.column(key + n).is_some(),
                    "neighbor of {key} unloaded"
                );
            }
        }
    }

    #[test]
    fn loads_and_meshes_circles() {
        let center = IVec2::new(-5, 3);
        let mut loader = Loader::new(center);
        let mut world = World::new();
        let mut meshed = HashSet::new();
        settle(&mut loader, &mut world, center, &mut meshed);
        assert_circles(&world, &meshed, center, DEFAULT_RADIUS);
        assert_eq!(loader.pending(), 0);
    }

    #[test]
    fn generates_nearest_first_and_meshes_when_neighbors_ready() {
        let mut loader = Loader::new(IVec2::ZERO);
        let mut world = World::new();
        let mut meshed = HashSet::new();
        // Центр и 4 соседа — ровно круг радиуса 1, после них центр готов к мешу.
        let first = frame(&mut loader, &mut world, IVec2::ZERO, &mut meshed, 6);
        assert_eq!(first[0], Progress::Generated(IVec2::ZERO));
        assert!(first[..5].iter().all(|p| matches!(
            p,
            Progress::Generated(key) if key.length_squared() <= 1
        )));
        assert_eq!(first[5], Progress::Mesh(IVec2::ZERO));

        let mut generated = Vec::new();
        while let progress @ [_, ..] =
            &frame(&mut loader, &mut world, IVec2::ZERO, &mut meshed, 7)[..]
        {
            assert!(progress.len() <= 7);
            generated.extend(progress.iter().filter_map(|p| match p {
                Progress::Generated(key) => Some(key.length_squared()),
                _ => None,
            }));
        }
        assert!(generated.is_sorted(), "{generated:?}");
        assert_eq!(loader.pending(), 0);
    }

    #[test]
    fn column_without_all_neighbors_is_not_meshed() {
        let mut loader = Loader::new(IVec2::ZERO);
        let mut world = World::new();
        for key in [IVec2::ZERO, IVec2::X, IVec2::NEG_X, IVec2::Y] {
            world.insert(key, Column::new());
        }
        loader.update(IVec2::ZERO, &mut world);
        assert_eq!(
            loader.advance(&mut world, air),
            Progress::Generated(IVec2::NEG_Y)
        );
        assert_eq!(loader.advance(&mut world, air), Progress::Mesh(IVec2::ZERO));
    }

    #[test]
    fn unloads_only_beyond_margin() {
        let mut loader = Loader::new(IVec2::ZERO);
        let mut world = World::new();
        let mut meshed = HashSet::new();
        settle(&mut loader, &mut world, IVec2::ZERO, &mut meshed);
        let before: HashSet<IVec2> = world.keys().collect();

        // Сдвиг на 2 колонки: старые данные дальше radius + 1 от нового центра,
        // но не дальше radius + 2, должны остаться.
        let center = IVec2::new(2, 0);
        let unload = loader.unload_radius();
        for key in loader.update(center, &mut world) {
            assert!(dist2(key, center) > loader.unmesh_radius().pow(2));
            assert!(meshed.remove(&key));
        }
        for key in &before {
            let d2 = dist2(*key, center);
            assert_eq!(world.column(*key).is_some(), d2 <= unload * unload);
        }
        assert!(
            before.iter().any(
                |&k| dist2(k, center) > loader.gen_radius().pow(2) && world.column(k).is_some()
            )
        );

        // Далёкий прыжок: всё старое выгружается, загруженное ограничено кругом.
        let far = IVec2::new(1000, -1000);
        settle(&mut loader, &mut world, far, &mut meshed);
        assert_circles(&world, &meshed, far, DEFAULT_RADIUS);
    }

    #[test]
    fn meshed_columns_keep_their_neighbors() {
        let mut loader = Loader::new(IVec2::ZERO);
        let mut world = World::new();
        let mut meshed: HashSet<IVec2> = HashSet::new();
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
            frame(&mut loader, &mut world, center, &mut meshed, 6);
            assert_neighbors_loaded(&world, &meshed);
            assert!(world.len() <= circle_points(loader.unload_radius()));
        }
    }

    #[test]
    fn set_radius_up_and_down_settles_to_new_circles() {
        let center = IVec2::new(3, -2);
        let mut loader = Loader::new(center);
        let mut world = World::new();
        let mut meshed = HashSet::new();
        settle(&mut loader, &mut world, center, &mut meshed);
        for radius in [12, 5, 16, 4, 9] {
            loader.set_radius(radius);
            assert_eq!(loader.radius(), radius);
            settle(&mut loader, &mut world, center, &mut meshed);
            assert_circles(&world, &meshed, center, radius);
            assert_eq!(loader.pending(), 0);
        }
    }

    #[test]
    fn sharp_shrink_keeps_neighbors_of_meshed_columns() {
        let mut center = IVec2::ZERO;
        let mut loader = Loader::new(center);
        let mut world = World::new();
        let mut meshed = HashSet::new();
        loader.set_radius(MAX_RADIUS);
        settle(&mut loader, &mut world, center, &mut meshed);

        loader.set_radius(MIN_RADIUS);
        for i in 0..60 {
            if i % 3 == 0 {
                center += IVec2::new(1, 1);
            }
            for key in loader.update(center, &mut world) {
                assert!(meshed.remove(&key));
            }
            assert_neighbors_loaded(&world, &meshed);
            for _ in 0..5 {
                if let Progress::Mesh(key) = loader.advance(&mut world, air) {
                    meshed.insert(key);
                }
                assert_neighbors_loaded(&world, &meshed);
            }
        }
        // После движения на краю остаётся запас гистерезиса, но не больше него.
        settle(&mut loader, &mut world, center, &mut meshed);
        assert_eq!(loader.pending(), 0);
        let r = MIN_RADIUS;
        assert!(meshed.iter().all(|&k| dist2(k, center) <= (r + 1).pow(2)));
        assert!(world.keys().all(|k| dist2(k, center) <= (r + 2).pow(2)));
    }

    #[test]
    fn radius_is_clamped() {
        let mut loader = Loader::new(IVec2::ZERO);
        assert_eq!(loader.radius(), DEFAULT_RADIUS);
        loader.set_radius(1);
        assert_eq!(loader.radius(), MIN_RADIUS);
        loader.set_radius(100);
        assert_eq!(loader.radius(), MAX_RADIUS);
    }

    #[test]
    fn warm_up_meshes_small_circle() {
        let center = IVec2::new(7, 7);
        let mut loader = Loader::new(center);
        let mut world = World::new();
        let mesh = loader.warm_up(&mut world, air, 2);
        assert_eq!(mesh.len(), circle_points(2));
        assert_eq!(world.len(), circle_points(3));
        assert!(mesh.iter().all(|&k| dist2(k, center) <= 4));
    }
}
