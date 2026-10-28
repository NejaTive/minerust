use crate::api::WorldStats;
use render::FrameCounts;
use web_sys::Performance;

/// Среднее время одной операции между вызовами [`AverageTime::take_micros`].
#[derive(Default)]
pub(crate) struct AverageTime {
    total_ms: f64,
    count: u32,
    last_micros: u32,
}

impl AverageTime {
    pub(crate) fn measure<T>(
        &mut self,
        performance: &Option<Performance>,
        f: impl FnOnce() -> T,
    ) -> T {
        let Some(performance) = performance else {
            return f();
        };
        let start = performance.now();
        let result = f();
        self.record(performance.now() - start);
        result
    }

    /// Сразу `count` операций общей длительностью `total_ms`.
    pub(crate) fn record_total(&mut self, total_ms: f64, count: u32) {
        self.total_ms += total_ms;
        self.count += count;
    }

    pub(crate) fn record(&mut self, ms: f64) {
        self.total_ms += ms;
        self.count += 1;
    }

    fn take_micros(&mut self) -> u32 {
        if self.count > 0 {
            self.last_micros = (self.total_ms * 1000.0 / f64::from(self.count)) as u32;
            self.total_ms = 0.0;
            self.count = 0;
        }
        self.last_micros
    }
}

/// Размеры сцены для `stats()`: их знают мир, загрузчик и рендерер, не профайлер.
pub(crate) struct SceneStats {
    pub loaded_columns: usize,
    pub meshed_columns: usize,
    pub sections: usize,
    /// Колонки и секции в очередях загрузки и перестройки.
    pub pending: usize,
    pub drawn_sections: u32,
    pub light_kib: usize,
    pub counts: FrameCounts,
    pub gpu_kib: u64,
    pub gpu_used_kib: u64,
    pub gpu_timer: bool,
}

/// Замеры времени для `stats()`.
pub(crate) struct Profiler {
    /// `None`, если `window.performance` недоступен: тогда замеры нулевые.
    pub(crate) performance: Option<Performance>,
    pub(crate) generate_time: AverageTime,
    pub(crate) mesh_time: AverageTime,
    /// Расчёт света колонки при подгрузке (`LightMap::add_column`).
    pub(crate) light_time: AverageTime,
    /// Правка блока: свет и немедленная перестройка задетых секций.
    pub(crate) edit_time: AverageTime,
    /// GPU за кадр: render pass по `timestamp-query`, приходит с задержкой в кадры.
    pub(crate) gpu_frame_time: AverageTime,
    /// Замеры кадра включены (`set_profiling`): иначе нет лишних вызовов `performance.now`.
    profiling: bool,
    /// CPU за кадр: подгрузка и меш в бюджете.
    stream_frame_time: AverageTime,
    /// CPU за кадр: всё остальное до `finish` (физика, матрицы, отсечение, запись команд).
    build_frame_time: AverageTime,
    /// CPU за кадр: `finish`, `submit`, `present`.
    submit_frame_time: AverageTime,
    /// Момент (мс `performance.now`) начала текущей загрузки мира: старт или `set_camera_state`.
    load_started: Option<f64>,
    /// Длительность последней завершённой загрузки, мс; 0, пока идёт.
    load_ms: u32,
}

impl Profiler {
    pub(crate) fn new() -> Self {
        let performance = web_sys::window().and_then(|w| w.performance());
        let load_started = performance.as_ref().map(|p| p.now());
        Profiler {
            performance,
            generate_time: AverageTime::default(),
            mesh_time: AverageTime::default(),
            light_time: AverageTime::default(),
            edit_time: AverageTime::default(),
            gpu_frame_time: AverageTime::default(),
            profiling: false,
            stream_frame_time: AverageTime::default(),
            build_frame_time: AverageTime::default(),
            submit_frame_time: AverageTime::default(),
            load_started,
            load_ms: 0,
        }
    }

    /// `performance.now`, мс.
    pub(crate) fn now(&self) -> Option<f64> {
        self.performance.as_ref().map(|p| p.now())
    }

    /// Время `performance.now`, если замеры кадра включены.
    pub(crate) fn tick(&self) -> Option<f64> {
        if self.profiling { self.now() } else { None }
    }

    pub(crate) fn set_profiling(&mut self, enabled: bool) {
        self.profiling = enabled;
    }

    /// Времена вех кадра: начало, до и после подгрузки, конец записи команд, конец `present`.
    pub(crate) fn record_frame(&mut self, ticks: [Option<f64>; 5]) {
        if let [
            Some(start),
            Some(stream),
            Some(streamed),
            Some(encoded),
            Some(end),
        ] = ticks
        {
            self.stream_frame_time.record(streamed - stream);
            self.build_frame_time
                .record((stream - start) + (encoded - streamed));
            self.submit_frame_time.record(end - encoded);
        }
    }

    /// Начинает отсчёт полной загрузки заново.
    pub(crate) fn restart_load(&mut self) {
        self.load_started = self.now();
        self.load_ms = 0;
    }

    /// Мир догружен целиком: фиксирует время с начала загрузки (один раз на загрузку).
    pub(crate) fn finish_load(&mut self) {
        let Some(start) = self.load_started.take() else {
            return;
        };
        if let Some(now) = self.now() {
            // Не меньше 1: ноль в `stats()` значит «ещё грузится».
            self.load_ms = ((now - start) as u32).max(1);
        }
    }

    pub(crate) fn load_ms(&self) -> u32 {
        self.load_ms
    }

    pub(crate) fn stats(&mut self, scene: &SceneStats) -> WorldStats {
        let counts = scene.counts;
        WorldStats {
            columns: scene.loaded_columns as u32,
            meshed_columns: scene.meshed_columns as u32,
            meshed_sections: scene.sections as u32,
            queued: scene.pending as u32,
            generate_us: self.generate_time.take_micros(),
            mesh_us: self.mesh_time.take_micros(),
            drawn_sections: scene.drawn_sections,
            light_us: self.light_time.take_micros(),
            edit_us: self.edit_time.take_micros(),
            light_kib: scene.light_kib as u32,
            stream_us: self.stream_frame_time.take_micros(),
            build_us: self.build_frame_time.take_micros(),
            submit_us: self.submit_frame_time.take_micros(),
            solid_vertices: counts.solid_vertices,
            solid_indices: counts.solid_indices,
            water_vertices: counts.water_vertices,
            water_indices: counts.water_indices,
            draw_calls: counts.draw_calls,
            gpu_kib: scene.gpu_kib as u32,
            gpu_used_kib: scene.gpu_used_kib as u32,
            gpu_frame_us: self.gpu_frame_time.take_micros(),
            gpu_timer: scene.gpu_timer,
            load_ms: self.load_ms,
        }
    }
}
