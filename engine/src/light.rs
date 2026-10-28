use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};

use glam::{IVec2, IVec3};

use crate::block::Block;
use crate::column::{Column, WORLD_HEIGHT};
use crate::section::SECTION_SIZE;
use crate::world::{World, affected_sections, column_key};

pub const MAX_LIGHT: u8 = 15;

/// Клеток в колонке 16×128×16.
const COLUMN_VOLUME: usize = (SECTION_SIZE * SECTION_SIZE * WORLD_HEIGHT) as usize;

/// Самая тёмная яркость: при 0.8^15 ≈ 0.035 стены в глубине пещер сливаются с чёрным.
const MIN_BRIGHTNESS: f32 = 0.05;

/// Яркость уровня света: 0.8^(15 − level), не ниже [`MIN_BRIGHTNESS`].
pub fn brightness(level: u8) -> f32 {
    level_brightness(f32::from(level))
}

pub fn level_brightness(level: f32) -> f32 {
    0.8f32
        .powf(f32::from(MAX_LIGHT) - level)
        .max(MIN_BRIGHTNESS)
}

/// Два канала света в одном байте: солнечный в младших 4 битах, свет блоков в старших.
/// Мешеру нужны оба уровня клетки, и один байт на клетку держит память колонки в 32 КиБ.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Channel {
    Sky,
    Block,
}

/// Байт света из уровней солнца и блоков (каждый 0..=15).
pub fn pack_light(sky: u8, block: u8) -> u8 {
    debug_assert!(sky <= MAX_LIGHT && block <= MAX_LIGHT);
    sky | block << 4
}

/// Солнечный уровень байта света.
pub fn sky_level(packed: u8) -> u8 {
    packed & MAX_LIGHT
}

/// Уровень света блоков байта света.
pub fn block_level(packed: u8) -> u8 {
    packed >> 4
}

impl Channel {
    /// Уровень этого канала в байте света.
    fn of(self, packed: u8) -> u8 {
        match self {
            Channel::Sky => sky_level(packed),
            Channel::Block => block_level(packed),
        }
    }

    /// Байт света с заменённым уровнем этого канала.
    fn with(self, packed: u8, level: u8) -> u8 {
        match self {
            Channel::Sky => pack_light(level, block_level(packed)),
            Channel::Block => pack_light(sky_level(packed), level),
        }
    }

    /// Свет, который клетка с блоком `block` получает от соседа с уровнем `level`.
    /// Солнечный свет 15 идёт вниз по воздуху без потерь — так столбец под открытым
    /// небом весь светлый; у света блоков такого правила нет. В остальных случаях шаг
    /// стоит 1, а вода гасит 2 за блок; листва — как воздух, 1 за шаг, поэтому под одним
    /// слоем листвы солнца уже 14, а ниже свет убывает по 1. Непрозрачный блок света не
    /// получает (свой уровень излучающего блока ставится отдельно, не через шаг).
    fn step(self, level: u8, down: bool, block: Block) -> u8 {
        if block.is_opaque() {
            return 0;
        }
        if self == Channel::Sky && down && level == MAX_LIGHT && block == Block::Air {
            return MAX_LIGHT;
        }
        level.saturating_sub(if block.is_water() { 2 } else { 1 })
    }
}

/// Байты света колонки ([`pack_light`]), индекс `x + 16·z + 256·y` — как у блоков ([`Column::get_index`]).
pub struct ColumnLight(Box<[u8; COLUMN_VOLUME]>);

impl ColumnLight {
    fn dark() -> Self {
        // Через Vec: `Box::new([0; N])` собирал бы 32 КБ на стеке.
        Self(
            vec![0; COLUMN_VOLUME]
                .into_boxed_slice()
                .try_into()
                .expect("length is COLUMN_VOLUME"),
        )
    }

    /// Байт света по локальным координатам колонки; за её пределами — panic.
    pub fn get(&self, local: IVec3) -> u8 {
        assert!(
            Column::contains(local),
            "local coords out of column: {local}"
        );
        self.0[column_index(local)]
    }
}

impl ColumnLight {
    /// Байты света строки по x при заданных y (0..128) и z (0..16): 16 подряд идущих.
    pub fn row(&self, y: i32, z: i32) -> &[u8] {
        let start = column_index(IVec3::new(0, y, z));
        &self.0[start..start + SECTION_SIZE as usize]
    }
}

fn column_index(local: IVec3) -> usize {
    (local.x + SECTION_SIZE * local.z + SECTION_SIZE * SECTION_SIZE * local.y) as usize
}

/// Свет загруженного мира (солнце и блоки); на диск не сохраняются — выводятся из блоков.
///
/// Свет — неподвижная точка распространения ([`spread`]) по колонкам, добавленным в
/// карту: недобавленные колонки считаются непрозрачной стеной. Поэтому результат не
/// зависит от порядка добавления и удаления: добавление колонки только убирает стену,
/// и свет можно лишь досчитать вверх, а удаление снимает свет, пришедший через неё.
///
/// Свет уходит вбок не дальше 15 блоков (каждый шаг вбок стоит ≥ 1), поэтому любая
/// операция трогает только колонку и её 8 соседей. Отсюда же: свет в колонке и в
/// одном блоке вокруг неё окончателен, как только загружены её 8 соседей, — ровно
/// тогда загрузчик и строит её меш.
pub struct LightMap {
    columns: HashMap<IVec2, ColumnLight>,
    /// Колонки, где свет блоков когда-либо был ярче 0 (с запасом: после снятия ключ
    /// остаётся). Без источников и без таких соседей проход по свету блоков не нужен —
    /// а это почти все колонки, пока игрок не ставил светокамень и не нашёл его в мире.
    block_lit: HashSet<IVec2>,
}

impl LightMap {
    pub fn new() -> Self {
        Self {
            columns: HashMap::new(),
            block_lit: HashSet::new(),
        }
    }

    /// Солнечный уровень по мировым координатам; выше мира — [`MAX_LIGHT`], ниже мира
    /// и в незагруженной колонке — 0.
    pub fn sky(&self, pos: IVec3) -> u8 {
        sky_level(self.packed(pos))
    }

    /// Уровень света блоков по мировым координатам; вне мира и в незагруженной
    /// колонке — 0.
    pub fn block(&self, pos: IVec3) -> u8 {
        block_level(self.packed(pos))
    }

    /// Байт света ([`pack_light`]) по мировым координатам; выше мира — солнце 15,
    /// ниже мира и в незагруженной колонке — 0.
    pub fn packed(&self, pos: IVec3) -> u8 {
        if pos.y >= WORLD_HEIGHT {
            return MAX_LIGHT;
        }
        if pos.y < 0 {
            return 0;
        }
        let key = column_key(pos.x, pos.z);
        let local = IVec3::new(
            pos.x.rem_euclid(SECTION_SIZE),
            pos.y,
            pos.z.rem_euclid(SECTION_SIZE),
        );
        self.columns.get(&key).map_or(0, |light| light.get(local))
    }

    /// Свет колонки `key`, если она в карте.
    pub fn column(&self, key: IVec2) -> Option<&ColumnLight> {
        self.columns.get(&key)
    }

    /// Свет колонки `key` и восьми соседей: индекс `dx + 1 + 3·(dz + 1)`, центр — 4.
    pub fn neighborhood(&self, key: IVec2) -> [Option<&ColumnLight>; 9] {
        std::array::from_fn(|i| {
            self.columns
                .get(&(key + IVec2::new(i as i32 % 3 - 1, i as i32 / 3 - 1)))
        })
    }

    /// Байт света по локальным координатам колонки `key`, в том числе за её краем (x, z в
    /// −16..32), как [`LightMap::packed`]. Мешер спрашивает свет всех клеток колонки, и
    /// поиск колонки в `HashMap` на каждую был бы заметен: колонки 3×3 берутся заранее.
    pub fn column_sampler(&self, key: IVec2) -> impl Fn(IVec3) -> u8 + '_ {
        let columns = self.neighborhood(key);
        move |local: IVec3| {
            if local.y >= WORLD_HEIGHT {
                return MAX_LIGHT;
            }
            let (dx, dz) = (
                local.x.div_euclid(SECTION_SIZE),
                local.z.div_euclid(SECTION_SIZE),
            );
            if local.y < 0 || !(-1..=1).contains(&dx) || !(-1..=1).contains(&dz) {
                return 0;
            }
            let inner = IVec3::new(
                local.x.rem_euclid(SECTION_SIZE),
                local.y,
                local.z.rem_euclid(SECTION_SIZE),
            );
            columns[(dx + 1 + 3 * (dz + 1)) as usize]
                .map_or(0, |light| light.0[column_index(inner)])
        }
    }

    /// Ключи колонок со светом, в произвольном порядке.
    pub fn keys(&self) -> impl Iterator<Item = IVec2> + '_ {
        self.columns.keys().copied()
    }

    pub fn len(&self) -> usize {
        self.columns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }

    /// Байт под уровни света всех колонок.
    pub fn memory_bytes(&self) -> usize {
        self.columns.len() * COLUMN_VOLUME
    }

    /// Считает свет колонки `key` из `world` и досчитывает свет соседей, которые уже
    /// в карте: солнце по столбцам, свет соседей через границу внутрь и свет колонки
    /// наружу. Соседей, которых ещё нет, учтёт их собственный `add_column`.
    ///
    /// Возвращает ключи секций других колонок, чей меш зависит от изменившегося света
    /// (сами клетки и блок вокруг них). Секции соседей, которые смотрят через край на
    /// саму новую колонку, сюда не входят: до неё они видели стену, и их меш
    /// перестраивает тот, кто добавляет колонку (загрузчик строит меш только после
    /// загрузки всех 8 соседей). Колонку, которой нет в `world`, пропускает;
    /// уже добавленную — пересчитывает заново.
    pub fn add_column(&mut self, world: &World, key: IVec2) -> Vec<IVec3> {
        let Some(column) = world.column(key) else {
            return Vec::new();
        };
        let mut light = ColumnLight::dark();
        let beams = sky_beams(column, &mut light);
        let mut changes = self.remove_column(world, key);
        self.columns.insert(key, light);

        let mut window = Window::new(&mut self.columns, world, key, false, Channel::Sky);
        window.seed_center(&beams);
        window.spread();
        changes.extend(window.changed_sections());

        let emits = (0..WORLD_HEIGHT / SECTION_SIZE).any(|sy| column.section(sy).emits_light());
        if emits || self.near_block_light(key) {
            let mut window = Window::new(&mut self.columns, world, key, false, Channel::Block);
            window.seed_emitters(column);
            window.spread();
            changes.extend(window.changed_sections());
            let touched = window.touched_columns();
            self.mark_block_lit(key, touched);
        }
        finish(changes, Some(key))
    }

    /// Убирает свет колонки `key` и снимает свет соседей, пришедший через неё; `world`
    /// нужен для блоков соседей, чтобы заново растечь их собственный свет. Возвращает
    /// ключи секций, чей меш зависит от изменившегося света соседей.
    pub fn remove_column(&mut self, world: &World, key: IVec2) -> Vec<IVec3> {
        let Some(old) = self.columns.remove(&key) else {
            return Vec::new();
        };
        let block_lit = self.block_lit.remove(&key);
        let mut changes = Vec::new();
        for channel in [Channel::Sky, Channel::Block] {
            if channel == Channel::Block && !block_lit {
                // Своего света блоков не было — снимать у соседей нечего.
                continue;
            }
            let mut window = Window::new(&mut self.columns, world, key, true, channel);
            let mut removal = Vec::new();
            for (dir, side) in SIDES {
                for y in 0..WORLD_HEIGHT {
                    for i in 0..SECTION_SIZE {
                        let edge = side.edge(i, y);
                        let level = channel.of(old.0[column_index(edge)]);
                        if level > 0 {
                            // Клетка соседа за краем — в координатах окна.
                            let cell = edge + IVec3::new(SECTION_SIZE, 0, SECTION_SIZE) + dir;
                            window.unlight_neighbor(cell, level, false, &mut removal);
                        }
                    }
                }
            }
            window.unspread(removal);
            window.spread();
            changes.extend(window.changed_sections());
            let touched = window.touched_columns();
            self.mark_block_lit(key, touched);
        }
        finish(changes, None)
    }

    /// После `world.set_block(pos, ..)`: снимает свет, который мог зависеть от клетки
    /// `pos`, и заново растекает его от границы снятого и от соседей `pos`. Возвращает
    /// ключи всех секций, чей меш зависит от изменившегося света. Вне высоты мира и в
    /// колонке без света ничего не делает.
    pub fn block_changed(&mut self, world: &World, pos: IVec3) -> Vec<IVec3> {
        let key = column_key(pos.x, pos.z);
        if !(0..WORLD_HEIGHT).contains(&pos.y) || !self.columns.contains_key(&key) {
            return Vec::new();
        }
        let mut changes = Vec::new();
        for channel in [Channel::Sky, Channel::Block] {
            if channel == Channel::Block
                && world.block(pos).emission() == 0
                && !self.near_block_light(key)
            {
                continue;
            }
            let mut window = Window::new(&mut self.columns, world, key, true, channel);
            let cell = pos - window.origin;
            let slot = window.slot(cell).expect("edited column is in the window");
            let old = window.level(slot);
            let mut removal = Vec::new();
            if old > 0 {
                window.set(cell, slot, 0);
                removal.push((cell, old));
            }
            window.unspread(removal);
            // Излучающий блок держит свой уровень сам; остальное придёт от соседей.
            let emission = if channel == Channel::Block {
                window.block(slot).emission()
            } else {
                0
            };
            if emission > 0 {
                window.set(cell, slot, emission);
                window.enqueue(cell, emission);
            }
            // Свет самой клетки заново придёт от соседей (и от неба, если она верхняя).
            for dir in DIRECTIONS {
                let neighbor = cell + dir;
                if let Some(slot) = window.slot(neighbor) {
                    window.enqueue(neighbor, window.level(slot));
                }
            }
            if channel == Channel::Sky && cell.y == WORLD_HEIGHT - 1 {
                window.relight_from_sky(cell);
            }
            window.spread();
            changes.extend(window.changed_sections());
            let touched = window.touched_columns();
            self.mark_block_lit(key, touched);
        }
        finish(changes, None)
    }

    /// Есть ли свет блоков в колонке или её 8 соседях.
    fn near_block_light(&self, key: IVec2) -> bool {
        !self.block_lit.is_empty()
            && (-1..=1)
                .any(|dz| (-1..=1).any(|dx| self.block_lit.contains(&(key + IVec2::new(dx, dz)))))
    }

    /// Запоминает колонки окна вокруг `center`, куда проход записал свет блоков;
    /// `touched` — их битовая маска по номеру в окне ([`Window::touched_columns`]).
    fn mark_block_lit(&mut self, center: IVec2, touched: u16) {
        for i in 0..9 {
            if touched >> i & 1 != 0 {
                self.block_lit
                    .insert(center + IVec2::new(i % 3 - 1, i / 3 - 1));
            }
        }
    }
}

impl Default for LightMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Убирает дубли, по желанию — секции колонки `skip`, и сортирует ради одинакового порядка.
fn finish(keys: Vec<IVec3>, skip: Option<IVec2>) -> Vec<IVec3> {
    let mut keys: Vec<IVec3> = keys
        .into_iter()
        .filter(|k| skip != Some(IVec2::new(k.x, k.z)))
        .collect();
    keys.sort_by_key(|k| k.to_array());
    keys.dedup();
    keys
}

/// Солнце по столбцам колонки (индекс столбца `x + 16·z`).
struct Beams {
    /// Нижняя клетка, до которой дошло солнце сверху (`WORLD_HEIGHT` — ни одной).
    bottom: [i32; 256],
    /// Нижняя клетка непрерывного уровня 15 от верха мира (`WORLD_HEIGHT` — ни одной).
    /// Выше неё и соседа свет 15 у обоих, и между ними растекаться нечему.
    full: [i32; 256],
}

/// Солнце по столбцам колонки: от верха мира вниз, пока свет не кончится или не
/// встретится непрозрачный блок.
fn sky_beams(column: &Column, light: &mut ColumnLight) -> Beams {
    let mut beams = Beams {
        bottom: [WORLD_HEIGHT; 256],
        full: [WORLD_HEIGHT; 256],
    };
    for xz in 0..256 {
        let mut level = MAX_LIGHT;
        for y in (0..WORLD_HEIGHT).rev() {
            let index = xz + (y * SECTION_SIZE * SECTION_SIZE) as usize;
            level = Channel::Sky.step(level, true, column.get_index(index));
            if level == 0 {
                break;
            }
            light.0[index] = level;
            beams.bottom[xz] = y;
            if level == MAX_LIGHT {
                beams.full[xz] = y;
            }
        }
    }
    beams
}

const DIRECTIONS: [IVec3; 6] = [
    IVec3::X,
    IVec3::NEG_X,
    IVec3::Z,
    IVec3::NEG_Z,
    IVec3::Y,
    IVec3::NEG_Y,
];

/// Сторона колонки: направление наружу и как перебрать её крайние клетки.
#[derive(Clone, Copy)]
enum Side {
    MaxX,
    MinX,
    MaxZ,
    MinZ,
}

const SIDES: [(IVec3, Side); 4] = [
    (IVec3::X, Side::MaxX),
    (IVec3::NEG_X, Side::MinX),
    (IVec3::Z, Side::MaxZ),
    (IVec3::NEG_Z, Side::MinZ),
];

impl Side {
    /// Крайняя клетка `i` (0..16 вдоль стороны) на высоте `y`, в координатах колонки.
    fn edge(self, i: i32, y: i32) -> IVec3 {
        let last = SECTION_SIZE - 1;
        match self {
            Side::MaxX => IVec3::new(last, y, i),
            Side::MinX => IVec3::new(0, y, i),
            Side::MaxZ => IVec3::new(i, y, last),
            Side::MinZ => IVec3::new(i, y, 0),
        }
    }
}

/// Ширина окна 3×3 колонок в блоках.
const SPAN: i32 = 3 * SECTION_SIZE;
/// Номер центральной колонки в окне.
const CENTER: usize = 4;

/// Колонка окна и индекс клетки в ней.
#[derive(Clone, Copy)]
struct Slot {
    column: usize,
    index: usize,
}

/// Колонка и 8 её соседей на время одной операции. Клетки — в координатах окна:
/// x, z в 0..48 (центральная колонка — 16..32), y в 0..128. Колонки без света
/// (или без блоков в `world`) — стена.
struct Window<'a> {
    light: [Option<&'a mut [u8; COLUMN_VOLUME]>; 9],
    blocks: [Option<&'a Column>; 9],
    /// Мировые координаты клетки окна (0, 0, 0).
    origin: IVec3,
    /// Очередь растекания по уровням: уровни только убывают, так что, разбирая от
    /// 15 вниз, каждая клетка получает окончательный уровень с первого раза.
    queue: [Vec<IVec3>; MAX_LIGHT as usize + 1],
    /// Исходный уровень клеток, менявшихся за операцию: изменились ли они в итоге,
    /// видно только в конце — снятый свет часто возвращается тем же уровнем.
    original: HashMap<u32, u8, BuildHasherDefault<CellHasher>>,
    /// Учитывать ли записи в центральную колонку (у новой они неинтересны и дороги).
    track_center: bool,
    /// Какой канал света окно считает; второй канал байта оно не трогает.
    channel: Channel,
    /// Колонки окна (битами по номеру), куда писался свет блоков ярче 0.
    block_touched: u16,
}

impl<'a> Window<'a> {
    fn new(
        columns: &'a mut HashMap<IVec2, ColumnLight>,
        world: &'a World,
        center: IVec2,
        track_center: bool,
        channel: Channel,
    ) -> Self {
        let keys: [IVec2; 9] =
            std::array::from_fn(|i| center + IVec2::new(i as i32 % 3 - 1, i as i32 / 3 - 1));
        let mut light = columns
            .get_disjoint_mut(keys.each_ref())
            .map(|l| l.map(|l| &mut *l.0));
        let blocks = std::array::from_fn(|i| {
            let column = world.column(keys[i]);
            if column.is_none() {
                light[i] = None;
            }
            column.filter(|_| light[i].is_some())
        });
        Self {
            light,
            blocks,
            origin: IVec3::new(
                (center.x - 1) * SECTION_SIZE,
                0,
                (center.y - 1) * SECTION_SIZE,
            ),
            queue: Default::default(),
            original: HashMap::default(),
            track_center,
            channel,
            block_touched: 0,
        }
    }

    /// `None` — клетка вне окна, вне высоты мира или в стене.
    fn slot(&self, cell: IVec3) -> Option<Slot> {
        if !(0..SPAN).contains(&cell.x)
            || !(0..SPAN).contains(&cell.z)
            || !(0..WORLD_HEIGHT).contains(&cell.y)
        {
            return None;
        }
        let column = (cell.x / SECTION_SIZE + 3 * (cell.z / SECTION_SIZE)) as usize;
        self.light[column].as_ref()?;
        let local = IVec3::new(cell.x % SECTION_SIZE, cell.y, cell.z % SECTION_SIZE);
        Some(Slot {
            column,
            index: column_index(local),
        })
    }

    fn level(&self, slot: Slot) -> u8 {
        self.channel
            .of(self.light[slot.column].as_ref().expect("slot is lit")[slot.index])
    }

    fn block(&self, slot: Slot) -> Block {
        self.blocks[slot.column]
            .expect("slot has blocks")
            .get_index(slot.index)
    }

    fn set(&mut self, cell: IVec3, slot: Slot, level: u8) {
        let light = self.light[slot.column].as_mut().expect("slot is lit");
        let packed = light[slot.index];
        if self.track_center || slot.column != CENTER {
            self.original
                .entry(pack(cell))
                .or_insert(self.channel.of(packed));
        }
        light[slot.index] = self.channel.with(packed, level);
        if self.channel == Channel::Block && level > 0 {
            self.block_touched |= 1 << slot.column;
        }
    }

    fn enqueue(&mut self, cell: IVec3, level: u8) {
        // Уровень 1 дальше не растекается.
        if level > 1 {
            self.queue[usize::from(level)].push(cell);
        }
    }

    /// Поднимает клетку до `level`, если это ярче, и ставит её в очередь.
    fn raise(&mut self, cell: IVec3, slot: Slot, level: u8) {
        if level > self.level(slot) {
            self.set(cell, slot, level);
            self.enqueue(cell, level);
        }
    }

    /// Начало растекания новой центральной колонки, когда её солнце уже посчитано:
    /// из освещённых столбцов вбок, внутри колонки и в соседей, и из крайних клеток
    /// соседей внутрь. Вниз и вверх столбцы уже растеклись сами, поэтому в очередь
    /// идут только клетки, которые стали светлее, — иначе в ней лежали бы тысячи
    /// клеток открытого неба, которым некуда светить.
    fn seed_center(&mut self, beams: &Beams) {
        let base = IVec3::new(SECTION_SIZE, 0, SECTION_SIZE);
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                let xz = (x + SECTION_SIZE * z) as usize;
                let column = base + IVec3::new(x, 0, z);
                for dir in &DIRECTIONS[..4] {
                    let beside = column + *dir;
                    let outside = !(0..SECTION_SIZE).contains(&(x + dir.x))
                        || !(0..SECTION_SIZE).contains(&(z + dir.z));
                    let beside_full = if outside {
                        let Some(full) = self.full_from(beside) else {
                            continue;
                        };
                        full
                    } else {
                        beams.full[(xz as i32 + dir.x + SECTION_SIZE * dir.z) as usize]
                    };
                    let top = beams.full[xz].max(beside_full);
                    for y in beams.bottom[xz]..top {
                        let cell = column.with_y(y);
                        let level = self.level(self.slot(cell).expect("center is lit"));
                        self.light_neighbor(beside.with_y(y), level, false);
                    }
                    if outside {
                        for y in 0..top {
                            let level = self.level(self.slot(beside.with_y(y)).expect("lit"));
                            self.light_neighbor(column.with_y(y), level, false);
                        }
                    }
                }
            }
        }
    }

    /// Начало растекания света блоков новой центральной колонки: излучающие блоки
    /// колонки и крайние клетки соседей, чей свет идёт внутрь. Блоки света в соседях
    /// уже растеклись и в новую колонку (до неё она была стеной) не попали.
    fn seed_emitters(&mut self, column: &Column) {
        let base = IVec3::new(SECTION_SIZE, 0, SECTION_SIZE);
        for sy in 0..WORLD_HEIGHT / SECTION_SIZE {
            if !column.section(sy).emits_light() {
                continue;
            }
            for y in sy * SECTION_SIZE..(sy + 1) * SECTION_SIZE {
                for z in 0..SECTION_SIZE {
                    for x in 0..SECTION_SIZE {
                        let local = IVec3::new(x, y, z);
                        let emission = column.get_index(column_index(local)).emission();
                        if emission > 0 {
                            let cell = base + local;
                            let slot = self.slot(cell).expect("center is lit");
                            self.set(cell, slot, emission);
                            self.enqueue(cell, emission);
                        }
                    }
                }
            }
        }
        for (dir, side) in SIDES {
            for y in 0..WORLD_HEIGHT {
                for i in 0..SECTION_SIZE {
                    let beside = side.edge(i, y) + base + dir;
                    if let Some(slot) = self.slot(beside) {
                        self.enqueue(beside, self.level(slot));
                    }
                }
            }
        }
    }

    /// Нижняя клетка непрерывного уровня 15 от верха мира в столбце `column` (y не
    /// важен); `None` — столбец в стене.
    fn full_from(&self, column: IVec3) -> Option<i32> {
        let mut full = WORLD_HEIGHT;
        for y in (0..WORLD_HEIGHT).rev() {
            if self.level(self.slot(column.with_y(y))?) != MAX_LIGHT {
                break;
            }
            full = y;
        }
        Some(full)
    }

    /// Свет `level` из соседней клетки в клетку `cell`.
    fn light_neighbor(&mut self, cell: IVec3, level: u8, down: bool) {
        if level <= 1 {
            return;
        }
        if let Some(slot) = self.slot(cell) {
            let lit = self.channel.step(level, down, self.block(slot));
            self.raise(cell, slot, lit);
        }
    }

    /// Растекание очереди, от ярких клеток к тёмным.
    fn spread(&mut self) {
        for level in (2..=MAX_LIGHT).rev() {
            while let Some(cell) = self.queue[usize::from(level)].pop() {
                // Клетку могли снять или осветить ярче после постановки в очередь:
                // тогда запись устарела, а новая лежит в своём уровне.
                if self.slot(cell).is_none_or(|slot| self.level(slot) != level) {
                    continue;
                }
                for dir in DIRECTIONS {
                    self.light_neighbor(cell + dir, level, dir == IVec3::NEG_Y);
                }
            }
        }
    }

    /// Снятие света: `removal` — клетки, уже обнулённые, с их прежним уровнем. Соседи,
    /// чей свет мог прийти от снятой клетки (темнее её или солнечный столбец под ней),
    /// снимаются тоже; остальные освещённые соседи — источники для повторного
    /// растекания и ставятся в очередь.
    fn unspread(&mut self, mut removal: Vec<(IVec3, u8)>) {
        let mut next = 0;
        while let Some(&(cell, old)) = removal.get(next) {
            next += 1;
            for dir in DIRECTIONS {
                self.unlight_neighbor(cell + dir, old, dir == IVec3::NEG_Y, &mut removal);
            }
        }
    }

    /// Соседняя клетка снятой клетки с уровнем `old`: снять или запомнить как источник.
    fn unlight_neighbor(
        &mut self,
        cell: IVec3,
        old: u8,
        down: bool,
        removal: &mut Vec<(IVec3, u8)>,
    ) {
        let Some(slot) = self.slot(cell) else {
            return;
        };
        let level = self.level(slot);
        if level == 0 {
            return;
        }
        // Излучающий блок держит свой уровень сам и снятию не подлежит.
        let emitter = self.channel == Channel::Block && self.block(slot).emission() > 0;
        // Свет, пришедший от снятой клетки, всегда темнее её — кроме солнечного
        // столбца, который идёт вниз без потерь.
        let sunbeam =
            self.channel == Channel::Sky && down && old == MAX_LIGHT && level == MAX_LIGHT;
        if !emitter && (level < old || sunbeam) {
            self.set(cell, slot, 0);
            removal.push((cell, level));
            if self.channel == Channel::Sky && cell.y == WORLD_HEIGHT - 1 {
                self.relight_from_sky(cell);
            }
        } else {
            self.enqueue(cell, level);
        }
    }

    /// Верхняя клетка мира получает солнце прямо с неба.
    fn relight_from_sky(&mut self, cell: IVec3) {
        self.light_neighbor(cell, MAX_LIGHT, true);
    }

    /// Колонки окна (битами по номеру), куда писался свет блоков ярче 0.
    fn touched_columns(&self) -> u16 {
        self.block_touched
    }

    /// Секции, чей меш зависит от клеток, уровень которых за операцию изменился.
    /// Изменённых клеток бывают тысячи (свет залил зал), а набор задетых секций у них
    /// определяется секцией клетки и тем, у каких её граней клетка лежит, — поэтому
    /// `affected_sections` зовётся один раз на такой набор.
    fn changed_sections(&self) -> Vec<IVec3> {
        const SECTIONS: usize = 9 * (WORLD_HEIGHT / SECTION_SIZE) as usize;
        let mut seen = vec![false; SECTIONS * 27];
        let mut keys = HashSet::new();
        for (&packed, &original) in &self.original {
            let cell = unpack(packed);
            let slot = self.slot(cell).expect("changed cell is lit");
            if self.level(slot) == original {
                continue;
            }
            let section = cell / SECTION_SIZE;
            let edge = |c: i32| match c % SECTION_SIZE {
                0 => 0,
                l if l == SECTION_SIZE - 1 => 1,
                _ => 2,
            };
            let kind = edge(cell.x) + 3 * edge(cell.y) + 9 * edge(cell.z);
            let index = (section.x + 3 * section.z) as usize * SECTIONS / 9 + section.y as usize;
            if !std::mem::replace(&mut seen[index * 27 + kind as usize], true) {
                keys.extend(affected_sections(self.origin + cell));
            }
        }
        keys.into_iter().collect()
    }
}

/// Клетка окна → число: ключ в `Window::original`.
fn pack(cell: IVec3) -> u32 {
    (cell.x + SPAN * cell.z + SPAN * SPAN * cell.y) as u32
}

fn unpack(packed: u32) -> IVec3 {
    let packed = packed as i32;
    IVec3::new(packed % SPAN, packed / (SPAN * SPAN), packed / SPAN % SPAN)
}

/// Хеш упакованной клетки умножением на нечётную константу (как FxHash): младшие
/// биты у соседних клеток различны, а SipHash по умолчанию для таких ключей в разы
/// дороже — на правке с тысячами изменённых клеток это заметно.
#[derive(Default)]
struct CellHasher(u64);

impl Hasher for CellHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = (self.0.rotate_left(5) ^ u64::from(byte)).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        }
    }

    fn write_u32(&mut self, value: u32) {
        self.0 = u64::from(value).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;
    use crate::terrain::Terrain;

    /// Ключи квадрата колонок `center ± radius`.
    fn square(center: IVec2, radius: i32) -> Vec<IVec2> {
        (-radius..=radius)
            .flat_map(|z| (-radius..=radius).map(move |x| center + IVec2::new(x, z)))
            .collect()
    }

    /// Мир из колонок `keys`, блок задаёт `block_at` по мировым координатам.
    fn world_of(keys: &[IVec2], block_at: impl Fn(IVec3) -> Block) -> World {
        let mut world = World::new();
        for &key in keys {
            let mut column = Column::new();
            for y in 0..WORLD_HEIGHT {
                for z in 0..SECTION_SIZE {
                    for x in 0..SECTION_SIZE {
                        let local = IVec3::new(x, y, z);
                        let pos = local + IVec3::new(key.x, 0, key.y) * SECTION_SIZE;
                        column.set(local, block_at(pos));
                    }
                }
            }
            world.insert(key, column);
        }
        world
    }

    fn terrain_world(keys: &[IVec2]) -> World {
        let terrain = Terrain::new(1337);
        let mut world = World::new();
        for &key in keys {
            world.insert(key, terrain.generate(key));
        }
        world
    }

    fn lit(world: &World, order: &[IVec2]) -> LightMap {
        let mut light = LightMap::new();
        for &key in order {
            light.add_column(world, key);
        }
        light
    }

    /// Эталон: байты света ([`pack_light`]) всех колонок `keys` целиком; клетки без
    /// света в карту не входят.
    fn reference(world: &World, keys: &[IVec2]) -> HashMap<IVec3, u8> {
        let mut packed = reference_channel(world, keys, Channel::Sky);
        for (pos, level) in reference_channel(world, keys, Channel::Block) {
            let sky = packed.get(&pos).copied().unwrap_or(0);
            packed.insert(pos, sky | level << 4);
        }
        packed
    }

    /// Эталон одного канала, отдельной простой реализацией — очередь с повторной
    /// постановкой при улучшении, без затравки по столбцам и без окна. Незагруженное —
    /// стена, как и в [`LightMap`].
    fn reference_channel(world: &World, keys: &[IVec2], channel: Channel) -> HashMap<IVec3, u8> {
        let loaded: HashSet<IVec2> = keys.iter().copied().collect();
        let inside =
            |p: IVec3| (0..WORLD_HEIGHT).contains(&p.y) && loaded.contains(&column_key(p.x, p.z));
        let mut light: HashMap<IVec3, u8> = HashMap::new();
        let mut queue = VecDeque::new();
        for &key in keys {
            for y in 0..WORLD_HEIGHT {
                for z in 0..SECTION_SIZE {
                    for x in 0..SECTION_SIZE {
                        let pos = IVec3::new(key.x * SECTION_SIZE + x, y, key.y * SECTION_SIZE + z);
                        let level = match channel {
                            Channel::Sky if y == WORLD_HEIGHT - 1 => {
                                channel.step(MAX_LIGHT, true, world.block(pos))
                            }
                            Channel::Sky => 0,
                            Channel::Block => world.block(pos).emission(),
                        };
                        if level > 0 {
                            light.insert(pos, level);
                            queue.push_back(pos);
                        }
                    }
                }
            }
        }
        while let Some(pos) = queue.pop_front() {
            let level = light[&pos];
            for dir in DIRECTIONS {
                let next = pos + dir;
                if !inside(next) {
                    continue;
                }
                let value = channel.step(level, dir == IVec3::NEG_Y, world.block(next));
                if value > light.get(&next).copied().unwrap_or(0) {
                    light.insert(next, value);
                    queue.push_back(next);
                }
            }
        }
        light
    }

    /// Все клетки колонок `keys` совпадают с эталоном.
    fn assert_matches_reference(light: &LightMap, world: &World, keys: &[IVec2], context: &str) {
        let expected = reference(world, keys);
        for &key in keys {
            for y in 0..WORLD_HEIGHT {
                for z in 0..SECTION_SIZE {
                    for x in 0..SECTION_SIZE {
                        let pos = IVec3::new(key.x * SECTION_SIZE + x, y, key.y * SECTION_SIZE + z);
                        let want = expected.get(&pos).copied().unwrap_or(0);
                        assert_eq!(light.packed(pos), want, "{context}: {pos}");
                    }
                }
            }
        }
    }

    fn floor(y: i32) -> impl Fn(IVec3) -> Block {
        move |p: IVec3| if p.y <= y { Block::Stone } else { Block::Air }
    }

    #[test]
    fn open_sky_is_full_above_ground() {
        let keys = square(IVec2::new(-1, 2), 1);
        let world = world_of(&keys, floor(10));
        let light = lit(&world, &keys);
        for &key in &keys {
            for y in 0..WORLD_HEIGHT + 2 {
                let pos = IVec3::new(key.x * 16 + 7, y, key.y * 16 + 15);
                assert_eq!(light.sky(pos), if y > 10 { MAX_LIGHT } else { 0 }, "{pos}");
            }
        }
        assert_eq!(light.sky(IVec3::new(0, -1, 0)), 0);
        assert_eq!(light.sky(IVec3::new(1000, 50, 0)), 0);
    }

    #[test]
    fn light_under_canopy_fades_from_edge() {
        let keys = square(IVec2::ZERO, 1);
        // Пол на y = 10, навес на y = 14 над x ≥ 0: под ним свет приходит сбоку от x = −1.
        let world = world_of(&keys, |p| {
            if p.y <= 10 || (p.y == 14 && p.x >= 0) {
                Block::Stone
            } else {
                Block::Air
            }
        });
        let light = lit(&world, &keys);
        for x in 0..20 {
            for y in 11..14 {
                let want = (14 - x).max(0) as u8;
                assert_eq!(light.sky(IVec3::new(x, y, 3)), want, "x {x}, y {y}");
            }
        }
        assert_eq!(light.sky(IVec3::new(-1, 12, 3)), MAX_LIGHT);
        assert_matches_reference(&light, &world, &keys, "canopy");
    }

    #[test]
    fn hole_in_box_lights_and_closing_darkens() {
        let keys = square(IVec2::ZERO, 1);
        // Полая коробка 9×9×9 в центральной колонке, стенки толщиной 1.
        let shell = |p: IVec3| {
            let inner = p - IVec3::new(4, 40, 4);
            inner.cmpge(IVec3::ZERO).all()
                && inner.cmplt(IVec3::splat(9)).all()
                && !(inner.cmpgt(IVec3::ZERO).all() && inner.cmplt(IVec3::splat(8)).all())
        };
        let mut world = world_of(&keys, |p| {
            if p.y <= 10 || shell(p) {
                Block::Stone
            } else {
                Block::Air
            }
        });
        let mut light = lit(&world, &keys);
        let inside = IVec3::new(8, 44, 8);
        assert_eq!(light.sky(inside), 0);
        let closed: Vec<u8> = light.column(IVec2::ZERO).unwrap().0.to_vec();

        let hole = IVec3::new(8, 48, 8);
        assert!(world.set_block(hole, Block::Air));
        let changed = light.block_changed(&world, hole);
        for y in 41..=48 {
            assert_eq!(light.sky(IVec3::new(8, y, 8)), MAX_LIGHT, "y {y}");
        }
        assert_eq!(light.sky(IVec3::new(6, 41, 8)), 13);
        assert_eq!(light.sky(IVec3::new(5, 41, 5)), 9);
        assert!(changed.contains(&IVec3::new(0, 2, 0)), "{changed:?}");
        assert_matches_reference(&light, &world, &keys, "open");

        assert!(world.set_block(hole, Block::Stone));
        let changed = light.block_changed(&world, hole);
        assert!(changed.contains(&IVec3::new(0, 2, 0)), "{changed:?}");
        assert_eq!(light.column(IVec2::ZERO).unwrap().0.to_vec(), closed);
    }

    #[test]
    fn water_dims_by_two_and_leaves_by_one() {
        let keys = square(IVec2::ZERO, 1);
        let world = world_of(&keys, |p| match p.y {
            ..=50 => Block::Stone,
            51..=60 => Block::Water,
            // Листва — только над соседней колонкой, чтобы не затенять воду.
            70..=74 if p.x >= 16 => Block::Leaves,
            _ => Block::Air,
        });
        let light = lit(&world, &keys);
        for depth in 1..=10 {
            let want = (15 - 2 * depth).max(0) as u8;
            assert_eq!(
                light.sky(IVec3::new(3, 61 - depth, 3)),
                want,
                "depth {depth}"
            );
        }
        for depth in 1..=5 {
            assert_eq!(light.sky(IVec3::new(24, 75 - depth, 3)), 15 - depth as u8);
        }
        // Под листвой свет уже не солнечный и убывает по 1 за блок; сбоку на 9 блоков
        // от открытого неба пришло бы только 6.
        assert_eq!(light.sky(IVec3::new(24, 69, 3)), 9);
        assert_eq!(light.sky(IVec3::new(24, 68, 3)), 8);
        assert_matches_reference(&light, &world, &keys, "water");
    }

    /// Окно 3×3 с пещерами, водой и деревьями: колонки задевают их на стыках.
    const CENTERS: [IVec2; 2] = [IVec2::new(-1, 2), IVec2::new(-6, 6)];

    #[test]
    fn order_of_columns_does_not_matter() {
        for center in CENTERS {
            let keys = square(center, 1);
            let world = terrain_world(&keys);
            let forward = lit(&world, &keys);
            let mut backward_order = keys.clone();
            backward_order.reverse();
            // Центр последним: свет соседей уже есть и должен перетечь в обе стороны.
            backward_order.retain(|&k| k != center);
            backward_order.push(center);
            let backward = lit(&world, &backward_order);
            for &key in &keys {
                assert_eq!(
                    forward.column(key).unwrap().0[..],
                    backward.column(key).unwrap().0[..],
                    "{key}"
                );
            }
            assert_matches_reference(&forward, &world, &keys, "terrain");
        }
    }

    #[test]
    fn remove_and_add_again_restores_light() {
        let center = CENTERS[0];
        let keys = square(center, 1);
        let world = terrain_world(&keys);
        let mut light = lit(&world, &keys);
        let before: Vec<Vec<u8>> = keys
            .iter()
            .map(|&k| light.column(k).unwrap().0.to_vec())
            .collect();

        for removed in [center, center + IVec2::X, center + IVec2::ONE] {
            light.remove_column(&world, removed);
            assert!(light.column(removed).is_none());
            let rest: Vec<IVec2> = keys.iter().copied().filter(|&k| k != removed).collect();
            assert_matches_reference(&light, &world, &rest, "removed");
            light.add_column(&world, removed);
            let after: Vec<Vec<u8>> = keys
                .iter()
                .map(|&k| light.column(k).unwrap().0.to_vec())
                .collect();
            assert!(after == before, "{removed}");
        }
    }

    #[test]
    fn random_edits_match_full_recompute() {
        let center = CENTERS[0];
        let keys = square(center, 1);
        let terrain = Terrain::new(1337);
        let mut world = terrain_world(&keys);
        let mut light = lit(&world, &keys);
        // Простой ГПСЧ (xorshift): без зависимостей и одинаково на любой платформе.
        let mut state = 0x9E37_79B9_7F4A_7C15_u64;
        let mut next = move |n: i32| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state % n as u64) as i32
        };
        let blocks = [
            Block::Air,
            Block::Air,
            Block::Stone,
            Block::Leaves,
            Block::Water,
            Block::Glowstone,
        ];
        let origin = IVec3::new(center.x - 1, 0, center.y - 1) * SECTION_SIZE;
        for step in 0..40 {
            let (x, z) = (next(48), next(48));
            let surface = terrain.surface_height(origin.x + x, origin.z + z);
            let y = (surface - 6 + next(10)).clamp(0, WORLD_HEIGHT - 1);
            let pos = origin + IVec3::new(x, y, z);
            let block = blocks[next(blocks.len() as i32) as usize];
            assert!(world.set_block(pos, block));
            let changed = light.block_changed(&world, pos);
            if step % 4 == 3 {
                assert_matches_reference(
                    &light,
                    &world,
                    &keys,
                    &format!("step {step} {pos} {block:?}"),
                );
            }
            for key in changed {
                assert!((0..8).contains(&key.y), "{key}");
            }
        }
        assert_matches_reference(&light, &world, &keys, "end");
    }

    #[test]
    fn changed_sections_cover_every_changed_cell() {
        let center = CENTERS[0];
        let keys = square(center, 1);
        let terrain = Terrain::new(1337);
        let mut world = terrain_world(&keys);
        let mut light = lit(&world, &keys);
        let x = center.x * 16 + 15;
        let z = center.y * 16;
        let top = terrain.surface_height(x, z);
        // Шахта вниз на краю колонки: свет уходит в соседей.
        for y in (top - 12..=top).rev() {
            let before = reference(&world, &keys);
            let pos = IVec3::new(x, y, z);
            world.set_block(pos, Block::Air);
            let changed: HashSet<IVec3> = light.block_changed(&world, pos).into_iter().collect();
            for (cell, level) in reference(&world, &keys) {
                if before.get(&cell).copied().unwrap_or(0) != level {
                    for key in affected_sections(cell) {
                        assert!(changed.contains(&key), "{cell}: {key} missing");
                    }
                }
            }
        }
    }

    /// Закрытый зал 3×3 колонок: камень, внутри воздух от y=10 до y=40.
    fn dark_hall() -> (World, Vec<IVec2>) {
        let keys = square(IVec2::ZERO, 1);
        let world = world_of(&keys, |p| {
            if (10..=40).contains(&p.y) && (-16..32).contains(&p.x) && (-16..32).contains(&p.z) {
                Block::Air
            } else {
                Block::Stone
            }
        });
        (world, keys)
    }

    fn place(world: &mut World, light: &mut LightMap, pos: IVec3, block: Block) {
        assert!(world.set_block(pos, block));
        light.block_changed(world, pos);
    }

    #[test]
    fn light_bits_are_packed_independently() {
        for sky in 0..=MAX_LIGHT {
            for block in 0..=MAX_LIGHT {
                let packed = pack_light(sky, block);
                assert_eq!((sky_level(packed), block_level(packed)), (sky, block));
                assert_eq!(Channel::Sky.with(packed, 7), pack_light(7, block));
                assert_eq!(Channel::Block.with(packed, 7), pack_light(sky, 7));
            }
        }
        assert_eq!(pack_light(MAX_LIGHT, 0), MAX_LIGHT);
    }

    #[test]
    fn glowstone_fades_by_one_across_column_border() {
        let (mut world, keys) = dark_hall();
        let mut light = lit(&world, &keys);
        let source = IVec3::new(8, 20, 8);
        place(&mut world, &mut light, source, Block::Glowstone);
        assert_eq!(light.block(source), 15);
        for d in 1..=14 {
            let level = 15 - d as u8;
            // x = 8 + d пересекает границу колонки на d = 8.
            assert_eq!(light.block(source + IVec3::X * d), level, "x {d}");
            assert_eq!(light.block(source + IVec3::NEG_Z * d), level, "z {d}");
        }
        assert_eq!(light.block(source + IVec3::X * 15), 0);
        assert_eq!(light.sky(source), 0);
        assert_matches_reference(&light, &world, &keys, "placed");
    }

    #[test]
    fn breaking_glowstone_removes_its_light() {
        let (mut world, keys) = dark_hall();
        let mut light = lit(&world, &keys);
        let source = IVec3::new(15, 20, 15);
        place(&mut world, &mut light, source, Block::Glowstone);
        place(&mut world, &mut light, source, Block::Air);
        for y in 0..WORLD_HEIGHT {
            for z in -16..32 {
                for x in -16..32 {
                    assert_eq!(light.packed(IVec3::new(x, y, z)), 0, "{x} {y} {z}");
                }
            }
        }
    }

    #[test]
    fn two_sources_do_not_add_up_and_one_can_go() {
        let (mut world, keys) = dark_hall();
        let mut light = lit(&world, &keys);
        let (a, b) = (IVec3::new(10, 20, 8), IVec3::new(12, 20, 8));
        place(&mut world, &mut light, a, Block::Glowstone);
        place(&mut world, &mut light, b, Block::Glowstone);
        // Между ними — максимум, а не сумма: 14 от каждого.
        assert_eq!(light.block(IVec3::new(11, 20, 8)), 14);
        assert_eq!(light.block(a), 15);
        place(&mut world, &mut light, a, Block::Air);
        assert_eq!(light.block(a), 13);
        assert_eq!(light.block(IVec3::new(11, 20, 8)), 14);
        assert_eq!(light.block(b), 15);
        assert_matches_reference(&light, &world, &keys, "one removed");
    }

    #[test]
    fn opaque_block_shadows_and_removing_wall_lets_light_through() {
        let (mut world, keys) = dark_hall();
        let mut light = lit(&world, &keys);
        let source = IVec3::new(2, 20, 8);
        place(&mut world, &mut light, source, Block::Glowstone);
        // Стена поперёк зала (x = 4) на всю высоту зала, кроме проёма никакого нет.
        for y in 10..=40 {
            for z in -16..32 {
                world.set_block(IVec3::new(4, y, z), Block::Stone);
            }
        }
        for y in 10..=40 {
            for z in -16..32 {
                light.block_changed(&world, IVec3::new(4, y, z));
            }
        }
        assert_eq!(light.block(IVec3::new(3, 20, 8)), 14);
        assert_eq!(light.block(IVec3::new(5, 20, 8)), 0);
        assert_matches_reference(&light, &world, &keys, "walled");
        place(&mut world, &mut light, IVec3::new(4, 20, 8), Block::Air);
        assert_eq!(light.block(IVec3::new(4, 20, 8)), 13);
        assert_eq!(light.block(IVec3::new(5, 20, 8)), 12);
        assert_matches_reference(&light, &world, &keys, "opened");
    }

    #[test]
    fn glowstone_light_does_not_depend_on_load_order() {
        let (mut world, keys) = dark_hall();
        for pos in [
            IVec3::new(15, 20, 15),
            IVec3::new(16, 30, 0),
            IVec3::new(-1, 12, 31),
        ] {
            world.set_block(pos, Block::Glowstone);
        }
        let reversed: Vec<IVec2> = keys.iter().rev().copied().collect();
        let a = lit(&world, &keys);
        let b = lit(&world, &reversed);
        assert_matches_reference(&a, &world, &keys, "forward");
        assert_matches_reference(&b, &world, &keys, "reversed");
        // Вынули и вернули колонку, куда светили соседи.
        let mut c = lit(&world, &keys);
        c.remove_column(&world, IVec2::new(1, 1));
        c.add_column(&world, IVec2::new(1, 1));
        assert_matches_reference(&c, &world, &keys, "reloaded");
    }

    #[test]
    fn brightness_is_monotonic_and_bounded() {
        assert_eq!(brightness(MAX_LIGHT), 1.0);
        assert!(brightness(0) >= 0.05);
        // Внизу 0.8^l упирается в минимум, выше — строго растёт.
        for level in 1..=MAX_LIGHT {
            assert!(brightness(level) >= brightness(level - 1), "{level}");
        }
        assert!(brightness(MAX_LIGHT) > brightness(MAX_LIGHT - 1));
        assert!(brightness(2) > brightness(1));
    }
}

#[cfg(test)]
mod bench {
    use super::*;
    use crate::terrain::Terrain;

    /// Среднее время `add_column` на квадрате 12×12 (колонки добавляются по строкам,
    /// как при подгрузке: соседи частью уже есть) и `block_changed`; запуск:
    /// `cargo test --release light_timing -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn light_timing() {
        let terrain = Terrain::new(1337);
        let keys: Vec<IVec2> = (-6..6)
            .flat_map(|z| (-6..6).map(move |x| IVec2::new(x, z)))
            .collect();
        let mut world = World::new();
        for &key in &keys {
            world.insert(key, terrain.generate(key));
        }
        let mut light = LightMap::new();
        let start = std::time::Instant::now();
        for &key in &keys {
            std::hint::black_box(light.add_column(&world, key));
        }
        let per_column = start.elapsed() / keys.len() as u32;
        println!("add_column: {per_column:?} per column");

        let start = std::time::Instant::now();
        let mut edits = 0;
        for x in -40..40 {
            let pos = IVec3::new(x, terrain.surface_height(x, 7), 7);
            world.set_block(pos, Block::Air);
            std::hint::black_box(light.block_changed(&world, pos));
            edits += 1;
        }
        println!("block_changed: {:?} per edit", start.elapsed() / edits);
        println!(
            "memory: {} KiB per column",
            light.memory_bytes() / light.len() / 1024
        );
    }

    /// Худший частый случай правки: пробить кровлю тёмного зала (свет заливает зал)
    /// и заложить обратно (свет снимается). Зал найден для сида 838037976: потолок
    /// над (142, −70) — y 61..=64, над ним небо. Запуск:
    /// `cargo test --release cave_edit_timing -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn cave_edit_timing() {
        let terrain = Terrain::new(838037976);
        let center = column_key(142, -70);
        let mut world = World::new();
        for z in -2..=2 {
            for x in -2..=2 {
                let key = center + IVec2::new(x, z);
                world.insert(key, terrain.generate(key));
            }
        }
        let mut light = LightMap::new();
        for key in world.keys().collect::<Vec<_>>() {
            light.add_column(&world, key);
        }
        let hall = IVec3::new(142, 55, -70);
        assert_eq!(light.sky(hall), 0);
        let roof: Vec<IVec3> = (61..=64).map(|y| IVec3::new(142, y, -70)).collect();
        for (block, label) in [(Block::Air, "open"), (Block::Stone, "close")] {
            let mut order = roof.clone();
            if block == Block::Air {
                order.reverse();
            }
            for pos in order {
                world.set_block(pos, block);
                let start = std::time::Instant::now();
                let changed = light.block_changed(&world, pos);
                println!(
                    "{label} {pos}: {:?}, {} sections, hall {}",
                    start.elapsed(),
                    changed.len(),
                    light.sky(hall)
                );
            }
        }
    }
}
