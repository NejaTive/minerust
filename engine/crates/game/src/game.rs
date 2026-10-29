use std::collections::HashSet;

use glam::{IVec3, Vec3};
use world::block::Block;
use world::column::{Column, WORLD_HEIGHT};
use world::fluid::Fluid;
use world::light::LightMap;
use world::pos::{ColumnPos, SectionPos};
use world::terrain::Terrain;
use world::world::{World, affected_sections};

use crate::camera::Camera;
use crate::daytime::{self, DayTime, SkyState};
use crate::double_tap::DoubleTap;
use crate::physics::{Clock, EYE_HEIGHT, Input, Mode, Player, next_fov_scale};
use crate::raycast::{Hit, raycast};
use crate::repeat::Repeat;

/// Во сколько раз время суток идёт быстрее при перемотке (отладочная клавиша T).
const TIME_FAST_FACTOR: u32 = 60;
/// Дальность действия игрока, блоков от камеры: как в креативе Minecraft.
const REACH: f32 = 6.0;
/// Период повтора ломания и установки при удержании кнопки, с.
const REPEAT_S: f32 = 0.25;
/// Блок в руке до первого выбора слота.
const DEFAULT_BLOCK: Block = Block::Grass;
/// Верхняя граница шага кадра, чтобы после сворачивания вкладки камера не улетала.
/// Заодно ограничивает число шагов физики за кадр: не больше 6.
const MAX_DT: f32 = 0.1;
/// Окно двойного нажатия пробела (полёт ↔ ходьба), с.
const DOUBLE_TAP_S: f32 = 0.3;
/// Шагов физики между шагами жидкости: при 60 тиках в секунду это 4 шага в секунду,
/// вода продвигается на клетку за четверть секунды, как в Minecraft.
const FLUID_STEP_TICKS: u32 = 15;

/// Что изменила правка блоков за кадр; перестраивает меши тот, кто их держит.
#[derive(Default)]
pub struct Edits {
    /// Секции с изменёнными блоками: их мешуют сразу.
    pub dirty: Vec<SectionPos>,
    /// Секции, где изменился только свет, без `dirty`.
    pub lit: Vec<SectionPos>,
}

/// Состояние симуляции: мир, свет, игрок, камера, время суток и ввод. Не знает ни про
/// GPU, ни про подгрузку колонок.
pub struct Game {
    pub world: World,
    /// Солнечный свет загруженных колонок: колонка попадает сюда сразу после загрузки,
    /// так что к её мешу (после загрузки всех соседей) свет окончателен.
    pub light: LightMap,
    /// Очередь клеток течения воды.
    fluid: Fluid,
    /// Шагов физики с прошлого шага жидкости.
    fluid_ticks: u32,
    /// Правки от течения за кадр: уходят вместе с правками игрока в [`Game::interact`],
    /// потому что мешует их тот же кадр.
    flow_dirty: HashSet<SectionPos>,
    flow_lit: Vec<SectionPos>,
    pub terrain: Terrain,
    seed: i32,
    /// Взгляд игрока. Позицию задаёт `player`: каждый кадр камера ставится в его глаза.
    camera: Camera,
    player: Player,
    /// Ноги до последнего шага физики: камера интерполирует от них к `player.feet`.
    prev_feet: Vec3,
    clock: Clock,
    /// Время суток; идёт по шагам физики, на паузе стоит.
    day: DayTime,
    /// Время суток заморожено (режим замера `?bench`).
    time_frozen: bool,
    /// Перемотка времени в [`TIME_FAST_FACTOR`] раз быстрее.
    time_fast: bool,
    space_tap: DoubleTap,
    sprint_tap: DoubleTap,
    /// Множитель угла обзора от бега, 1..=`SPRINT_FOV_SCALE`; настройку в `camera` не меняет.
    fov_scale: f32,
    /// Коды (`KeyboardEvent.code`) зажатых сейчас клавиш.
    keys: HashSet<String>,
    last_time: Option<f32>,
    /// Пауза: физика, ввод и действия мыши стоят, мир подгружается и рисуется.
    paused: bool,
    /// Блок под прицелом в последнем кадре; рамка рисуется вокруг него.
    target: Option<IVec3>,
    breaking: Repeat,
    placing: Repeat,
    /// Блок, который ставит правая кнопка; `None` — пустая рука.
    selected: Option<Block>,
}

impl Game {
    /// `state` — `[x, y, z, yaw, pitch, mode]` из [`Game::camera_state`] (см.
    /// [`restore_player`]); `Err`, если он неверный. Без него игрок встаёт на землю над
    /// точкой (0, 0).
    pub fn new(seed: i32, state: Option<&[f32]>) -> Result<Game, BadCameraState> {
        let terrain = Terrain::new(seed);
        let (camera, player) = match state {
            Some(state) => restore_player(state).ok_or(BadCameraState)?,
            None => {
                // Ноги — на верхнюю грань самого верхнего блока столбца (0, 0): по одной
                // `surface_height` игрок мог бы появиться внутри ствола или кроны, а
                // по твёрдым блокам — на дне озера.
                let spawn = terrain.generate(ColumnPos::ZERO);
                let ground = (0..WORLD_HEIGHT)
                    .rev()
                    .find(|&y| spawn.get(IVec3::new(0, y, 0)) != Block::Air)
                    .unwrap_or(0) as f32;
                let player = Player::new(Vec3::new(0.5, ground + 1.0, 0.5), Mode::Walking);
                (Camera::spawn(player.eye()), player)
            }
        };
        Ok(Game {
            world: World::new(),
            light: LightMap::new(),
            fluid: Fluid::new(),
            fluid_ticks: 0,
            flow_dirty: HashSet::new(),
            flow_lit: Vec::new(),
            terrain,
            seed,
            camera,
            prev_feet: player.feet,
            player,
            clock: Clock::default(),
            day: DayTime::default(),
            time_frozen: false,
            time_fast: false,
            space_tap: DoubleTap::new(DOUBLE_TAP_S),
            sprint_tap: DoubleTap::new(DOUBLE_TAP_S),
            fov_scale: 1.0,
            keys: HashSet::new(),
            last_time: None,
            paused: false,
            target: None,
            breaking: Repeat::new(REPEAT_S),
            placing: Repeat::new(REPEAT_S),
            selected: Some(DEFAULT_BLOCK),
        })
    }

    pub fn seed(&self) -> i32 {
        self.seed
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    pub fn player(&self) -> &Player {
        &self.player
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn fov_scale(&self) -> f32 {
        self.fov_scale
    }

    /// Ключ колонки, над которой стоит камера.
    pub fn camera_column(&self) -> ColumnPos {
        let p = self.camera.position.floor().as_ivec3();
        ColumnPos::of_block(p)
    }

    /// Состояние `[x, y, z, yaw, pitch, mode]` для передачи в новый `Game`: x, y, z —
    /// глаза игрока, `mode` — 0 ходьба, 1 полёт, 2 полёт сквозь блоки. Скорость не
    /// передаётся.
    pub fn camera_state(&self) -> Vec<f32> {
        let eye = self.player.eye();
        let mode = match self.player.mode {
            Mode::Walking => 0.0,
            Mode::Flying => 1.0,
            Mode::Noclip => 2.0,
        };
        vec![
            eye.x,
            eye.y,
            eye.z,
            self.camera.yaw,
            self.camera.pitch,
            mode,
        ]
    }

    /// Ставит игрока и взгляд из `state`; `Err`, если он неверный (мир не меняется).
    pub fn set_camera_state(&mut self, state: &[f32]) -> Result<(), BadCameraState> {
        let (camera, player) = restore_player(state).ok_or(BadCameraState)?;
        self.camera.position = camera.position;
        self.camera.yaw = camera.yaw;
        self.camera.pitch = camera.pitch;
        self.prev_feet = player.feet;
        self.player = player;
        self.clock.reset();
        self.target = None;
        Ok(())
    }

    // --- время суток ---

    pub fn time_of_day(&self) -> f32 {
        self.day.fraction()
    }

    /// Ставит время суток; вне 0..1 берётся дробная часть, не число — утро.
    pub fn set_time_of_day(&mut self, time: f32) {
        self.day = DayTime::from_fraction(time);
    }

    pub fn day_phase(&self) -> &'static str {
        daytime::phase(self.day.fraction()).name()
    }

    pub fn set_time_frozen(&mut self, frozen: bool) {
        self.time_frozen = frozen;
    }

    pub fn set_time_fast(&mut self, fast: bool) {
        self.time_fast = fast;
    }

    pub fn sky(&self) -> SkyState {
        daytime::sky_state(self.day.fraction())
    }

    pub fn sun_direction(&self) -> [f32; 3] {
        daytime::sun_direction(self.day.fraction())
    }

    pub fn star_rotation(&self) -> [f32; 2] {
        daytime::star_rotation(self.day.fraction())
    }

    /// Глаза игрока под водой.
    pub fn underwater(&self) -> bool {
        self.player
            .eyes_in_water(|pos| self.world.water_height(pos))
    }

    // --- ввод ---

    /// Пауза: физика, поворот камеры, ввод и действия мыши стоят. При снятии паузы
    /// накопитель физики обнуляется, чтобы игрок не «догонял» простой. Включение паузы
    /// отпускает клавиши и кнопки мыши.
    pub fn set_paused(&mut self, paused: bool) {
        if paused == self.paused {
            return;
        }
        self.paused = paused;
        if paused {
            self.release_all_keys();
        } else {
            self.clock.reset();
            // Иначе камера на первом кадре перескочила бы от ног прошлого шага к текущим.
            self.prev_feet = self.player.feet;
        }
    }

    pub fn mouse_move(&mut self, dx: f32, dy: f32) {
        if !self.paused {
            self.camera.rotate(dx, dy);
        }
    }

    /// Нажатие/отпускание клавиши; `code` — `KeyboardEvent.code`. Двойной Space переключает
    /// ходьбу и полёт, двойной W включает бег.
    pub fn set_key(&mut self, code: &str, pressed: bool) {
        if pressed {
            if self.paused {
                return;
            }
            // Повтор нажатия без отпускания (автоповтор клавиатуры) — не новое нажатие.
            let fresh = self.keys.insert(code.to_owned());
            // Время прошлого кадра, а не события: точность в кадр для окна 0.3 с хватает.
            if fresh && code == "Space" && self.space_tap.press(self.last_time.unwrap_or(0.0)) {
                self.player.toggle_mode();
            }
            if fresh && code == "KeyW" && self.sprint_tap.press(self.last_time.unwrap_or(0.0)) {
                self.player.start_sprint();
            }
        } else {
            self.keys.remove(code);
        }
    }

    /// Включает отладочный полёт сквозь блоки или возвращает из него. На паузе не действует.
    pub fn toggle_noclip(&mut self) {
        if !self.paused {
            self.player.toggle_noclip();
        }
    }

    /// Кнопка мыши: `button` — `MouseEvent.button` (0 — левая ломает, 2 — правая ставит),
    /// остальные игнорируются; при удержании действие повторяется раз в [`REPEAT_S`].
    pub fn mouse_button(&mut self, button: u8, pressed: bool) {
        if self.paused && pressed {
            return;
        }
        match button {
            0 => self.breaking.set(pressed),
            2 => self.placing.set(pressed),
            _ => {}
        }
    }

    /// Блок для установки по имени; пустая строка — пустая рука. `false` для незнакомого имени.
    pub fn select_block(&mut self, name: &str) -> bool {
        if name.is_empty() {
            self.selected = None;
            return true;
        }
        match Block::from_name(name) {
            Some(block) => {
                self.selected = Some(block);
                true
            }
            None => false,
        }
    }

    /// Отпускает все клавиши и кнопки мыши (потеря pointer lock, blur окна).
    pub fn release_all_keys(&mut self) {
        self.keys.clear();
        // Без зажатой W бег всё равно кончился бы на первом шаге, но флаг видит HUD и угол обзора.
        self.player.sprinting = false;
        self.breaking.reset();
        self.placing.reset();
    }

    // --- кадр ---

    /// Начало кадра: `time` — секунды с начала работы страницы. Шаги физики и камера в
    /// интерполированных глазах игрока (на паузе стоят), множитель угла обзора от бега.
    /// Возвращает шаг кадра `dt`.
    pub fn update(&mut self, time: f32) -> f32 {
        let dt = self
            .last_time
            .map_or(0.0, |last| (time - last).clamp(0.0, MAX_DT));
        self.last_time = Some(time);
        // На паузе камера и цель под прицелом остаются как были: мир только достраивается.
        if !self.paused {
            self.update_player(dt);
        }
        self.fov_scale = next_fov_scale(self.fov_scale, self.player.sprinting, dt);
        dt
    }

    /// Ломание и установка по кнопкам мыши. Перестраивать меши задетых секций надо в
    /// этом же кадре — вне бюджета подгрузки, чтобы правка была видна сразу.
    pub fn interact(&mut self, time: f32) -> Edits {
        let mut dirty = std::mem::take(&mut self.flow_dirty);
        let mut lit = std::mem::take(&mut self.flow_lit);
        if self.paused {
            return Edits {
                dirty: dirty.into_iter().collect(),
                lit,
            };
        }
        let mut target = self.pick();
        // `fire` вызывается каждый кадр, даже без цели: так при удержании темп не сбивается.
        if self.breaking.fire(time)
            && let Some(hit) = target
            // Ниже y = 0 — дно мира: `World::block` считает его камнем, и луч в него
            // попадает, но ломать его нельзя.
            && hit.block.y >= 0
            && let changed = self.world.set_block_supported(hit.block, Block::Air)
            && !changed.is_empty()
        {
            self.apply_edit(&changed, &mut dirty, &mut lit);
            target = self.pick();
        }
        if self.placing.fire(time)
            && let Some(hit) = target
            && let Some(block) = self.selected
        {
            let pos = hit.block + hit.normal;
            // Данные (ось бревна, сторона факела) выбирает грань, в которую смотрит
            // прицел; `None` — на такую грань блок не ставится.
            // `set_block` сам отказывает вне высоты мира и в незагруженной колонке.
            if let Some(data) = block.placement_data(hit.normal)
                && self.world.can_place_with_data(pos, block, data)
                && (block.is_water() || !self.player.overlaps_block(pos))
            {
                let changed = self.world.set_block_supported_with_data(pos, block, data);
                if !changed.is_empty() {
                    self.apply_edit(&changed, &mut dirty, &mut lit);
                    target = self.pick();
                }
            }
        }
        self.target = target.map(|hit| hit.block);
        lit.retain(|key| !dirty.contains(key));
        Edits {
            dirty: dirty.into_iter().collect(),
            lit,
        }
    }

    /// Клетка под прицелом, блок в ней и его данные; рамка рисуется вокруг него.
    pub fn target_block(&self) -> Option<(IVec3, Block, u8)> {
        self.target
            .map(|pos| (pos, self.world.block(pos), self.world.block_data(pos)))
    }

    /// Импорт сохранённой колонки: свет есть только у загруженной колонки, и
    /// `add_column` пересчитывает его заново вместе с соседями.
    pub fn import_column(&mut self, key: ColumnPos, column: Column) {
        self.world.import(key, column);
        if self.light.column(key).is_some() {
            self.light.add_column(&self.world, key);
        }
    }

    /// Блок под прицелом: луч из камеры вдоль взгляда не дальше [`REACH`].
    fn pick(&self) -> Option<Hit> {
        raycast(self.camera.position, self.camera.forward(), REACH, |pos| {
            self.world.block(pos).is_targetable()
        })
    }

    /// Собирает секции и свет по клеткам правки (`World::set_block_supported`) и ставит
    /// клетки в очередь течения.
    fn apply_edit(
        &mut self,
        changed: &[IVec3],
        dirty: &mut HashSet<SectionPos>,
        lit: &mut Vec<SectionPos>,
    ) {
        for &pos in changed {
            self.fluid.block_changed(&self.world, pos);
            dirty.extend(affected_sections(pos));
            lit.extend(self.light.block_changed(&self.world, pos));
        }
    }

    /// Шаги физики за время кадра `dt` и камера в интерполированных глазах игрока.
    fn update_player(&mut self, dt: f32) {
        let input = self.input();
        let ticks = self.clock.advance(dt);
        if !self.time_frozen {
            let factor = if self.time_fast { TIME_FAST_FACTOR } else { 1 };
            self.day.advance(ticks * factor);
        }
        self.advance_fluid(ticks);
        for _ in 0..ticks {
            if !self.body_loaded() {
                // Под незагруженной колонкой `World::block` — воздух, и игрок провалился
                // бы сквозь ещё не сгенерированную землю. Стоим, пока она не появится;
                // скорость сохраняется, чтобы падение продолжилось как было.
                self.clock.reset();
                self.prev_feet = self.player.feet;
                break;
            }
            self.prev_feet = self.player.feet;
            self.player.step(
                &input,
                self.camera.yaw,
                |pos| self.world.block(pos).is_solid(),
                |pos| self.world.water_height(pos),
            );
        }
        let feet = self.prev_feet.lerp(self.player.feet, self.clock.alpha());
        self.camera.position = feet + Vec3::Y * EYE_HEIGHT;
    }

    /// Шаги жидкости за `ticks` шагов физики; правки копятся до [`Game::interact`].
    fn advance_fluid(&mut self, ticks: u32) {
        self.fluid_ticks += ticks;
        while self.fluid_ticks >= FLUID_STEP_TICKS {
            self.fluid_ticks -= FLUID_STEP_TICKS;
            let changed = self.fluid.step(&mut self.world);
            let (mut dirty, mut lit) = (
                std::mem::take(&mut self.flow_dirty),
                std::mem::take(&mut self.flow_lit),
            );
            self.apply_edit(&changed, &mut dirty, &mut lit);
            (self.flow_dirty, self.flow_lit) = (dirty, lit);
        }
    }

    /// Намерения игрока по зажатым клавишам.
    fn input(&self) -> Input {
        let pressed = |code: &str| self.keys.contains(code);
        let axis = |plus: &str, minus: &str| {
            f32::from(u8::from(pressed(plus))) - f32::from(u8::from(pressed(minus)))
        };
        Input {
            forward: axis("KeyW", "KeyS"),
            strafe: axis("KeyD", "KeyA"),
            jump: pressed("Space"),
            descend: pressed("ShiftLeft") || pressed("ShiftRight"),
        }
    }

    /// Загружены ли все колонки, которые тело может задеть за шаг. Запас в блок по
    /// горизонтали с лихвой покрывает шаг полёта (8 / 60 блока).
    fn body_loaded(&self) -> bool {
        let (min, max) = self.player.aabb();
        let lo = (min - Vec3::ONE).floor().as_ivec3();
        let hi = (max + Vec3::ONE).floor().as_ivec3();
        let (lo, hi) = (ColumnPos::of_block(lo), ColumnPos::of_block(hi));
        (lo.x()..=hi.x())
            .all(|x| (lo.z()..=hi.z()).all(|z| self.world.column(ColumnPos::new(x, z)).is_some()))
    }
}

/// Состояние камеры не 5 или 6 конечных чисел.
#[derive(Debug)]
pub struct BadCameraState;

/// Камера и игрок из [`Game::camera_state`]: `[x, y, z, yaw, pitch]` и необязательный
/// `mode`. `None`, если длина другая или есть не-числа.
fn restore_player(state: &[f32]) -> Option<(Camera, Player)> {
    let (view, mode) = match state {
        [view @ .., mode] if state.len() == 6 => (view, Some(*mode)),
        _ => (state, None),
    };
    let camera = Camera::from_state(view)?;
    let mode = match mode {
        None => Mode::Walking,
        Some(1.0) => Mode::Flying,
        Some(2.0) => Mode::Noclip,
        Some(m) if m.is_finite() => Mode::Walking,
        Some(_) => return None,
    };
    let player = Player::new(camera.position - Vec3::Y * EYE_HEIGHT, mode);
    Some((camera, player))
}
