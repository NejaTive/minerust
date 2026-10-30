use std::collections::HashSet;

use glam::{IVec3, Vec3};
use world::block::{Block, FLUID_SOURCE, FluidKind};
use world::column::{Column, WORLD_HEIGHT};
use world::falling::Falling;
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

const TIME_FAST_FACTOR: u32 = 60;
const REACH: f32 = 6.0;
const REPEAT_S: f32 = 0.25;
const DEFAULT_BLOCK: Block = Block::Grass;
const MAX_DT: f32 = 0.1;
const DOUBLE_TAP_S: f32 = 0.3;
const FLUID_STEP_TICKS: u32 = 15;
const LAVA_STEP_TICKS: u32 = 30;
const FALL_STEP_TICKS: u32 = 6;

#[derive(Default)]
pub struct Edits {
    pub dirty: Vec<SectionPos>,
    pub lit: Vec<SectionPos>,
}

pub struct Game {
    pub world: World,
    pub light: LightMap,
    fluid: Fluid,
    fluid_ticks: u32,
    lava: Fluid,
    lava_ticks: u32,
    falling: Falling,
    fall_ticks: u32,
    pending_dirty: HashSet<SectionPos>,
    pending_lit: Vec<SectionPos>,
    pub terrain: Terrain,
    seed: i32,
    camera: Camera,
    player: Player,
    prev_feet: Vec3,
    clock: Clock,
    day: DayTime,
    time_frozen: bool,
    time_fast: bool,
    space_tap: DoubleTap,
    sprint_tap: DoubleTap,
    fov_scale: f32,
    keys: HashSet<String>,
    last_time: Option<f32>,
    paused: bool,
    target: Option<IVec3>,
    breaking: Repeat,
    placing: Repeat,
    selected: Option<Block>,
}

impl Game {
    pub fn new(seed: i32, state: Option<&[f32]>) -> Result<Game, BadCameraState> {
        let terrain = Terrain::new(seed);
        let (camera, player) = match state {
            Some(state) => restore_player(state).ok_or(BadCameraState)?,
            None => {
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
            lava: Fluid::of(FluidKind::Lava),
            lava_ticks: 0,
            falling: Falling::new(),
            fall_ticks: 0,
            pending_dirty: HashSet::new(),
            pending_lit: Vec::new(),
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

    pub fn camera_column(&self) -> ColumnPos {
        let p = self.camera.position.floor().as_ivec3();
        ColumnPos::of_block(p)
    }

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

    pub fn time_of_day(&self) -> f32 {
        self.day.fraction()
    }

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

    pub fn eye_fluid(&self) -> Option<FluidKind> {
        self.player.eye_fluid(|pos| self.world.fluid_height(pos))
    }

    pub fn set_paused(&mut self, paused: bool) {
        if paused == self.paused {
            return;
        }
        self.paused = paused;
        if paused {
            self.release_all_keys();
        } else {
            self.clock.reset();
            self.prev_feet = self.player.feet;
        }
    }

    pub fn mouse_move(&mut self, dx: f32, dy: f32) {
        if !self.paused {
            self.camera.rotate(dx, dy);
        }
    }

    pub fn set_key(&mut self, code: &str, pressed: bool) {
        if pressed {
            if self.paused {
                return;
            }
            let fresh = self.keys.insert(code.to_owned());
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

    pub fn toggle_noclip(&mut self) {
        if !self.paused {
            self.player.toggle_noclip();
        }
    }

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

    pub fn release_all_keys(&mut self) {
        self.keys.clear();
        self.player.sprinting = false;
        self.breaking.reset();
        self.placing.reset();
    }

    pub fn update(&mut self, time: f32) -> f32 {
        let dt = self
            .last_time
            .map_or(0.0, |last| (time - last).clamp(0.0, MAX_DT));
        self.last_time = Some(time);
        if !self.paused {
            self.update_player(dt);
        }
        self.fov_scale = next_fov_scale(self.fov_scale, self.player.sprinting, dt);
        dt
    }

    pub fn interact(&mut self, time: f32) -> Edits {
        let mut dirty = std::mem::take(&mut self.pending_dirty);
        let mut lit = std::mem::take(&mut self.pending_lit);
        if self.paused {
            return Edits {
                dirty: dirty.into_iter().collect(),
                lit,
            };
        }
        let mut target = self.pick();
        if self.breaking.fire(time)
            && let Some(hit) = target
            && hit.block.y >= 0
            && let changed = self.world.set_block_supported(hit.block, Block::Air)
            && !changed.is_empty()
        {
            self.apply_edit(&changed, &mut dirty, &mut lit);
            target = self.pick();
        }
        if self.placing.fire(time)
            && let Some(block) = self.selected
            && let Some((hit, pos)) = self.placement_hit(target, block)
            && let Some(data) = block.placement_data(hit.normal)
            && self.world.can_place_with_data(pos, block, data)
            && (block.fluid().is_some() || !self.player.overlaps_block(pos))
        {
            let changed = self.world.set_block_supported_with_data(pos, block, data);
            if !changed.is_empty() {
                self.apply_edit(&changed, &mut dirty, &mut lit);
                target = self.pick();
            }
        }
        self.target = target.map(|hit| hit.block);
        lit.retain(|key| !dirty.contains(key));
        Edits {
            dirty: dirty.into_iter().collect(),
            lit,
        }
    }

    pub fn target_block(&self) -> Option<(IVec3, Block, u8)> {
        self.target
            .map(|pos| (pos, self.world.block(pos), self.world.block_data(pos)))
    }

    /// `add_column` пересчитывает его заново вместе с соседями.
    pub fn import_column(&mut self, key: ColumnPos, column: Column) {
        self.world.import(key, column);
        if self.light.column(key).is_some() {
            self.light.add_column(&self.world, key);
        }
    }

    fn placement_hit(&self, solid: Option<Hit>, block: Block) -> Option<(Hit, IVec3)> {
        let held = block.fluid();
        let refills = |pos: IVec3| {
            held.is_some()
                && self.world.block(pos).fluid() == held
                && self.world.block_data(pos) != FLUID_SOURCE
        };
        let fluid = raycast(self.camera.position, self.camera.forward(), REACH, refills)
            .map(|hit| (hit, hit.block));
        let solid = solid.map(|hit| (hit, hit.block + hit.normal));
        match (solid, fluid) {
            (Some(solid), Some(fluid)) if fluid.0.distance < solid.0.distance => Some(fluid),
            (Some(solid), _) => Some(solid),
            (None, fluid) => fluid,
        }
    }

    fn pick(&self) -> Option<Hit> {
        raycast(self.camera.position, self.camera.forward(), REACH, |pos| {
            self.world.block(pos).is_targetable()
        })
    }

    fn apply_edit(
        &mut self,
        changed: &[IVec3],
        dirty: &mut HashSet<SectionPos>,
        lit: &mut Vec<SectionPos>,
    ) {
        for &pos in changed {
            self.fluid.block_changed(&self.world, pos);
            self.lava.block_changed(&self.world, pos);
            self.falling.block_changed(&self.world, pos);
            dirty.extend(affected_sections(pos));
            lit.extend(self.light.block_changed(&self.world, pos));
        }
    }

    fn update_player(&mut self, dt: f32) {
        let input = self.input();
        let ticks = self.clock.advance(dt);
        if !self.time_frozen {
            let factor = if self.time_fast { TIME_FAST_FACTOR } else { 1 };
            self.day.advance(ticks * factor);
        }
        self.advance_fluid(ticks);
        self.advance_lava(ticks);
        self.advance_falling(ticks);
        for _ in 0..ticks {
            if !self.body_loaded() {
                self.clock.reset();
                self.prev_feet = self.player.feet;
                break;
            }
            self.prev_feet = self.player.feet;
            self.player.step(
                &input,
                self.camera.yaw,
                |pos| self.world.block(pos).is_solid(),
                |pos| self.world.fluid_height(pos),
            );
        }
        let feet = self.prev_feet.lerp(self.player.feet, self.clock.alpha());
        self.camera.position = feet + Vec3::Y * EYE_HEIGHT;
    }

    fn advance_fluid(&mut self, ticks: u32) {
        self.fluid_ticks += ticks;
        while self.fluid_ticks >= FLUID_STEP_TICKS {
            self.fluid_ticks -= FLUID_STEP_TICKS;
            let changed = self.fluid.step(&mut self.world);
            self.apply_pending(&changed);
        }
    }

    fn advance_lava(&mut self, ticks: u32) {
        self.lava_ticks += ticks;
        while self.lava_ticks >= LAVA_STEP_TICKS {
            self.lava_ticks -= LAVA_STEP_TICKS;
            let changed = self.lava.step(&mut self.world);
            self.apply_pending(&changed);
        }
    }

    fn advance_falling(&mut self, ticks: u32) {
        self.fall_ticks += ticks;
        while self.fall_ticks >= FALL_STEP_TICKS {
            self.fall_ticks -= FALL_STEP_TICKS;
            let player = &self.player;
            let changed = self
                .falling
                .step(&mut self.world, |pos| player.overlaps_block(pos));
            self.apply_pending(&changed);
        }
    }

    fn apply_pending(&mut self, changed: &[IVec3]) {
        let (mut dirty, mut lit) = (
            std::mem::take(&mut self.pending_dirty),
            std::mem::take(&mut self.pending_lit),
        );
        self.apply_edit(changed, &mut dirty, &mut lit);
        (self.pending_dirty, self.pending_lit) = (dirty, lit);
    }

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

    fn body_loaded(&self) -> bool {
        let (min, max) = self.player.aabb();
        let lo = (min - Vec3::ONE).floor().as_ivec3();
        let hi = (max + Vec3::ONE).floor().as_ivec3();
        let (lo, hi) = (ColumnPos::of_block(lo), ColumnPos::of_block(hi));
        (lo.x()..=hi.x())
            .all(|x| (lo.z()..=hi.z()).all(|z| self.world.column(ColumnPos::new(x, z)).is_some()))
    }
}

#[derive(Debug)]
pub struct BadCameraState;

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
