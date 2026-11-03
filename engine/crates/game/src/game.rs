use std::collections::{HashMap, HashSet};

use glam::{IVec3, Vec2, Vec3};
use world::block::{Block, FLUID_SOURCE, FluidKind};
use world::clouds::cloud_drift;
use world::column::{Column, WORLD_HEIGHT};
use world::falling::Falling;
use world::fluid::Fluid;
use world::item::Item;
use world::light::LightMap;
use world::mob::{MobKind, MobRecord};
use world::pos::{ColumnPos, SectionPos};
use world::random_ticks::{RandomTicks, STEP_TICKS};
use world::terrain::{Biome, DEBUG_SPAWN_Y, Terrain, WorldType};
use world::world::{World, affected_sections};

use crate::camera::Camera;
use crate::collide::{center_xz, overlap_depth, push_away};
use crate::daytime::{self, DayTime, SkyState};
use crate::double_tap::DoubleTap;
use crate::loader::DEFAULT_RADIUS;
use crate::mob::{Mob, Pose, Surroundings, spec};
use crate::mob_shadow::{Ground, ShadowPatch, shadow_patches};
use crate::physics::{Clock, Collision, EYE_HEIGHT, Input, Mode, Player, TICK, next_fov_scale};
use crate::rain_cover::RainCover;
use crate::raycast::{Hit, raycast};
use crate::repeat::Repeat;
use crate::rng::Rng;
use crate::weather::{self, Weather, WeatherState};

const TIME_FAST_FACTOR: u32 = 60;
const REACH: f32 = 6.0;
const REPEAT_S: f32 = 0.25;
const DEFAULT_BLOCK: Block = Block::Grass;
const MAX_DT: f32 = 0.1;
const DOUBLE_TAP_S: f32 = 0.3;
const FLUID_STEP_TICKS: u32 = 15;
const LAVA_STEP_TICKS: u32 = 30;
const FALL_STEP_TICKS: u32 = 6;
const MOB_RECORD_GRID: f32 = 8.0;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Held {
    Block(Block),
    Item(Item),
}

pub struct MobView {
    pub kind: MobKind,
    pub pose: Pose,
    pub sky_level: u8,
    pub block_level: u8,
    pub shadow_radius: f32,
    pub shadow: Vec<ShadowPatch>,
}

#[derive(Default)]
pub struct Edits {
    pub dirty: Vec<SectionPos>,
    pub lit: Vec<SectionPos>,
}

impl Edits {
    fn new(dirty: HashSet<SectionPos>, lit: HashSet<SectionPos>) -> Self {
        let lit = lit.into_iter().filter(|key| !dirty.contains(key)).collect();
        Self {
            dirty: dirty.into_iter().collect(),
            lit,
        }
    }
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
    random_ticks: RandomTicks,
    random_tick_acc: u32,
    view_radius: i32,
    pending_dirty: HashSet<SectionPos>,
    pending_lit: HashSet<SectionPos>,
    pub terrain: Terrain,
    seed: i32,
    camera: Camera,
    player: Player,
    prev_feet: Vec3,
    prev_step_offset: f32,
    clock: Clock,
    day: DayTime,
    anim_ticks: u64,
    cloud_ticks: u64,
    time_frozen: bool,
    time_fast: bool,
    weather: WeatherState,
    weather_enabled: bool,
    rain_cover: RainCover,
    space_tap: DoubleTap,
    sprint_tap: DoubleTap,
    fov_scale: f32,
    keys: HashSet<String>,
    last_time: Option<f32>,
    paused: bool,
    target: Option<IVec3>,
    breaking: Repeat,
    placing: Repeat,
    selected: Option<Held>,
    mobs: Vec<Mob>,
    adopted: HashSet<ColumnPos>,
    mob_rng: Rng,
}

impl Game {
    pub fn new(seed: i32, state: Option<&[f32]>) -> Result<Game, BadCameraState> {
        Self::of(seed, WorldType::Standard, state)
    }

    pub fn of(
        seed: i32,
        world_type: WorldType,
        state: Option<&[f32]>,
    ) -> Result<Game, BadCameraState> {
        let terrain = Terrain::of(seed, world_type);
        let (camera, player) = match state {
            Some(state) => restore_player(state).ok_or(BadCameraState)?,
            None if world_type == WorldType::Debug => {
                let player = Player::new(Vec3::new(0.5, DEBUG_SPAWN_Y as f32, 0.5), Mode::Flying);
                (Camera::spawn(player.eye()), player)
            }
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
            random_ticks: RandomTicks::new(seed),
            random_tick_acc: 0,
            view_radius: DEFAULT_RADIUS,
            pending_dirty: HashSet::new(),
            pending_lit: HashSet::new(),
            terrain,
            seed,
            camera,
            prev_feet: player.feet,
            prev_step_offset: player.step_offset,
            player,
            clock: Clock::default(),
            day: DayTime::default(),
            anim_ticks: 0,
            cloud_ticks: 0,
            time_frozen: false,
            time_fast: false,
            weather: WeatherState::new(seed),
            weather_enabled: true,
            rain_cover: RainCover::new(),
            space_tap: DoubleTap::new(DOUBLE_TAP_S),
            sprint_tap: DoubleTap::new(DOUBLE_TAP_S),
            fov_scale: 1.0,
            keys: HashSet::new(),
            last_time: None,
            paused: false,
            target: None,
            breaking: Repeat::new(REPEAT_S),
            placing: Repeat::new(REPEAT_S),
            selected: Some(Held::Block(DEFAULT_BLOCK)),
            mobs: Vec::new(),
            adopted: HashSet::new(),
            mob_rng: Rng::new(seed as u32 ^ 0x9E37_79B9),
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
        self.player = player;
        self.sync_prev();
        self.clock.reset();
        self.target = None;
        Ok(())
    }

    pub fn anim_time(&self) -> f32 {
        (self.anim_ticks as f64 * f64::from(TICK)) as f32
    }

    pub fn cloud_drift(&self) -> f32 {
        cloud_drift(self.cloud_ticks as f64 * f64::from(TICK))
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

    pub fn set_auto_step(&mut self, enabled: bool) {
        self.player.auto_step = enabled;
    }

    pub fn set_view_radius(&mut self, radius: i32) {
        self.view_radius = radius;
    }

    pub fn set_time_frozen(&mut self, frozen: bool) {
        self.time_frozen = frozen;
    }

    pub fn set_time_fast(&mut self, fast: bool) {
        self.time_fast = fast;
    }

    pub fn sky(&self) -> SkyState {
        weather::tint_sky(
            daytime::sky_state(self.day.fraction()),
            self.weather.rain(),
            self.flash(),
        )
    }

    pub fn weather(&self) -> Weather {
        self.weather.kind()
    }

    pub fn rain(&self) -> f32 {
        self.weather.rain()
    }

    pub fn flash(&self) -> f32 {
        if self.camera_biome().is_snowy() {
            0.0
        } else {
            self.weather.flash()
        }
    }

    fn camera_biome(&self) -> Biome {
        let p = self.camera.position.floor().as_ivec3();
        self.terrain.biome_at(p.x, p.z)
    }

    pub fn snowing(&self) -> bool {
        self.weather.rain() > 0.0 && self.camera_biome().is_snowy()
    }

    pub fn rain_drops(&self) -> f32 {
        let rain = self.weather.rain();
        if rain > 0.0 && self.camera_biome() != Biome::Desert {
            rain
        } else {
            0.0
        }
    }

    pub fn cycle_weather(&mut self) {
        if self.weather_enabled {
            self.weather.set_kind(self.weather.kind().next());
        }
    }

    pub fn weather_enabled(&self) -> bool {
        self.weather_enabled
    }

    pub fn set_weather_enabled(&mut self, enabled: bool) {
        self.weather_enabled = enabled;
    }

    pub fn weather_state(&self) -> Vec<f32> {
        self.weather.to_state()
    }

    pub fn set_weather_state(&mut self, state: &[f32]) {
        if let Some(restored) = WeatherState::from_state(self.seed, state) {
            self.weather = restored;
        }
    }

    pub fn take_rain_cover(&mut self) -> Option<&[u8]> {
        self.rain_cover.take_dirty()
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
            self.sync_prev();
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
                self.prev_step_offset = 0.0;
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
            self.prev_step_offset = 0.0;
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
        let held = Block::from_name(name)
            .map(Held::Block)
            .or_else(|| Item::from_name(name).map(Held::Item));
        match held {
            Some(held) => {
                self.selected = Some(held);
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
        if self.rain_drops() > 0.0 {
            let p = self.camera.position.floor().as_ivec3();
            self.rain_cover.update(&self.world, p.x, p.z);
        }
        dt
    }

    pub fn interact(&mut self, time: f32) -> Edits {
        let mut dirty = std::mem::take(&mut self.pending_dirty);
        let mut lit = std::mem::take(&mut self.pending_lit);
        if self.paused {
            return Edits::new(dirty, lit);
        }
        let mut target = self.pick();
        let editable = self.terrain.world_type() != WorldType::Debug;
        if editable
            && self.breaking.fire(time)
            && let Some(hit) = target
            && hit.block.y >= 0
            && !self.world.block(hit.block).is_unbreakable()
            && let changed = self
                .world
                .set_block_supported(hit.block, left_after_break(self.world.block(hit.block)))
            && !changed.is_empty()
        {
            self.apply_edit(&changed, &[], &mut dirty, &mut lit);
            target = self.pick();
        }
        if let Some(Held::Item(item)) = self.selected {
            if self.placing.fire_once()
                && let Some(hit) = target
            {
                self.use_item(item, hit);
            }
        } else if editable
            && let Some(Held::Block(block)) = self.selected
            && self.placing.fire(time)
            && let Some((hit, pos)) = self.placement_hit(target, block)
            && let Some(data) = block.placement_data(hit.normal)
            && self.world.can_place_with_data(pos, block, data)
            && (block.fluid().is_some()
                || !self.player.overlaps_block(pos)
                    && (block.collision_height() == 0.0 || !self.mob_overlaps_block(pos)))
        {
            let changed = self.world.set_block_supported_with_data(pos, block, data);
            if !changed.is_empty() {
                self.apply_edit(&changed, &[], &mut dirty, &mut lit);
                target = self.pick();
            }
        }
        self.target = target.map(|hit| hit.block);
        Edits::new(dirty, lit)
    }

    pub fn target_block(&self) -> Option<(IVec3, Block, u8)> {
        self.target
            .map(|pos| (pos, self.world.block(pos), self.world.block_data(pos)))
    }

    /// `add_column` пересчитывает его заново вместе с соседями.
    pub fn import_column(&mut self, key: ColumnPos, column: Column) {
        self.mobs
            .retain(|mob| ColumnPos::of_block(mob.body.feet.floor().as_ivec3()) != key);
        self.adopted.remove(&key);
        self.world.import(key, column);
        self.rain_cover.invalidate_column(key);
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
        light_neutral: &[IVec3],
        dirty: &mut HashSet<SectionPos>,
        lit: &mut HashSet<SectionPos>,
    ) {
        for &pos in changed {
            self.fluid.block_changed(&self.world, pos);
            self.lava.block_changed(&self.world, pos);
            self.falling.block_changed(&self.world, pos);
            self.rain_cover.invalidate(pos);
            dirty.extend(affected_sections(pos));
            if !light_neutral.contains(&pos) {
                lit.extend(self.light.block_changed(&self.world, pos));
            }
        }
    }

    fn use_item(&mut self, item: Item, hit: Hit) {
        let cell = hit.block + hit.normal;
        let feet = Vec3::new(cell.x as f32 + 0.5, cell.y as f32, cell.z as f32 + 0.5);
        let yaw = self.mob_rng.range(0.0, std::f32::consts::TAU);
        let seed = self.mob_rng.next_u32();
        let mob = Mob::new(item.def().spawns, feet, yaw, seed);
        let (min, max) = mob.body.aabb();
        let (lo, hi) = (
            (min + 1e-3).floor().as_ivec3(),
            (max - 1e-3).floor().as_ivec3(),
        );
        let clear = (lo.x..=hi.x).all(|x| {
            (lo.y..=hi.y).all(|y| {
                (lo.z..=hi.z)
                    .all(|z| self.world.block(IVec3::new(x, y, z)).collision_height() == 0.0)
            })
        });
        if clear && self.world.is_loaded(cell) {
            self.mobs.push(mob);
        }
    }

    pub fn mob_count(&self) -> usize {
        self.mobs.len()
    }

    pub fn mobs(&self) -> &[Mob] {
        &self.mobs
    }

    pub fn mob_views(&self) -> Vec<MobView> {
        let alpha = self.clock.alpha();
        self.mobs
            .iter()
            .map(|mob| {
                let pose = mob.interpolated(alpha);
                let cell = (pose.feet + Vec3::Y * 0.45).floor().as_ivec3();
                MobView {
                    kind: mob.kind,
                    pose,
                    sky_level: self.light.sky(cell),
                    block_level: self.light.block(cell),
                    shadow_radius: spec(mob.kind).shadow_radius,
                    shadow: shadow_patches(pose.feet, spec(mob.kind).shadow_radius, |pos| {
                        let block = self.world.block(pos);
                        let height = block.collision_height();
                        if block == Block::Air {
                            Ground::Air
                        } else if height > 0.0 && block.fluid().is_none() {
                            Ground::Solid(height)
                        } else {
                            Ground::Blocked
                        }
                    }),
                }
            })
            .collect()
    }

    fn adopt_mobs(&mut self) {
        self.adopted.retain(|key| self.world.column(*key).is_some());
        let loaded: HashSet<ColumnPos> = self.world.keys().collect();
        self.mobs
            .retain(|mob| loaded.contains(&ColumnPos::of_block(mob.body.feet.floor().as_ivec3())));
        let fresh: Vec<ColumnPos> = loaded
            .into_iter()
            .filter(|key| !self.adopted.contains(key))
            .collect();
        for key in fresh {
            self.adopted.insert(key);
            let Some(column) = self.world.column(key) else {
                continue;
            };
            for record in column.mobs.clone() {
                let seed = self.mob_rng.next_u32();
                self.mobs.push(Mob::from_record(&record, seed));
            }
        }
    }

    fn tick_mobs(&mut self) {
        let world = &self.world;
        let eye = self.player.eye();
        let collision = |pos| {
            let block = world.block(pos);
            Collision {
                height: block.collision_height(),
                slippery: block.is_slippery(),
            }
        };
        let fluid = |pos| world.fluid_height(pos);
        let around = Surroundings {
            collision: &collision,
            fluid: &fluid,
            player_eye: eye,
        };
        for mob in &mut self.mobs {
            let (min, max) = mob.body.aabb();
            let (lo, hi) = (
                ColumnPos::of_block((min - Vec3::ONE).floor().as_ivec3()),
                ColumnPos::of_block((max + Vec3::ONE).floor().as_ivec3()),
            );
            let loaded = (lo.x()..=hi.x())
                .all(|x| (lo.z()..=hi.z()).all(|z| world.column(ColumnPos::new(x, z)).is_some()));
            if loaded {
                mob.tick(&around);
            }
        }
    }

    fn push_entities(&mut self) {
        let world = &self.world;
        let collision = |pos| {
            let block = world.block(pos);
            Collision {
                height: block.collision_height(),
                slippery: block.is_slippery(),
            }
        };
        let bounds: Vec<_> = self.mobs.iter().map(|mob| mob.body.aabb()).collect();
        let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for (i, bound) in bounds.iter().enumerate() {
            let at = center_xz(*bound).floor();
            grid.entry((at.x as i32, at.y as i32)).or_default().push(i);
        }
        let mut pushes = vec![Vec2::ZERO; bounds.len()];
        for (i, bound) in bounds.iter().enumerate() {
            let at = center_xz(*bound).floor();
            for dx in -1..=1 {
                for dz in -1..=1 {
                    let Some(cell) = grid.get(&(at.x as i32 + dx, at.y as i32 + dz)) else {
                        continue;
                    };
                    for &j in cell.iter().filter(|&&j| j > i) {
                        if let Some(depth) = overlap_depth(*bound, bounds[j]) {
                            let push = push_away(*bound, bounds[j], depth, (i + j) as u32);
                            pushes[i] -= push;
                            pushes[j] += push;
                        }
                    }
                }
            }
        }
        let player = self.player.aabb();
        let mut player_push = Vec2::ZERO;
        for (i, bound) in bounds.iter().enumerate() {
            if let Some(depth) = overlap_depth(player, *bound) {
                let push = push_away(player, *bound, depth, i as u32);
                pushes[i] += push;
                player_push -= push;
            }
        }
        for (mob, push) in self.mobs.iter_mut().zip(pushes) {
            if push != Vec2::ZERO {
                mob.body.push(push, collision);
            }
        }
        if player_push != Vec2::ZERO {
            self.player.push(player_push, collision);
        }
    }

    fn mob_overlaps_block(&self, pos: IVec3) -> bool {
        self.mobs.iter().any(|mob| mob.body.overlaps_block(pos))
    }

    fn sync_mobs(&mut self) {
        let mut by_column: std::collections::HashMap<ColumnPos, Vec<MobRecord>> =
            self.adopted.iter().map(|key| (*key, Vec::new())).collect();
        for mob in &self.mobs {
            let mut record = mob.record();
            record.pos = (record.pos * MOB_RECORD_GRID).round() / MOB_RECORD_GRID;
            record.yaw = (record.yaw * MOB_RECORD_GRID).round() / MOB_RECORD_GRID;
            let key = ColumnPos::of_block(mob.body.feet.floor().as_ivec3());
            by_column.entry(key).or_default().push(record);
        }
        for (key, records) in by_column {
            self.world.set_mobs(key, records);
        }
    }

    fn update_player(&mut self, dt: f32) {
        let input = self.input();
        let ticks = self.clock.advance(dt);
        self.anim_ticks += u64::from(ticks);
        let factor = if self.time_fast { TIME_FAST_FACTOR } else { 1 };
        self.cloud_ticks += u64::from(ticks * factor);
        if !self.time_frozen {
            self.day.advance(ticks * factor);
        }
        let weather_pace = if self.time_frozen { 0 } else { factor };
        self.weather
            .advance(ticks, weather_pace, self.weather_enabled);
        if self.terrain.world_type() != WorldType::Debug {
            self.advance_fluid(ticks);
            self.advance_lava(ticks);
            self.advance_falling(ticks);
            self.advance_random_ticks(ticks * factor);
        }
        self.adopt_mobs();
        for _ in 0..ticks {
            if !self.body_loaded() {
                self.clock.reset();
                self.sync_prev();
                break;
            }
            self.sync_prev();
            self.player.step_with(
                &input,
                self.camera.yaw,
                |pos| {
                    let block = self.world.block(pos);
                    Collision {
                        height: block.collision_height(),
                        slippery: block.is_slippery(),
                    }
                },
                |pos| self.world.fluid_height(pos),
            );
            self.tick_mobs();
            self.push_entities();
        }
        self.sync_mobs();
        let from = self.prev_feet - Vec3::Y * self.prev_step_offset;
        let to = self.player.feet - Vec3::Y * self.player.step_offset;
        self.camera.position = from.lerp(to, self.clock.alpha()) + Vec3::Y * EYE_HEIGHT;
    }

    fn sync_prev(&mut self) {
        self.prev_feet = self.player.feet;
        self.prev_step_offset = self.player.step_offset;
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

    fn advance_random_ticks(&mut self, ticks: u32) {
        self.random_tick_acc += ticks;
        let center = self.camera_column();
        while self.random_tick_acc >= STEP_TICKS {
            self.random_tick_acc -= STEP_TICKS;
            let ticked =
                self.random_ticks
                    .step(&mut self.world, &self.light, center, self.view_radius);
            self.apply_pending_with(&ticked.changed, &ticked.light_neutral);
        }
    }

    fn apply_pending(&mut self, changed: &[IVec3]) {
        self.apply_pending_with(changed, &[]);
    }

    fn apply_pending_with(&mut self, changed: &[IVec3], light_neutral: &[IVec3]) {
        let (mut dirty, mut lit) = (
            std::mem::take(&mut self.pending_dirty),
            std::mem::take(&mut self.pending_lit),
        );
        self.apply_edit(changed, light_neutral, &mut dirty, &mut lit);
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

fn left_after_break(block: Block) -> Block {
    if block == Block::Ice {
        Block::Water
    } else {
        Block::Air
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

mod command;

#[cfg(test)]
mod tests;
