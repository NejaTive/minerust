use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn init_worker() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn run_job(job: &[u8]) -> Vec<u8> {
    mesh::job::run_job(job)
}
