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

/// Круг колонок, который `warm_up` грузит и мешует синхронно, чтобы первый кадр не был пустым.
const START_RADIUS: i32 = 2;
/// Бюджет потоковой загрузки на кадр, мс. Сам кадр без подгрузки занимает на CPU
/// ~1–1.5 мс, и 3 мс сверху оставляют запас даже при 120 Гц (8.3 мс на кадр).
/// В release это ~10 мешей колонки (~0.25 мс) за кадр: круг дальности 8 (~200 колонок)
/// заполняется быстрее, чем за секунду, а полёт 40 блоков/с требует меньше колонки
/// на кадр. Бюджет по времени, а не по числу колонок, сам подстраивается под dev-сборку
/// и слабые машины: кадр не проседает, просто круг заполняется дольше.
const STREAM_BUDGET_MS: f64 = 3.0;

/// Подгрузка мира вокруг камеры: генерация и свет колонок, меши, выгрузка.
pub(crate) struct Streamer {
    loader: Loader,
    mesh_options: MeshOptions,
    /// Колонки, чьи меши построены со старыми `mesh_options`, дальние первыми: их
    /// перестраивает `update` в бюджете кадра, с конца. До замены рисуются старые.
    stale: Vec<ColumnPos>,
    /// Секции, где после правки или подгрузки изменился свет, дальние первыми: их перестраивает `update` с конца, раньше
    /// подгрузки — их немного, и правка должна дойти до экрана сразу.
    stale_sections: Vec<SectionPos>,
    /// Учёт заданий в воркерах; без воркеров (`slots == 0`) меши строятся синхронно.
    jobs: MeshJobs,
    /// Колонки, которым загрузчик велел построить меш, но задание ещё не ушло (нет
    /// свободного места в воркерах), ближние первыми. Данные колонки к отправке берутся
    /// свежими.
    backlog: VecDeque<ColumnPos>,
    /// Закодированные задания, которые `Engine::take_mesh_job` ещё не забрал.
    outbox: VecDeque<Vec<u8>>,
    /// Результаты воркеров, ещё не поставленные на GPU: ставит `update` в своём бюджете.
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

    /// Синхронно грузит и мешует круг [`START_RADIUS`] вокруг камеры.
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
        // Порядок не важен: свет не зависит от порядка добавления колонок.
        for key in game.world.keys().collect::<Vec<_>>() {
            profiler.light_time.measure(&profiler.performance, || {
                game.light.add_column(&game.world, key)
            });
        }
        // Воркеров на старте нет: слоты открываются после `Engine::create`.
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

    /// Меши всех колонок перестраиваются постепенно, ближние первыми, в бюджете подгрузки.
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

    /// Колонки и секции в очередях загрузки и перестройки.
    pub(crate) fn pending(&self) -> usize {
        self.loader.pending()
            + self.stale.len()
            + self.stale_sections.len()
            + self.backlog.len()
            + self.jobs.len()
    }

    /// Заданий мешера в воркерах (с непоставленными результатами).
    pub(crate) fn jobs_in_flight(&self) -> usize {
        self.jobs.len()
    }

    /// Сколько заданий можно держать в воркерах; 0 — воркеров нет, меши строит основной
    /// поток. При переходе к нулю задания в полёте строятся заново синхронно.
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

    /// Следующее закодированное задание для воркера.
    pub(crate) fn take_job(&mut self) -> Option<Vec<u8>> {
        self.outbox.pop_front()
    }

    /// Результат воркера; на GPU попадёт в ближайшем `update`.
    pub(crate) fn push_result(&mut self, bytes: &[u8], mesh_ms: f64, profiler: &mut Profiler) {
        profiler.worker_time.record(mesh_ms);
        self.results.push_back(bytes.to_vec());
    }

    /// Подгрузка колонок вокруг камеры и перестройка устаревших мешей (`stale`), пока
    /// не выйдет [`STREAM_BUDGET_MS`]. Одна операция за кадр делается всегда, даже если
    /// бюджет съели раньше: иначе на медленной машине мир не загрузился бы никогда.
    pub(crate) fn update(
        &mut self,
        game: &mut Game,
        renderer: &mut Renderer,
        profiler: &mut Profiler,
    ) {
        let deadline = profiler.now().map(|now| now + STREAM_BUDGET_MS);
        // Буферы освобождаются вместе с мешем колонки.
        for key in self.loader.update(game.camera_column(), &mut game.world) {
            renderer.remove_column(key);
            self.jobs.unload(key);
            self.backlog.retain(|&queued| queued != key);
        }
        self.unload_light(game, renderer);
        let mut loader_done = false;
        for step in 0usize.. {
            // Перестройка чередуется с подгрузкой: при смене AO на ходу у игрока не
            // остаётся дыр на краю мира, а подгрузка не задерживает перестройку.
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
                // Работы нет: либо всё построено, либо остаток ждёт воркеров.
                if loader_done && self.is_idle() {
                    profiler.finish_load();
                }
                break;
            }
            // Без `performance` (в браузере он есть всегда) время не измерить:
            // остаёмся на минимуме, одна операция за кадр.
            let in_time = match (profiler.now(), deadline) {
                (Some(now), Some(deadline)) => now < deadline,
                _ => false,
            };
            if !in_time {
                break;
            }
        }
    }

    /// Нечего ни строить, ни ждать.
    fn is_idle(&self) -> bool {
        self.stale.is_empty()
            && self.stale_sections.is_empty()
            && self.backlog.is_empty()
            && self.jobs.is_empty()
    }

    /// Можно ли начать ещё один меш: в воркерах есть место, либо воркеров нет и меш
    /// строится на месте.
    fn can_start(&self) -> bool {
        !self.jobs.enabled() || self.jobs.free_slots() > 0
    }

    /// Ставит на GPU один пришедший результат. Устаревший (колонку выгрузили, блок
    /// успели поправить, колонку заказали заново) отбрасывает.
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
                    // Опции сменились, пока задание считалось: меш уже стоит, но со
                    // старым AO.
                    if ticket.options != self.mesh_options {
                        self.stale.push(key);
                    }
                }
                Target::Section(key) => renderer.upload_section(key, &result.into_section()),
            }
        });
        true
    }

    /// Отдаёт воркеру (или строит на месте) самую дальнюю секцию из `stale_sections`,
    /// чья колонка не ждёт результата: тот был бы старше и перезаписал бы секцию.
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

    /// Первый меш колонки из очереди, когда в воркерах появилось место.
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

    /// Кладёт закодированное задание в исходящую очередь; `None` — колонка уже
    /// выгружена, задание снимается.
    fn queue_job(&mut self, id: u32, job: Option<Vec<u8>>) {
        match job {
            Some(job) => self.outbox.push_back(job),
            None => self.jobs.cancel(id),
        }
    }

    /// Перестройка после правки блоков: задетые секции — сразу, вне бюджета, остальные
    /// с изменённым светом — в очередь.
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
            // Правка видна в тот же кадр, поэтому секция строится на месте; задания
            // воркеров со старыми блоками отсеются.
            self.jobs.section_changed(key);
            self.column_edited(key.column());
            self.remesh_section(game, renderer, key);
        }
        self.queue_sections(edits.lit, game, renderer);
        if let (Some(now), Some(start)) = (profiler.now(), started) {
            profiler.edit_time.record(now - start);
        }
    }

    /// Данные колонки поменялись: задание на неё целиком, если оно в полёте, устарело и
    /// колонку надо поставить на меш заново.
    fn column_edited(&mut self, key: ColumnPos) {
        if self.jobs.column_changed(key) && !self.backlog.contains(&key) {
            self.backlog.push_front(key);
        }
    }

    /// Перестраивает меши колонки и её соседей, если они уже есть. Соседей целиком, а не
    /// только граничные секции: импорт идёт раз при старте, до первого кадра, и простота
    /// тут дороже пары лишних мешей. Диагональные — из-за AO на углу колонки.
    pub(crate) fn remesh_around(&mut self, game: &Game, renderer: &mut Renderer, key: ColumnPos) {
        for key in std::iter::once(key).chain(NEIGHBORS.map(|n| key + n)) {
            self.column_edited(key);
            if renderer.has_column(key) {
                self.upload_column(&game.world, &game.light, renderer, key);
            }
        }
    }

    /// Один шаг загрузчика; `false`, если работы нет.
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

    /// Перестраивает ближайшую колонку из `stale` с текущими `mesh_options`. Колонку,
    /// чей меш загрузчик уже снял, пропускает. `false`, если очередь пуста или в
    /// воркерах нет места.
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

    /// Строит меш колонки: отдаёт воркеру, а без воркеров делает сам.
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

    /// Строит меш колонки и загружает её на GPU. Загрузчик отдаёт колонку на меш только
    /// вместе с 8 соседями, так что грани и AO на стыках верные сразу.
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

    /// Убирает свет колонок, которые загрузчик выгрузил из `World`. Загрузчик не
    /// сообщает о выгрузке, но в одном `update` колонки только убывают, так что хватает
    /// сравнить количество.
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

    /// Ставит секции в очередь на перестройку из-за света. Секции колонок без меша
    /// пропускает: загрузчик построит их целиком позже, уже с новым светом.
    fn queue_sections(&mut self, keys: Vec<SectionPos>, game: &Game, renderer: &Renderer) {
        if keys.is_empty() {
            return;
        }
        for key in keys {
            // Колонка в полёте меша ещё не на GPU, но её задание собрано по старому
            // свету: секция перестроится после результата.
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

    /// Перестраивает меш секции по текущему миру.
    /// Колонку без меша пропускает: загрузчик мешует её целиком позже, уже с правкой.
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
