use glam::{IVec3, Vec2, Vec3};

pub const PLAYER_WIDTH: f32 = 0.6;
pub const PLAYER_HEIGHT: f32 = 1.8;
pub const EYE_HEIGHT: f32 = 1.62;
/// Фиксированный шаг физики, с. Шаг не зависит от частоты кадров: высота прыжка
/// и путь за секунду одинаковы при любом FPS, а плавность даёт интерполяция.
pub const TICK: f32 = 1.0 / 60.0;

/// Ускорение свободного падения, бл/с²: падение ощущается как в Minecraft (~32).
pub const GRAVITY: f32 = 32.0;
/// Предельная скорость падения, бл/с.
pub const TERMINAL_SPEED: f32 = 78.0;
/// Начальная скорость прыжка, бл/с. В непрерывной модели высота v²/2g ≈ 1.27, при шаге
/// 1/60 с — ≈ 1.19: на блок высотой 1 запрыгнуть можно, на 2 — нет.
pub const JUMP_SPEED: f32 = 9.0;
/// Скорость ходьбы, бл/с, как у Minecraft.
pub const WALK_SPEED: f32 = 4.3;
/// Скорость полёта, бл/с: как у прежней свободной камеры.
pub const FLY_SPEED: f32 = 8.0;
/// Бег: множитель скорости ходьбы. Как в Minecraft (×1.3).
pub const SPRINT_FACTOR: f32 = 1.3;
/// Бег в полёте: множитель скорости полёта.
pub const FLY_SPRINT_FACTOR: f32 = 2.0;
/// Во сколько раз бег раздвигает угол обзора: на 10% от настройки, чтобы скорость ощущалась.
pub const SPRINT_FOV_SCALE: f32 = 1.1;
/// За сколько секунд угол обзора доходит до значения бега и возвращается обратно.
pub const SPRINT_FOV_TIME: f32 = 0.15;
/// Управление в воздухе, бл/с²: горизонтальная скорость тянется к желаемой медленно,
/// поэтому прыжок с разбега сохраняет инерцию, а с места до полной скорости не разогнаться.
pub const AIR_ACCEL: f32 = 10.0;

/// Плавание. Гравитация в воде вчетверо слабее, а тонуть быстрее предела не выйдет:
/// игрок медленно опускается, а не падает.
pub const WATER_GRAVITY: f32 = GRAVITY / 4.0;
/// Предельная скорость погружения, бл/с.
pub const WATER_SINK_SPEED: f32 = 2.0;
/// Скорость всплытия с зажатым Space, бл/с, и разгон до неё, бл/с².
pub const SWIM_UP_SPEED: f32 = 3.0;
const SWIM_UP_ACCEL: f32 = 24.0;
/// Скорость плавания по горизонтали, бл/с: вдвое медленнее ходьбы.
pub const SWIM_SPEED: f32 = WALK_SPEED * 0.5;
/// Горизонтальный разгон и торможение в воде, бл/с²: за ~0.1 с до `SWIM_SPEED`.
const WATER_ACCEL: f32 = 20.0;
/// Доля вертикальной скорости сверх предела, которая остаётся за шаг: прыжок в воду
/// с высоты гасится за доли секунды, а не мгновенно. 0.8 — как в Minecraft за тик 1/20 с;
/// при 60 шагах в секунду это 0.8^(1/3).
const WATER_DRAG: f32 = 0.928;
/// Всплывать Space позволяет, пока ноги глубже этого под поверхностью: игрок держится
/// на воде по грудь, а не встаёт на неё.
pub const SWIM_DEPTH: f32 = 0.4;
/// Толчок вверх, когда в воде с зажатым Space игрок упёрся в стену: так он выбирается
/// на берег на блок выше воды. Чуть больше прыжка: первые `SWIM_DEPTH` блока подъём
/// идёт ещё в воде.
pub const WATER_EXIT_SPEED: f32 = 10.0;
/// Толчок — только у поверхности: глубже у стены игрок всплывает обычной скоростью.
const WATER_EXIT_DEPTH: f32 = 1.0;

/// Запас в сравнениях с гранями блоков. Без него стоящий на полу игрок задевал бы
/// пол при движении вбок, а прижатый к стене — стену при падении. 1e-3, а не меньше:
/// у f32 на координатах ~8000 шаг уже ~1e-3, и «ровно на грани» перестаёт быть ровно.
const EPS: f32 = 1e-3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Walking,
    Flying,
    /// Отладочный полёт сквозь блоки: без коллизий и воды, на землю не садится.
    Noclip,
}

/// Намерения игрока на шаг: оси в пространстве взгляда (forward/strafe в −1..1),
/// кнопки удерживаются.
#[derive(Clone, Copy, Default, Debug)]
pub struct Input {
    pub forward: f32,
    pub strafe: f32,
    pub jump: bool,
    pub descend: bool,
}

pub struct Player {
    /// Центр нижней грани тела.
    pub feet: Vec3,
    /// Скорость, бл/с.
    pub velocity: Vec3,
    pub mode: Mode,
    /// В последнем шаге движение вниз упёрлось в блок. В полёте без спуска — `false`.
    pub on_ground: bool,
    /// Бег включён. Выключает его сама физика: W отпущена, упор в стену, вход в воду.
    pub sprinting: bool,
}

impl Player {
    pub fn new(feet: Vec3, mode: Mode) -> Self {
        Self {
            feet,
            velocity: Vec3::ZERO,
            mode,
            on_ground: false,
            sprinting: false,
        }
    }

    /// Включает бег; он действует, пока [`Player::step`] не встретит условие выхода.
    pub fn start_sprint(&mut self) {
        self.sprinting = true;
    }

    /// Тело в мировых координатах: (min, max).
    pub fn aabb(&self) -> (Vec3, Vec3) {
        let half = PLAYER_WIDTH / 2.0;
        (
            self.feet - Vec3::new(half, 0.0, half),
            self.feet + Vec3::new(half, PLAYER_HEIGHT, half),
        )
    }

    pub fn eye(&self) -> Vec3 {
        self.feet + Vec3::Y * EYE_HEIGHT
    }

    /// Переключает ходьбу и полёт. В полёт — без вертикальной скорости, чтобы игрок
    /// завис там, где был; из полёта гравитация подхватывает его со следующего шага.
    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            Mode::Walking => {
                self.velocity.y = 0.0;
                Mode::Flying
            }
            Mode::Flying | Mode::Noclip => Mode::Walking,
        };
    }

    /// Включает полёт сквозь блоки или возвращает из него в обычный полёт.
    pub fn toggle_noclip(&mut self) {
        self.mode = match self.mode {
            Mode::Noclip => Mode::Flying,
            Mode::Walking | Mode::Flying => {
                self.velocity.y = 0.0;
                Mode::Noclip
            }
        };
    }

    /// Пересекает ли блок `pos` тело игрока; касание гранями не считается.
    pub fn overlaps_block(&self, pos: IVec3) -> bool {
        let (min, max) = self.aabb();
        let block_min = pos.as_vec3();
        let block_max = block_min + Vec3::ONE;
        (min + EPS).cmplt(block_max).all() && (max - EPS).cmpgt(block_min).all()
    }

    /// На сколько блоков ноги ниже поверхности воды, которой касается тело; 0 — тело
    /// не в воде. Поверхность — верх самого высокого блока воды в теле. `water_height` —
    /// высота воды в клетке над её низом (`World::water_height`), `None` — воды нет.
    pub fn water_depth(&self, water_height: impl Fn(IVec3) -> Option<f32>) -> f32 {
        let (min, max) = self.aabb();
        let lo = (min + EPS).floor().as_ivec3();
        let hi = (max - EPS).floor().as_ivec3();
        let mut surface = None;
        for y in lo.y..=hi.y {
            for z in lo.z..=hi.z {
                for x in lo.x..=hi.x {
                    let pos = IVec3::new(x, y, z);
                    if let Some(height) = water_height(pos) {
                        let top = y as f32 + height;
                        surface = Some(surface.map_or(top, |s: f32| s.max(top)));
                    }
                }
            }
        }
        surface.map_or(0.0, |s| (s - self.feet.y).max(0.0))
    }

    /// В воде ли глаза: в клетке воды и ниже её поверхности.
    pub fn eyes_in_water(&self, water_height: impl Fn(IVec3) -> Option<f32>) -> bool {
        let eye = self.eye();
        let cell = eye.floor().as_ivec3();
        water_height(cell).is_some_and(|height| eye.y < cell.y as f32 + height)
    }

    /// Один шаг физики длиной [`TICK`]. `yaw` — направление взгляда по горизонтали
    /// (как у камеры: 0 — вдоль +X). В полёте вода не мешает.
    pub fn step(
        &mut self,
        input: &Input,
        yaw: f32,
        is_solid: impl Fn(IVec3) -> bool,
        water_height: impl Fn(IVec3) -> Option<f32>,
    ) {
        let (sin, cos) = yaw.sin_cos();
        let forward = Vec3::new(cos, 0.0, sin);
        let right = Vec3::new(-sin, 0.0, cos);
        // Ограничиваем длину, чтобы по диагонали не идти быстрее.
        let wish = (forward * input.forward + right * input.strafe).clamp_length_max(1.0);
        let vertical = f32::from(u8::from(input.jump)) - f32::from(u8::from(input.descend));

        let depth = match self.mode {
            Mode::Walking => self.water_depth(&water_height),
            Mode::Flying | Mode::Noclip => 0.0,
        };
        // В полёте вода не мешает, поэтому и бег в ней не прерывается (depth там 0).
        if input.forward <= 0.0 || depth > 0.0 {
            self.sprinting = false;
        }
        match self.mode {
            Mode::Walking if depth > 0.0 => {
                let target = Vec2::new(wish.x, wish.z) * SWIM_SPEED;
                let horizontal = Vec2::new(self.velocity.x, self.velocity.z)
                    .move_towards(target, WATER_ACCEL * TICK);
                self.velocity.x = horizontal.x;
                self.velocity.z = horizontal.y;
                let mut vy = self.velocity.y - WATER_GRAVITY * TICK;
                if input.jump && depth > SWIM_DEPTH && vy < SWIM_UP_SPEED {
                    vy = (vy + SWIM_UP_ACCEL * TICK).min(SWIM_UP_SPEED);
                }
                if vy < -WATER_SINK_SPEED {
                    vy = (vy * WATER_DRAG).min(-WATER_SINK_SPEED);
                }
                self.velocity.y = vy;
            }
            Mode::Walking => {
                let speed = if self.sprinting {
                    WALK_SPEED * SPRINT_FACTOR
                } else {
                    WALK_SPEED
                };
                let target = Vec2::new(wish.x, wish.z) * speed;
                // На земле скорость меняется сразу (без скольжения), в воздухе — медленно.
                let horizontal = if self.on_ground {
                    target
                } else {
                    Vec2::new(self.velocity.x, self.velocity.z)
                        .move_towards(target, AIR_ACCEL * TICK)
                };
                self.velocity.x = horizontal.x;
                self.velocity.z = horizontal.y;
                if self.on_ground && input.jump {
                    self.velocity.y = JUMP_SPEED;
                }
                self.velocity.y = (self.velocity.y - GRAVITY * TICK).max(-TERMINAL_SPEED);
            }
            Mode::Flying | Mode::Noclip => {
                let speed = if self.sprinting {
                    FLY_SPEED * FLY_SPRINT_FACTOR
                } else {
                    FLY_SPEED
                };
                self.velocity = (wish + Vec3::Y * vertical).clamp_length_max(1.0) * speed;
            }
        }

        // По осям отдельно: так упор в стену гасит только скорость в стену, а вдоль
        // неё игрок скользит. Y первым — тогда вбок двигается уже стоящее на полу тело.
        let delta = self.velocity * TICK;
        self.on_ground = false;
        if self.mode == Mode::Noclip {
            self.feet += delta;
            return;
        }
        let mut against_wall = false;
        for axis in [1, 0, 2] {
            let blocked = self.move_axis(axis, delta[axis], &is_solid);
            against_wall |= blocked && axis != 1;
        }
        if against_wall {
            self.sprinting = false;
        }
        if depth > 0.0 && depth < WATER_EXIT_DEPTH && input.jump && against_wall {
            self.velocity.y = self.velocity.y.max(WATER_EXIT_SPEED);
        }
        // Приземление в полёте — только когда спуск упёрся в землю. Не «стоит на земле»:
        // полёт, включённый двойным Space без прыжка (оба нажатия короче кадра), начинается
        // на земле, и такое правило выключало бы его на первом же шаге.
        if self.mode == Mode::Flying && self.on_ground {
            self.mode = Mode::Walking;
        }
    }

    /// Сдвигает тело вдоль оси `axis` на `delta`, останавливаясь у первого твёрдого блока;
    /// `true`, если упёрлось.
    ///
    /// Слои блоков, в которые входит передняя грань, проверяются по порядку от ближнего,
    /// а не только конечная позиция: так на любой скорости (78 бл/с — это 1.3 блока за
    /// шаг) тело не проскакивает сквозь пол толщиной в блок.
    fn move_axis(&mut self, axis: usize, delta: f32, is_solid: &impl Fn(IVec3) -> bool) -> bool {
        if delta == 0.0 {
            return false;
        }
        let (min, max) = self.aabb();
        // Протяжённость тела от ног вниз и вверх по оси: ноги ставятся от неё, а не от
        // разности min/max и feet, чтобы позиция у стены была точной, без ошибки округления.
        let (below, above) = if axis == 1 {
            (0.0, PLAYER_HEIGHT)
        } else {
            (PLAYER_WIDTH / 2.0, PLAYER_WIDTH / 2.0)
        };
        // Поперечное сечение тела по двум другим осям, сжатое на EPS: блоки, которых
        // тело только касается гранью, не мешают.
        let lo = (min + EPS).floor().as_ivec3();
        let hi = (max - EPS).floor().as_ivec3();
        let blocked = |layer: i32| {
            let mut pos = lo;
            pos[axis] = layer;
            let (a, b) = match axis {
                0 => (1, 2),
                1 => (0, 2),
                _ => (0, 1),
            };
            (lo[a]..=hi[a]).any(|i| {
                (lo[b]..=hi[b]).any(|j| {
                    pos[a] = i;
                    pos[b] = j;
                    is_solid(pos)
                })
            })
        };

        if delta > 0.0 {
            let first = (max[axis] - EPS).ceil() as i32;
            let last = (max[axis] + delta - EPS).floor() as i32;
            if let Some(layer) = (first..=last).find(|&layer| blocked(layer)) {
                self.feet[axis] = layer as f32 - above;
                self.velocity[axis] = 0.0;
                return true;
            }
        } else {
            let first = (min[axis] + EPS).floor() as i32 - 1;
            let last = (min[axis] + delta + EPS).floor() as i32;
            if let Some(layer) = (last..=first).rev().find(|&layer| blocked(layer)) {
                self.feet[axis] = (layer + 1) as f32 + below;
                self.velocity[axis] = 0.0;
                if axis == 1 {
                    self.on_ground = true;
                }
                return true;
            }
        }
        self.feet[axis] += delta;
        false
    }
}

/// Множитель угла обзора на следующий кадр: идёт к [`SPRINT_FOV_SCALE`] при беге и к 1 иначе
/// с постоянной скоростью, так что весь путь занимает [`SPRINT_FOV_TIME`].
pub fn next_fov_scale(scale: f32, sprinting: bool, dt: f32) -> f32 {
    let target = if sprinting { SPRINT_FOV_SCALE } else { 1.0 };
    let step = (SPRINT_FOV_SCALE - 1.0) * dt / SPRINT_FOV_TIME;
    if scale < target {
        (scale + step).min(target)
    } else {
        (scale - step).max(target)
    }
}

/// Накопитель времени кадров для фиксированного шага [`TICK`].
#[derive(Default)]
pub struct Clock {
    /// Время, ещё не отработанное шагами, с; всегда в 0..TICK.
    acc: f32,
}

impl Clock {
    /// Добавляет время кадра `dt` (с) и возвращает, сколько шагов физики сделать.
    pub fn advance(&mut self, dt: f32) -> u32 {
        self.acc += dt;
        let ticks = (self.acc / TICK).floor() as u32;
        self.acc = (self.acc - ticks as f32 * TICK).max(0.0);
        ticks
    }

    /// Доля шага, прошедшая после последнего шага, в 0..1: вес для интерполяции
    /// между прежним и текущим положением.
    pub fn alpha(&self) -> f32 {
        (self.acc / TICK).min(1.0)
    }

    pub fn reset(&mut self) {
        self.acc = 0.0;
    }
}

#[cfg(test)]
mod tests;
