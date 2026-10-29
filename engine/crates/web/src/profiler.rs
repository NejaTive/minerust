use crate::api::WorldStats;
use render::FrameCounts;
use web_sys::Performance;

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

pub(crate) struct SceneStats {
    pub loaded_columns: usize,
    pub meshed_columns: usize,
    pub sections: usize,
    pub pending: usize,
    pub mesh_jobs: usize,
    pub drawn_sections: u32,
    pub light_kib: usize,
    pub counts: FrameCounts,
    pub gpu_kib: u64,
    pub gpu_used_kib: u64,
    pub gpu_timer: bool,
}

pub(crate) struct Profiler {
    pub(crate) performance: Option<Performance>,
    pub(crate) generate_time: AverageTime,
    pub(crate) mesh_time: AverageTime,
    pub(crate) encode_time: AverageTime,
    pub(crate) apply_time: AverageTime,
    pub(crate) worker_time: AverageTime,
    pub(crate) light_time: AverageTime,
    pub(crate) edit_time: AverageTime,
    pub(crate) gpu_frame_time: AverageTime,
    profiling: bool,
    stream_frame_time: AverageTime,
    build_frame_time: AverageTime,
    submit_frame_time: AverageTime,
    load_started: Option<f64>,
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
            encode_time: AverageTime::default(),
            apply_time: AverageTime::default(),
            worker_time: AverageTime::default(),
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

    pub(crate) fn now(&self) -> Option<f64> {
        self.performance.as_ref().map(|p| p.now())
    }

    pub(crate) fn tick(&self) -> Option<f64> {
        if self.profiling { self.now() } else { None }
    }

    pub(crate) fn set_profiling(&mut self, enabled: bool) {
        self.profiling = enabled;
    }

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

    pub(crate) fn restart_load(&mut self) {
        self.load_started = self.now();
        self.load_ms = 0;
    }

    pub(crate) fn finish_load(&mut self) {
        let Some(start) = self.load_started.take() else {
            return;
        };
        if let Some(now) = self.now() {
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
            mesh_jobs: scene.mesh_jobs as u32,
            encode_us: self.encode_time.take_micros(),
            apply_us: self.apply_time.take_micros(),
            worker_mesh_us: self.worker_time.take_micros(),
            generate_us: self.generate_time.take_micros(),
            mesh_us: self.mesh_time.take_micros(),
            drawn_sections: scene.drawn_sections,
            light_us: self.light_time.take_micros(),
            edit_us: self.edit_time.take_micros(),
            light_kib: scene.light_kib as u32,
            stream_us: self.stream_frame_time.take_micros(),
            build_us: self.build_frame_time.take_micros(),
            submit_us: self.submit_frame_time.take_micros(),
            solid_quads: counts.solid_quads,
            cutout_quads: counts.cutout_quads,
            water_quads: counts.water_quads,
            draw_calls: counts.draw_calls,
            gpu_kib: scene.gpu_kib as u32,
            gpu_used_kib: scene.gpu_used_kib as u32,
            gpu_frame_us: self.gpu_frame_time.take_micros(),
            gpu_timer: scene.gpu_timer,
            load_ms: self.load_ms,
        }
    }
}
