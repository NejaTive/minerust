//! Мешер для Web Worker: отдельный wasm без wgpu и мира, только `mesh::job`. Основной
//! движок (`engine`) в каждый воркер не грузится: он больше и тянет GPU-код.
use wasm_bindgen::prelude::*;

/// Ставит хук паники; вызывается один раз при старте воркера.
#[wasm_bindgen]
pub fn init_worker() {
    console_error_panic_hook::set_once();
}

/// Строит меши по заданию `mesh::job::encode_*_job` и возвращает результат для
/// `mesh::job::decode_result`.
#[wasm_bindgen]
pub fn run_job(job: &[u8]) -> Vec<u8> {
    mesh::job::run_job(job)
}
