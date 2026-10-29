use std::collections::HashSet;

use glam::IVec2;

use world::column::Column;
use world::pos::ColumnPos;
use world::world::World;

/// Пределы и значение по умолчанию дальности прорисовки, колонок; фронт берёт их из движка
/// (`settings_limits()`).
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
    Generated(ColumnPos),
    /// Колонке нужно построить меш. Загрузчик уже считает её мешированной.
    Mesh(ColumnPos),
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
    center: ColumnPos,
    radius: i32,
    /// Радиус уменьшился, и следующий `update` обрезает круги без запаса.
    shrunk: bool,
    /// Смещения в круге `gen_radius`, ближние первыми.
    offsets: Vec<IVec2>,
    meshed: HashSet<ColumnPos>,
    /// Индексы в `offsets`, до которых всё уже мешировано и загружено. Между вызовами
    /// `update` колонки только прибавляются, так что `advance` не просматривает
    /// готовое начало круга заново.
    meshed_prefix: usize,
    loaded_prefix: usize,
}

impl Loader {
    /// `center` — ключ колонки, в которой стоит игрок; дальность — [`DEFAULT_RADIUS`].
    pub fn new(center: ColumnPos) -> Self {
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
        mut generate: impl FnMut(ColumnPos) -> Column,
        radius: i32,
    ) -> Vec<ColumnPos> {
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
        let mesh: Vec<ColumnPos> = self
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
    pub fn update(&mut self, center: ColumnPos, world: &mut World) -> Vec<ColumnPos> {
        self.center = center;
        self.reset_prefixes();
        let dist2 = |key: ColumnPos| (key - center).length_squared();

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
        let far: Vec<ColumnPos> = world
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
        generate: impl FnOnce(ColumnPos) -> Column,
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

    fn next_to_mesh(&mut self, world: &World) -> Option<ColumnPos> {
        let center = self.center;
        let circle = &self.offsets[..self.circle_len(self.radius)];
        while circle
            .get(self.meshed_prefix)
            .is_some_and(|&o| self.meshed.contains(&(center + o)))
        {
            self.meshed_prefix += 1;
        }
        let loaded = |key: ColumnPos| world.column(key).is_some();
        circle[self.meshed_prefix..]
            .iter()
            .map(|&o| center + o)
            .filter(|key| !self.meshed.contains(key))
            .find(|&key| loaded(key) && NEIGHBORS.iter().all(|&n| loaded(key + n)))
    }

    fn next_to_generate(&mut self, world: &World) -> Option<ColumnPos> {
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
fn load(world: &mut World, key: ColumnPos, generate: impl FnOnce(ColumnPos) -> Column) {
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
mod tests;
