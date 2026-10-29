/// <reference lib="webworker" />
import init, { init_worker, run_job } from '@engine-worker'

const ready = init().then(() => {
  init_worker()
  self.postMessage({ type: 'ready' })
})

self.onmessage = async (event: MessageEvent<ArrayBuffer>) => {
  await ready
  const started = performance.now()
  const result = run_job(new Uint8Array(event.data))
  const ms = performance.now() - started
  // Буфер уходит без копии: из `Vec<u8>` wasm-bindgen уже сделал отдельный Uint8Array.
  self.postMessage({ type: 'result', buffer: result.buffer, ms }, [result.buffer])
}
