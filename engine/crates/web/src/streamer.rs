use std::collections::VecDeque;

use game::loader::{Loader, NEIGHBORS, Progress};
use game::{Edits, Game};
use mesh::job::{decode_result, encode_column_job, encode_section_job};
use mesh::mesher::{MeshOptions, mesh_world_column, mesh_world_section};
use render::Renderer;
use world::light::LightMap;
use world::pos::{ColumnPos, SectionPos};
use world::section::SECTION_SIZE;
use world::world::World;

use crate::jobs::{MeshJobs, Target};
use crate::profiler::Profiler;

const START_RADIUS: i32 = 2;
const STREAM_BUDGET_MS: f64 = 3.0;

pub(crate) struct Streamer {
    loader: Loader,
    mesh_options: MeshOptions,
    stale: Vec<ColumnPos>,
    stale_sections: Vec<SectionPos>,
    jobs: MeshJobs,
    backlog: VecDeque<ColumnPos>,
    outbox: VecDeque<Vec<u8>>,
    results: VecDeque<Vec<u8>>,
}

impl Streamer {
    pub(crate) fn new(center: ColumnPos) -> Self {
        Streamer {
            loader: Loader::new(center),
            mesh_options: MeshOptions::default(),
            stale: Vec::new(),
            stale_sections: Vec::new(),
            jobs: MeshJobs::default(),
            backlog: VecDeque::new(),
            outbox: VecDeque::new(),
            results: VecDeque::new(),
        }
    }

    pub(crate) fn warm_up(
        &mut self,
        game: &mut Game,
        renderer: &mut Renderer,
        profiler: &mut Profiler,
    ) {
        let start = self.loader.warm_up(
            &mut game.world,
            |key| {
                profiler
                    .generate_time
                    .measure(&profiler.performance, || game.terrain.generate(key))
            },
            START_RADIUS,
        );
        for key in game.world.keys().collect::<Vec<_>>() {
            profiler.light_time.measure(&profiler.performance, || {
                game.light.add_column(&game.world, key)
            });
        }
        for key in start {
            self.mesh_column_here(&game.world, &game.light, renderer, profiler, key);
        }
    }

    pub(crate) fn radius(&self) -> i32 {
        self.loader.radius()
    }

    pub(crate) fn set_radius(&mut self, columns: i32) {
        self.loader.set_radius(columns);
    }

    pub(crate) fn set_ambient_occlusion(
        &mut self,
        enabled: bool,
        game: &Game,
        renderer: &Renderer,
    ) {
        if enabled == self.mesh_options.ambient_occlusion {
            return;
        }
        self.mesh_options.ambient_occlusion = enabled;
        let center = game.camera_column();
        self.stale = renderer.column_keys().collect();
        self.stale
            .sort_by_key(|&key| std::cmp::Reverse((key - center).length_squared()));
    }

    pub(crate) fn pending(&self) -> usize {
        self.loader.pending()
            + self.stale.len()
            + self.stale_sections.len()
            + self.backlog.len()
            + self.jobs.len()
    }

    pub(crate) fn jobs_in_flight(&self) -> usize {
        self.jobs.len()
    }

    pub(crate) fn set_job_slots(&mut self, slots: usize) {
        self.jobs.set_slots(slots);
        if slots > 0 {
            return;
        }
        self.outbox.clear();
        self.results.clear();
        for ticket in self.jobs.clear() {
            match ticket.target {
                Target::Column(key) => self.backlog.push_back(key),
                Target::Section(key) => self.stale_sections.push(key),
            }
        }
    }

    pub(crate) fn take_job(&mut self) -> Option<Vec<u8>> {
        self.outbox.pop_front()
    }

    pub(crate) fn push_result(&mut self, bytes: &[u8], mesh_ms: f64, profiler: &mut Profiler) {
        profiler.worker_time.record(mesh_ms);
        self.results.push_back(bytes.to_vec());
    }

    pub(crate) fn update(
        &mut self,
        game: &mut Game,
        renderer: &mut Renderer,
        profiler: &mut Profiler,
    ) {
        let deadline = profiler.now().map(|now| now + STREAM_BUDGET_MS);
        for key in self.loader.update(game.camera_column(), &mut game.world) {
            renderer.remove_column(key);
            self.jobs.unload(key);
            self.backlog.retain(|&queued| queued != key);
        }
        self.unload_light(game, renderer);
        let mut loader_done = false;
        for step in 0usize.. {
            let stale_turn = loader_done || step % 2 == 0;
            let mut did = self.apply_result(renderer, profiler)
                || self.mesh_next_section(game, renderer, profiler)
                || self.mesh_next_backlog(game, renderer, profiler)
                || (stale_turn && self.rebuild_stale(game, renderer, profiler));
            if !did && !loader_done {
                did = self.advance_loader(game, renderer, profiler);
                if !did {
                    loader_done = true;
                    did = self.rebuild_stale(game, renderer, profiler);
                }
            }
            if !did {
                if loader_done && self.is_idle() {
                    profiler.finish_load();
                }
                break;
            }
            let in_time = match (profiler.now(), deadline) {
                (Some(now), Some(deadline)) => now < deadline,
                _ => false,
            };
            if !in_time {
                break;
            }
        }
    }

    fn is_idle(&self) -> bool {
        self.stale.is_empty()
            && self.stale_sections.is_empty()
            && self.backlog.is_empty()
            && self.jobs.is_empty()
    }

    fn can_start(&self) -> bool {
        !self.jobs.enabled() || self.jobs.free_slots() > 0
    }

    fn apply_result(&mut self, renderer: &mut Renderer, profiler: &mut Profiler) -> bool {
        let Some(bytes) = self.results.pop_front() else {
            return false;
        };
        profiler.apply_time.measure(&profiler.performance, || {
            let result = decode_result(&bytes);
            let Some(ticket) = self.jobs.finish(result.id) else {
                return;
            };
            match ticket.target {
                Target::Column(key) => {
                    renderer.upload_column(key, &result.into_column());
                    if ticket.options != self.mesh_options {
                        self.stale.push(key);
                    }
                }
                Target::Section(key) => renderer.upload_section(key, &result.into_section()),
            }
        });
        true
    }

    fn mesh_next_section(
        &mut self,
        game: &Game,
        renderer: &mut Renderer,
        profiler: &mut Profiler,
    ) -> bool {
        if !self.can_start() {
            return false;
        }
        let Some(index) = self
            .stale_sections
            .iter()
            .rposition(|key| !self.jobs.column_in_flight(key.column()))
        else {
            return false;
        };
        let key = self.stale_sections.remove(index);
        if !self.jobs.enabled() {
            self.remesh_section(game, renderer, key);
            return true;
        }
        if !renderer.has_column(key.column()) {
            return true;
        }
        let id = self.jobs.begin(Target::Section(key), self.mesh_options);
        let job = profiler.encode_time.measure(&profiler.performance, || {
            encode_section_job(&game.world, &game.light, id, key, self.mesh_options)
        });
        self.queue_job(id, job);
        true
    }

    fn mesh_next_backlog(
        &mut self,
        game: &Game,
        renderer: &mut Renderer,
        profiler: &mut Profiler,
    ) -> bool {
        if !self.can_start() {
            return false;
        }
        let Some(key) = self.backlog.pop_front() else {
            return false;
        };
        self.mesh_column(game, renderer, profiler, key);
        true
    }

    fn queue_job(&mut self, id: u32, job: Option<Vec<u8>>) {
        match job {
            Some(job) => self.outbox.push_back(job),
            None => self.jobs.cancel(id),
        }
    }

    pub(crate) fn apply_edits(
        &mut self,
        edits: Edits,
        game: &Game,
        renderer: &mut Renderer,
        profiler: &mut Profiler,
        started: Option<f64>,
    ) {
        if edits.dirty.is_empty() {
            return;
        }
        self.stale_sections.retain(|key| !edits.dirty.contains(key));
        for key in edits.dirty {
            self.jobs.section_changed(key);
            self.column_edited(key.column());
            self.remesh_section(game, renderer, key);
        }
        self.queue_sections(edits.lit, game, renderer);
        if let (Some(now), Some(start)) = (profiler.now(), started) {
            profiler.edit_time.record(now - start);
        }
    }

    fn column_edited(&mut self, key: ColumnPos) {
        if self.jobs.column_changed(key) && !self.backlog.contains(&key) {
            self.backlog.push_front(key);
        }
    }

    pub(crate) fn remesh_around(&mut self, game: &Game, renderer: &mut Renderer, key: ColumnPos) {
        for key in std::iter::once(key).chain(NEIGHBORS.map(|n| key + n)) {
            self.column_edited(key);
            if renderer.has_column(key) {
                self.upload_column(&game.world, &game.light, renderer, key);
            }
        }
    }

    fn advance_loader(
        &mut self,
        game: &mut Game,
        renderer: &mut Renderer,
        profiler: &mut Profiler,
    ) -> bool {
        let progress = self.loader.advance(&mut game.world, |key| {
            profiler
                .generate_time
                .measure(&profiler.performance, || game.terrain.generate(key))
        });
        match progress {
            Progress::Done => return false,
            Progress::Generated(key) => {
                let changed = profiler.light_time.measure(&profiler.performance, || {
                    game.light.add_column(&game.world, key)
                });
                self.queue_sections(changed, game, renderer);
            }
            Progress::Mesh(key) => {
                if self.can_start() && self.backlog.is_empty() {
                    self.mesh_column(game, renderer, profiler, key);
                } else {
                    self.backlog.push_back(key);
                }
            }
        }
        true
    }

    fn rebuild_stale(
        &mut self,
        game: &Game,
        renderer: &mut Renderer,
        profiler: &mut Profiler,
    ) -> bool {
        if self.stale.is_empty() || !self.can_start() {
            return false;
        }
        if let Some(key) = self.stale.pop()
            && renderer.has_column(key)
        {
            self.mesh_column(game, renderer, profiler, key);
        }
        true
    }

    fn mesh_column(
        &mut self,
        game: &Game,
        renderer: &mut Renderer,
        profiler: &mut Profiler,
        key: ColumnPos,
    ) {
        if !self.jobs.enabled() {
            self.mesh_column_here(&game.world, &game.light, renderer, profiler, key);
            return;
        }
        let id = self.jobs.begin(Target::Column(key), self.mesh_options);
        let job = profiler.encode_time.measure(&profiler.performance, || {
            encode_column_job(&game.world, &game.light, id, key, self.mesh_options)
        });
        self.queue_job(id, job);
    }

    fn mesh_column_here(
        &self,
        world: &World,
        light: &LightMap,
        renderer: &mut Renderer,
        profiler: &mut Profiler,
        key: ColumnPos,
    ) {
        profiler.mesh_time.measure(&profiler.performance, || {
            self.upload_column(world, light, renderer, key)
        });
    }

    fn upload_column(
        &self,
        world: &World,
        light: &LightMap,
        renderer: &mut Renderer,
        key: ColumnPos,
    ) {
        let data = mesh_world_column(world, light, key, self.mesh_options)
            .expect("loader meshes only loaded columns");
        renderer.upload_column(key, &data);
    }

    fn unload_light(&mut self, game: &mut Game, renderer: &Renderer) {
        if game.light.len() <= game.world.len() {
            return;
        }
        let gone: Vec<ColumnPos> = game
            .light
            .keys()
            .filter(|&key| game.world.column(key).is_none())
            .collect();
        for key in gone {
            let changed = game.light.remove_column(&game.world, key);
            self.queue_sections(changed, game, renderer);
        }
    }

    fn queue_sections(&mut self, keys: Vec<SectionPos>, game: &Game, renderer: &Renderer) {
        if keys.is_empty() {
            return;
        }
        for key in keys {
            let meshed =
                renderer.has_column(key.column()) || self.jobs.column_in_flight(key.column());
            if meshed && !self.stale_sections.contains(&key) {
                self.stale_sections.push(key);
            }
        }
        let eye = game.camera().position;
        let distance = |key: &SectionPos| {
            let center = (key.0.as_vec3() + 0.5) * SECTION_SIZE as f32;
            (center - eye).length_squared()
        };
        self.stale_sections
            .sort_by(|a, b| distance(b).total_cmp(&distance(a)));
    }

    fn remesh_section(&self, game: &Game, renderer: &mut Renderer, key: SectionPos) {
        if !renderer.has_column(key.column()) {
            return;
        }
        let Some(data) = mesh_world_section(&game.world, &game.light, key, self.mesh_options)
        else {
            return;
        };
        renderer.upload_section(key, &data);
    }
}
