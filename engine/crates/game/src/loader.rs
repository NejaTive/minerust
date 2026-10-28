use std::collections::HashSet;

use glam::IVec2;

use world::column::Column;
use world::world::World;

/// Пределы дальности прорисовки, колонок. Фронт (`src/settings.ts`) держит те же числа.
pub const MIN_RADIUS: i32 = 4;
pub const MAX_RADIUS: i32 = 16;
pub const DEFAULT_RADIUS: i32 = 8;

/// Соседи колонки по граням и диагоналям: мешер смотрит в них на стыках, а AO — и
/// через угол колонки.
pub const NEIGHBORS: [IVec2; 8] = [
    IVec2::new(1, 0),
    IVec2::new(-1, 0),
    IVec2::new(0, 1),
    IVec2::new(0, -1),
    IVec2::new(1, 1),
    IVec2::new(-1, 1),
    IVec2::new(1, -1),
    IVec2::new(-1, -1),
];

/// Результат одного [`Loader::advance`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Progress {
    /// Колонка загружена (сгенерирована или взята из отложенных правок) и лежит в `World`.
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
/// Расстояние везде евклидово между ключами колонок. Меш колонки строится, только когда
/// загружены все 8 её соседей ([`NEIGHBORS`]), диагональный — на расстоянии √2. От
/// дальности прорисовки `radius` зависят остальные радиусы:
/// - данные грузим до `radius + 2`: диагональный сосед колонки на краю круга лежит
///   не дальше `radius + √2`;
/// - меш удаляем дальше `radius + 1`, данные — дальше `radius + 3`: на границе колонки
///   не мигают туда-обратно при шаге игрока, и у колонки с мешем (≤ radius + 1) соседи
///   (≤ radius + 1 + √2 < radius + 3 по неравенству треугольника) никогда не выгружаются.
///
/// После уменьшения радиуса запас на гистерезис не нужен: первый `update` снимает
/// меши дальше `radius`, а данные дальше `radius + 2` (соседи ≤ radius + √2). Инвариант
/// про соседей держится и тут — меши снимаются раньше данных в том же `update`.
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
        self.radius + 2
    }

    fn unmesh_radius(&self) -> i32 {
        self.radius + 1
    }

    fn unload_radius(&self) -> i32 {
        self.radius + 3
    }

    /// Синхронно грузит круг `radius` (и соседей за ним до `radius + 2`) и возвращает все
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
        for &o in self.offsets.iter().filter(in_circle(radius + 2)) {
            let key = center + o;
            if world.column(key).is_none() {
                load(world, key, &mut generate);
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
    /// загрузка ближайшей незагруженной: колонка с правками берётся из отложенных
    /// в `World`, остальные генерируются `generate`. Меши идут первыми, чтобы рельеф вокруг
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
            load(world, key, generate);
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

/// Кладёт колонку в мир: изменённую — из отложенных правок, иначе — новую от генератора.
fn load(world: &mut World, key: IVec2, generate: impl FnOnce(IVec2) -> Column) {
    let column = world.take_edited(key).unwrap_or_else(|| generate(key));
    world.insert(key, column);
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

    /// Загружен ровно круг `radius + 2`, мешируется ровно круг `radius`.
    fn assert_circles(world: &World, meshed: &HashSet<IVec2>, center: IVec2, radius: i32) {
        let r = radius + 2;
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
        // Центр и 8 соседей — квадрат 3×3, после них центр готов к мешу.
        let first = frame(&mut loader, &mut world, IVec2::ZERO, &mut meshed, 10);
        assert_eq!(first[0], Progress::Generated(IVec2::ZERO));
        assert!(first[..9].iter().all(|p| matches!(
            p,
            Progress::Generated(key) if key.length_squared() <= 2
        )));
        assert_eq!(first[9], Progress::Mesh(IVec2::ZERO));

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
        // Без одного соседа — по грани или по диагонали — колонка ждёт его загрузки.
        for missing in NEIGHBORS {
            let mut loader = Loader::new(IVec2::ZERO);
            let mut world = World::new();
            world.insert(IVec2::ZERO, Column::new());
            for n in NEIGHBORS.into_iter().filter(|&n| n != missing) {
                world.insert(n, Column::new());
            }
            loader.update(IVec2::ZERO, &mut world);
            assert_eq!(
                loader.advance(&mut world, air),
                Progress::Generated(missing)
            );
            assert_eq!(loader.advance(&mut world, air), Progress::Mesh(IVec2::ZERO));
        }
    }

    #[test]
    fn unloads_only_beyond_margin() {
        let mut loader = Loader::new(IVec2::ZERO);
        let mut world = World::new();
        let mut meshed = HashSet::new();
        settle(&mut loader, &mut world, IVec2::ZERO, &mut meshed);
        let before: HashSet<IVec2> = world.keys().collect();

        // Сдвиг на 2 колонки: старые данные дальше radius + 2 от нового центра,
        // но не дальше radius + 3, должны остаться.
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
        assert!(world.keys().all(|k| dist2(k, center) <= (r + 3).pow(2)));
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
        assert_eq!(world.len(), circle_points(4));
        assert!(mesh.iter().all(|&k| dist2(k, center) <= 4));
    }

    #[test]
    fn edited_column_comes_back_without_generator() {
        use glam::IVec3;

        use world::block::Block;

        let mut loader = Loader::new(IVec2::ZERO);
        let mut world = World::new();
        let mut meshed = HashSet::new();
        settle(&mut loader, &mut world, IVec2::ZERO, &mut meshed);
        let pos = IVec3::new(3, 50, -5);
        let key = IVec2::new(0, -1);
        assert!(world.set_block(pos, Block::Stone));

        let far = IVec2::new(100, 0);
        settle(&mut loader, &mut world, far, &mut meshed);
        assert!(world.column(key).is_none());

        // Обратно: генератор не должен трогать изменённую колонку.
        for key in loader.update(IVec2::ZERO, &mut world) {
            meshed.remove(&key);
        }
        loop {
            let progress = loader.advance(&mut world, |k| {
                assert_ne!(k, key, "edited column regenerated");
                Column::new()
            });
            match progress {
                Progress::Done => break,
                Progress::Mesh(k) => assert!(meshed.insert(k)),
                Progress::Generated(_) => {}
            }
        }
        assert_eq!(world.block(pos), Block::Stone);

        // И после второй выгрузки правка снова сохраняется.
        settle(&mut loader, &mut world, far, &mut meshed);
        assert!(world.is_edited(key));
        settle(&mut loader, &mut world, IVec2::ZERO, &mut meshed);
        assert_eq!(world.block(pos), Block::Stone);
    }

    #[test]
    fn import_into_unloaded_goes_to_loader() {
        let key = IVec2::new(0, 1);
        let pos = glam::IVec3::new(5, 60, 20);
        let mut world = World::new();
        // Правка до импорта: импорт перекрывает и её признак «не сохранено».
        world.insert(key, Column::new());
        world.set_block(pos, world::block::Block::Dirt);
        world.remove(key);
        let mut saved = Column::new();
        saved.set(glam::IVec3::new(5, 60, 4), world::block::Block::Stone);
        world.import(key, saved);
        assert!(world.drain_dirty().is_empty());
        assert!(world.column(key).is_none());
        assert!(world.is_edited(key));

        let mut loader = Loader::new(IVec2::ZERO);
        loader.update(IVec2::ZERO, &mut world);
        while loader.advance(&mut world, |k| {
            assert_ne!(k, key, "imported column regenerated");
            Column::new()
        }) != Progress::Done
        {}
        assert_eq!(world.block(pos), world::block::Block::Stone);
        assert!(world.drain_dirty().is_empty());
    }
}
