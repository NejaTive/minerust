use glam::{IVec3, Vec2, Vec3};

use world::block::WATER_SURFACE;

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
    /// не в воде. Поверхность — верх самого высокого блока воды в теле, у верхнего
    /// блока воды — [`WATER_SURFACE`].
    pub fn water_depth(&self, is_water: impl Fn(IVec3) -> bool) -> f32 {
        let (min, max) = self.aabb();
        let lo = (min + EPS).floor().as_ivec3();
        let hi = (max - EPS).floor().as_ivec3();
        let mut surface = None;
        for y in lo.y..=hi.y {
            for z in lo.z..=hi.z {
                for x in lo.x..=hi.x {
                    let pos = IVec3::new(x, y, z);
                    if is_water(pos) {
                        let top = y as f32 + water_height(pos, &is_water);
                        surface = Some(surface.map_or(top, |s: f32| s.max(top)));
                    }
                }
            }
        }
        surface.map_or(0.0, |s| (s - self.feet.y).max(0.0))
    }

    /// В воде ли глаза: в клетке воды и ниже её поверхности.
    pub fn eyes_in_water(&self, is_water: impl Fn(IVec3) -> bool) -> bool {
        let eye = self.eye();
        let cell = eye.floor().as_ivec3();
        is_water(cell) && eye.y < cell.y as f32 + water_height(cell, &is_water)
    }

    /// Один шаг физики длиной [`TICK`]. `yaw` — направление взгляда по горизонтали
    /// (как у камеры: 0 — вдоль +X). В полёте вода не мешает.
    pub fn step(
        &mut self,
        input: &Input,
        yaw: f32,
        is_solid: impl Fn(IVec3) -> bool,
        is_water: impl Fn(IVec3) -> bool,
    ) {
        let (sin, cos) = yaw.sin_cos();
        let forward = Vec3::new(cos, 0.0, sin);
        let right = Vec3::new(-sin, 0.0, cos);
        // Ограничиваем длину, чтобы по диагонали не идти быстрее.
        let wish = (forward * input.forward + right * input.strafe).clamp_length_max(1.0);
        let vertical = f32::from(u8::from(input.jump)) - f32::from(u8::from(input.descend));

        let depth = match self.mode {
            Mode::Walking => self.water_depth(&is_water),
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

/// Высота воды в клетке `pos` над её низом: полная, если над ней тоже вода.
fn water_height(pos: IVec3, is_water: &impl Fn(IVec3) -> bool) -> f32 {
    if is_water(pos + IVec3::Y) {
        1.0
    } else {
        WATER_SURFACE
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
mod tests {
    use std::collections::HashSet;
    use std::f32::consts::FRAC_PI_2;
    use std::f32::consts::FRAC_PI_4;

    use super::*;

    const FLOOR: i32 = 63;
    const GROUND: f32 = (FLOOR + 1) as f32;

    /// Мир: пол — всё с y ≤ FLOOR, плюс отдельные блоки.
    fn world(blocks: &[IVec3]) -> impl Fn(IVec3) -> bool + use<> {
        let blocks: HashSet<IVec3> = blocks.iter().copied().collect();
        move |pos| pos.y <= FLOOR || blocks.contains(&pos)
    }

    /// Стена в плоскости x = `x` от пола до y = 80 в пределах z −5..5.
    fn wall_x(x: i32) -> Vec<IVec3> {
        (GROUND as i32..80)
            .flat_map(|y| (-5..5).map(move |z| IVec3::new(x, y, z)))
            .collect()
    }

    fn wall_z(z: i32) -> Vec<IVec3> {
        (GROUND as i32..80)
            .flat_map(|y| (-5..5).map(move |x| IVec3::new(x, y, z)))
            .collect()
    }

    fn standing() -> Player {
        let mut player = Player::new(Vec3::new(0.5, GROUND, 0.5), Mode::Walking);
        player.on_ground = true;
        player
    }

    const FORWARD: Input = Input {
        forward: 1.0,
        strafe: 0.0,
        jump: false,
        descend: false,
    };

    fn run(
        player: &mut Player,
        input: Input,
        yaw: f32,
        ticks: u32,
        solid: &impl Fn(IVec3) -> bool,
    ) {
        for _ in 0..ticks {
            player.step(&input, yaw, solid, dry);
        }
    }

    #[test]
    fn stands_on_flat_floor() {
        let solid = world(&[]);
        let mut player = standing();
        run(&mut player, Input::default(), 0.0, 120, &solid);
        assert_eq!(player.feet.y, GROUND);
        assert!(player.on_ground);
    }

    #[test]
    fn fall_lands_exactly_on_surface() {
        let solid = world(&[]);
        let mut player = Player::new(Vec3::new(0.5, GROUND + 50.0, 0.5), Mode::Walking);
        let mut ticks = 0;
        while !player.on_ground {
            player.step(&Input::default(), 0.0, &solid, dry);
            ticks += 1;
            assert!(ticks < 1000, "не приземлился");
            assert!(player.feet.y >= GROUND);
        }
        assert_eq!(player.feet.y, GROUND);
        assert_eq!(player.velocity.y, 0.0);
    }

    #[test]
    fn terminal_speed_does_not_pass_thin_floor() {
        // Единственный пол — слой y = 10 толщиной в блок.
        let solid = |pos: IVec3| pos.y == 10;
        let mut player = Player::new(Vec3::new(0.5, 200.0, 0.5), Mode::Walking);
        player.velocity.y = -TERMINAL_SPEED;
        run(&mut player, Input::default(), 0.0, 600, &solid);
        assert_eq!(player.feet.y, 11.0);
        assert!(player.on_ground);
    }

    #[test]
    fn fall_speed_is_limited() {
        let solid = |_: IVec3| false;
        let mut player = Player::new(Vec3::new(0.5, 10_000.0, 0.5), Mode::Walking);
        run(&mut player, Input::default(), 0.0, 600, &solid);
        assert_eq!(player.velocity.y, -TERMINAL_SPEED);
    }

    #[test]
    fn jump_height_between_one_and_two_blocks() {
        let solid = world(&[]);
        let mut player = standing();
        let jump = Input {
            jump: true,
            ..Input::default()
        };
        player.step(&jump, 0.0, &solid, dry);
        let mut top = player.feet.y;
        for _ in 0..120 {
            player.step(&Input::default(), 0.0, &solid, dry);
            top = top.max(player.feet.y);
        }
        let height = top - GROUND;
        assert!(height > 1.0 && height < 2.0, "высота прыжка {height}");
        assert_eq!(player.feet.y, GROUND);
    }

    #[test]
    fn jumps_onto_one_block_but_not_two() {
        let input = Input {
            jump: true,
            ..FORWARD
        };
        let step: Vec<IVec3> = (2..20)
            .flat_map(|x| (-5..5).map(move |z| IVec3::new(x, GROUND as i32, z)))
            .collect();
        let solid = world(&step);
        let mut player = standing();
        run(&mut player, input, 0.0, 120, &solid);
        assert!(
            player.feet.y >= GROUND + 1.0,
            "не запрыгнул: {}",
            player.feet
        );
        assert!(player.feet.x > 2.3);

        let solid = world(&wall_x(2));
        let mut player = standing();
        run(&mut player, input, 0.0, 120, &solid);
        assert!(player.feet.x <= 2.0 - PLAYER_WIDTH / 2.0);
    }

    #[test]
    fn ceiling_stops_jump() {
        // Два блока воздуха над полом: голова (1.8) упирается в потолок через 0.2 блока.
        let ceiling: Vec<IVec3> = (-5..5)
            .flat_map(|x| (-5..5).map(move |z| IVec3::new(x, GROUND as i32 + 2, z)))
            .collect();
        let solid = world(&ceiling);
        let mut player = standing();
        let jump = Input {
            jump: true,
            ..Input::default()
        };
        player.step(&jump, 0.0, &solid, dry);
        let limit = GROUND + 2.0 - PLAYER_HEIGHT;
        let mut hit = false;
        for _ in 0..60 {
            player.step(&Input::default(), 0.0, &solid, dry);
            assert!(player.feet.y <= limit + 1e-4);
            if (player.feet.y - limit).abs() < 1e-4 && player.velocity.y == 0.0 {
                hit = true;
            }
        }
        assert!(hit, "прыжок не упёрся в потолок");
        assert_eq!(player.feet.y, GROUND);
    }

    #[test]
    fn walls_stop_movement_on_x_and_z() {
        let solid = world(&wall_x(2));
        let mut player = standing();
        run(&mut player, FORWARD, 0.0, 120, &solid);
        assert_eq!(player.feet.x, 2.0 - PLAYER_WIDTH / 2.0);
        assert_eq!(player.velocity.x, 0.0);

        let solid = world(&wall_z(2));
        let mut player = standing();
        run(&mut player, FORWARD, FRAC_PI_2, 120, &solid);
        assert_eq!(player.feet.z, 2.0 - PLAYER_WIDTH / 2.0);
    }

    #[test]
    fn diagonal_slides_along_wall() {
        let solid = world(&wall_x(2));
        let mut player = standing();
        run(&mut player, FORWARD, FRAC_PI_4, 60, &solid);
        assert_eq!(player.feet.x, 2.0 - PLAYER_WIDTH / 2.0);
        // За секунду вдоль стены — путь проекции диагонали, без потерь на трение.
        let along = WALK_SPEED * FRAC_PI_4.sin();
        assert!(
            (player.feet.z - 0.5 - along).abs() < 0.01,
            "z = {}",
            player.feet.z
        );
        assert_eq!(player.feet.y, GROUND);
    }

    #[test]
    fn leaves_corner_freely() {
        let mut blocks = wall_x(2);
        blocks.extend(wall_z(2));
        let solid = world(&blocks);
        let mut player = standing();
        run(&mut player, FORWARD, FRAC_PI_4, 60, &solid);
        let corner = 2.0 - PLAYER_WIDTH / 2.0;
        assert_eq!((player.feet.x, player.feet.z), (corner, corner));

        let back = Input {
            forward: -1.0,
            ..Input::default()
        };
        run(&mut player, back, FRAC_PI_4, 30, &solid);
        assert!(player.feet.x < corner - 1.0 && player.feet.z < corner - 1.0);
        // Из угла можно и выйти вдоль одной стены, продолжая давить во вторую.
        run(&mut player, FORWARD, FRAC_PI_4, 60, &solid);
        let along = Input {
            strafe: -1.0,
            ..FORWARD
        };
        run(&mut player, along, 0.0, 30, &solid);
        assert_eq!(player.feet.x, corner);
        assert!(player.feet.z < corner - 1.0);
    }

    #[test]
    fn flying_has_no_gravity_and_follows_vertical_input() {
        let solid = world(&[]);
        let start = GROUND + 10.0;
        let mut player = Player::new(Vec3::new(0.5, start, 0.5), Mode::Flying);
        run(&mut player, Input::default(), 0.0, 60, &solid);
        assert_eq!(player.feet.y, start);

        let up = Input {
            jump: true,
            ..Input::default()
        };
        run(&mut player, up, 0.0, 60, &solid);
        assert!((player.feet.y - start - FLY_SPEED).abs() < 0.01);
        assert_eq!(player.mode, Mode::Flying);
    }

    #[test]
    fn flying_collides_and_lands_into_walking() {
        let solid = world(&wall_x(2));
        let mut player = Player::new(Vec3::new(0.5, GROUND + 3.0, 0.5), Mode::Flying);
        run(&mut player, FORWARD, 0.0, 60, &solid);
        assert_eq!(player.feet.x, 2.0 - PLAYER_WIDTH / 2.0);
        assert_eq!(player.mode, Mode::Flying);

        let down = Input {
            descend: true,
            ..Input::default()
        };
        run(&mut player, down, 0.0, 60, &solid);
        assert_eq!(player.feet.y, GROUND);
        assert_eq!(player.mode, Mode::Walking);
    }

    #[test]
    fn flight_toggled_on_ground_lasts_until_descend() {
        let solid = world(&[]);
        let mut player = standing();
        player.toggle_mode();
        run(&mut player, Input::default(), 0.0, 30, &solid);
        assert_eq!(player.mode, Mode::Flying);
        assert_eq!(player.feet.y, GROUND);

        let down = Input {
            descend: true,
            ..Input::default()
        };
        player.step(&down, 0.0, &solid, dry);
        assert_eq!(player.mode, Mode::Walking);
        assert_eq!(player.feet.y, GROUND);
    }

    #[test]
    fn toggle_into_flight_stops_vertical_motion() {
        let solid = world(&[]);
        let mut player = Player::new(Vec3::new(0.5, GROUND + 10.0, 0.5), Mode::Walking);
        run(&mut player, Input::default(), 0.0, 10, &solid);
        player.toggle_mode();
        let y = player.feet.y;
        run(&mut player, Input::default(), 0.0, 30, &solid);
        assert_eq!(player.feet.y, y);
        player.toggle_mode();
        player.step(&Input::default(), 0.0, &solid, dry);
        assert!(player.feet.y < y);
    }

    /// Симуляция с кадрами длины `frame` в течение `seconds`: высший тик-уровень ног
    /// и интерполированное положение в конце.
    fn simulate(frame: f32, seconds: f32) -> (f32, Vec3) {
        let solid = world(&[]);
        let mut player = standing();
        let mut prev = player.feet;
        let mut clock = Clock::default();
        let input = Input {
            jump: true,
            ..FORWARD
        };
        let mut top = player.feet.y;
        let frames = (seconds / frame).round() as u32;
        for _ in 0..frames {
            for _ in 0..clock.advance(frame) {
                prev = player.feet;
                player.step(&input, 0.0, &solid, dry);
                top = top.max(player.feet.y);
            }
        }
        (top, prev.lerp(player.feet, clock.alpha()))
    }

    #[test]
    fn independent_of_frame_rate() {
        let (top60, end60) = simulate(1.0 / 60.0, 2.0);
        for fps in [30.0, 144.0] {
            let (top, end) = simulate(1.0 / fps, 2.0);
            assert_eq!(top, top60, "высота прыжка при {fps} FPS");
            assert!(
                (end.x - end60.x).abs() < 1e-3,
                "путь при {fps} FPS: {} против {}",
                end.x,
                end60.x
            );
        }
        // Путь за 2 с ходьбы — 2 × WALK_SPEED без одного шага: интерполяция между
        // прежним и текущим положением показывает тело на шаг позади.
        let expected = (2.0 - TICK) * WALK_SPEED;
        assert!(
            (end60.x - 0.5 - expected).abs() < 1e-3,
            "путь {}",
            end60.x - 0.5
        );
    }

    #[test]
    fn clock_counts_ticks_and_keeps_remainder() {
        let mut clock = Clock::default();
        assert_eq!(clock.advance(TICK * 0.5), 0);
        assert!((clock.alpha() - 0.5).abs() < 1e-4);
        assert_eq!(clock.advance(TICK * 3.0), 3);
        assert!((clock.alpha() - 0.5).abs() < 1e-3);
        clock.reset();
        assert_eq!(clock.alpha(), 0.0);
    }

    fn dry(_: IVec3) -> bool {
        false
    }

    /// Бассейн над полом: вода в x, z −10..10 от пола до y = `top` включительно.
    fn pool(top: i32) -> impl Fn(IVec3) -> bool {
        move |p: IVec3| {
            p.y > FLOOR && p.y <= top && (-10..10).contains(&p.x) && (-10..10).contains(&p.z)
        }
    }

    const JUMP: Input = Input {
        forward: 0.0,
        strafe: 0.0,
        jump: true,
        descend: false,
    };

    #[test]
    fn sinks_slowly_in_water() {
        let (solid, water) = (world(&[]), pool(75));
        let mut player = Player::new(Vec3::new(0.5, 72.0, 0.5), Mode::Walking);
        for _ in 0..60 {
            player.step(&Input::default(), 0.0, &solid, &water);
            assert!(
                player.velocity.y >= -WATER_SINK_SPEED,
                "{}",
                player.velocity.y
            );
        }
        let sunk = 72.0 - player.feet.y;
        assert!(sunk > 0.5 && sunk <= WATER_SINK_SPEED, "sunk {sunk}");
        // Дно держит, как и на суше.
        run_in(&mut player, Input::default(), 600, &solid, &water);
        assert_eq!(player.feet.y, GROUND);
    }

    fn run_in(
        player: &mut Player,
        input: Input,
        ticks: u32,
        solid: &impl Fn(IVec3) -> bool,
        water: &impl Fn(IVec3) -> bool,
    ) {
        for _ in 0..ticks {
            player.step(&input, 0.0, solid, water);
        }
    }

    #[test]
    fn space_swims_up_and_floats_at_surface() {
        let (solid, water) = (world(&[]), pool(70));
        let mut player = standing();
        run_in(&mut player, JUMP, 60, &solid, &water);
        let risen = player.feet.y - GROUND;
        assert!(risen > 2.0 && risen <= SWIM_UP_SPEED, "risen {risen}");
        // Доплывает до поверхности и дальше держится по грудь: ноги под водой, глаза над ней.
        run_in(&mut player, JUMP, 180, &solid, &water);
        let surface = 70.0 + WATER_SURFACE;
        for _ in 0..600 {
            player.step(&JUMP, 0.0, &solid, &water);
            assert!(
                player.feet.y < surface && player.feet.y > surface - 1.0,
                "{}",
                player.feet.y
            );
            assert!(!player.eyes_in_water(&water));
        }
        // Без Space — снова тонет: по воде не ходят.
        run_in(&mut player, Input::default(), 120, &solid, &water);
        assert!(player.feet.y < surface - 1.5);
        assert!(player.eyes_in_water(&water));
    }

    #[test]
    fn swims_slower_than_walking() {
        let (solid, water) = (world(&[]), pool(70));
        let mut player = Player::new(Vec3::new(-8.0, 66.0, 0.5), Mode::Walking);
        run_in(&mut player, FORWARD, 30, &solid, &water);
        assert!(
            (player.velocity.x - SWIM_SPEED).abs() < 1e-4,
            "{}",
            player.velocity.x
        );
    }

    #[test]
    fn fall_into_water_slows_down() {
        let (solid, water) = (world(&[]), pool(70));
        let mut player = Player::new(Vec3::new(0.5, 110.0, 0.5), Mode::Walking);
        while !water(player.feet.floor().as_ivec3()) {
            player.step(&Input::default(), 0.0, &solid, &water);
        }
        assert!(player.velocity.y < -20.0, "{}", player.velocity.y);
        run_in(&mut player, Input::default(), 30, &solid, &water);
        assert!(
            player.velocity.y >= -WATER_SINK_SPEED - 0.5,
            "{}",
            player.velocity.y
        );
        assert!(player.feet.y >= GROUND);
    }

    #[test]
    fn eyes_in_water_by_eye_cell_and_surface() {
        let water = pool(70);
        let at_eye = |y: f32| Player::new(Vec3::new(0.5, y - EYE_HEIGHT, 0.5), Mode::Walking);
        // Верхний блок воды — до 70.875; нижние — во всю высоту.
        assert!(at_eye(70.5).eyes_in_water(&water));
        assert!(!at_eye(70.9).eyes_in_water(&water));
        assert!(at_eye(69.95).eyes_in_water(&water));
        assert!(!at_eye(72.0).eyes_in_water(&water));
        let outside = Player::new(Vec3::new(20.5, 66.0, 0.5), Mode::Walking);
        assert!(!outside.eyes_in_water(&water));
    }

    #[test]
    fn flying_ignores_water() {
        let (solid, water) = (world(&[]), pool(75));
        let mut player = Player::new(Vec3::new(0.5, 70.0, 0.5), Mode::Flying);
        run_in(&mut player, Input::default(), 60, &solid, &water);
        assert_eq!(player.feet.y, 70.0);
        run_in(&mut player, FORWARD, 60, &solid, &water);
        assert!(
            (player.feet.x - 0.5 - FLY_SPEED).abs() < 1e-3,
            "{}",
            player.feet.x
        );
    }

    #[test]
    fn noclip_passes_through_blocks_and_never_lands() {
        let rock = |_: IVec3| true;
        let mut player = Player::new(Vec3::new(0.5, 70.0, 0.5), Mode::Walking);
        player.toggle_noclip();
        assert_eq!(player.mode, Mode::Noclip);
        let down = Input {
            descend: true,
            ..FORWARD
        };
        run(&mut player, down, 0.0, 60, &rock);
        assert!(
            player.feet.x > 5.0 && player.feet.y < 65.0,
            "{}",
            player.feet
        );
        assert_eq!(player.mode, Mode::Noclip);
        assert!(!player.on_ground);

        player.toggle_noclip();
        assert_eq!(player.mode, Mode::Flying);
        player.toggle_noclip();
        player.toggle_mode();
        assert_eq!(player.mode, Mode::Walking);
    }

    #[test]
    fn jumps_out_onto_shore_one_block_above_water() {
        // Вода до y = 66 включительно, берег при x ≥ 3 — до y = 67: верх берега на
        // блок выше верхнего блока воды.
        let shore: Vec<IVec3> = (GROUND as i32..=67)
            .flat_map(|y| (-5..5).flat_map(move |z| (3..40).map(move |x| IVec3::new(x, y, z))))
            .collect();
        let solid = world(&shore);
        let water = |p: IVec3| pool(66)(p) && p.x < 3;
        let mut player = standing();
        let out = Input {
            jump: true,
            ..FORWARD
        };
        for _ in 0..240 {
            if player.feet.x > 3.5 {
                break;
            }
            player.step(&out, 0.0, &solid, water);
        }
        assert!(player.feet.x > 3.5, "still in water at {}", player.feet);
        // На суше с зажатым Space он прыгал бы дальше: отпускаем и проверяем, что стоит на берегу.
        run_in(&mut player, FORWARD, 60, &solid, &water);
        assert_eq!(player.feet.y, 68.0);
        assert!(player.on_ground);
    }

    #[test]
    fn block_overlap_allows_touching_faces() {
        let player = standing();
        let cell = IVec3::new(0, GROUND as i32, 0);
        assert!(player.overlaps_block(cell));
        assert!(player.overlaps_block(cell + IVec3::Y));
        assert!(!player.overlaps_block(cell + IVec3::Y * 2));
        assert!(!player.overlaps_block(cell - IVec3::Y));
        assert!(!player.overlaps_block(cell + IVec3::X));
        assert!(!player.overlaps_block(cell - IVec3::Z));
    }

    #[test]
    fn sprint_speeds_up_walking_and_flying() {
        let solid = world(&[]);
        let mut player = standing();
        run(&mut player, FORWARD, 0.0, 5, &solid);
        assert!((player.velocity.x - WALK_SPEED).abs() < 1e-4);
        player.start_sprint();
        run(&mut player, FORWARD, 0.0, 5, &solid);
        assert!(player.sprinting);
        assert!((player.velocity.x - WALK_SPEED * SPRINT_FACTOR).abs() < 1e-4);

        let mut flyer = Player::new(Vec3::new(0.5, 80.0, 0.5), Mode::Flying);
        flyer.start_sprint();
        run(&mut flyer, FORWARD, 0.0, 5, &solid);
        assert!((flyer.velocity.x - FLY_SPEED * FLY_SPRINT_FACTOR).abs() < 1e-4);
    }

    #[test]
    fn sprint_ends_when_forward_released() {
        let solid = world(&[]);
        let mut player = standing();
        player.start_sprint();
        run(&mut player, FORWARD, 0.0, 5, &solid);
        run(&mut player, Input::default(), 0.0, 1, &solid);
        assert!(!player.sprinting);
        // Назад тоже не бег: forward < 0.
        player.start_sprint();
        let back = Input {
            forward: -1.0,
            ..FORWARD
        };
        run(&mut player, back, 0.0, 1, &solid);
        assert!(!player.sprinting);
    }

    #[test]
    fn sprint_ends_against_wall() {
        let solid = world(&wall_x(3));
        let mut player = standing();
        player.start_sprint();
        run(&mut player, FORWARD, 0.0, 120, &solid);
        assert!(!player.sprinting);
        assert!(player.feet.x < 3.0);
    }

    #[test]
    fn sprint_ends_in_water() {
        let (solid, water) = (world(&[]), pool(70));
        let mut player = Player::new(Vec3::new(-8.0, 66.0, 0.5), Mode::Walking);
        player.start_sprint();
        run_in(&mut player, FORWARD, 1, &solid, &water);
        assert!(!player.sprinting);
    }

    #[test]
    fn fov_scale_follows_sprint_in_set_time() {
        let mut scale = 1.0;
        for _ in 0..9 {
            scale = next_fov_scale(scale, true, TICK);
        }
        // 9 шагов по 1/60 с = 0.15 с.
        assert!((scale - SPRINT_FOV_SCALE).abs() < 1e-4, "{scale}");
        scale = next_fov_scale(scale, true, 1.0);
        assert_eq!(scale, SPRINT_FOV_SCALE);
        for _ in 0..9 {
            scale = next_fov_scale(scale, false, TICK);
        }
        assert!((scale - 1.0).abs() < 1e-4, "{scale}");
    }
}
