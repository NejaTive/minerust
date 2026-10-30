import type { Engine } from '@engine'
import { JOBS_PER_WORKER, pickWorker, slotsFor, workerCount } from './mesh-pool-logic'

declare global {
  interface Window {
    __meshWorkers?: number
  }
}

let live = 0
const setLive = (value: number) => {
  live = value
  window.__meshWorkers = live
}

type WorkerMessage =
  | { type: 'ready' }
  | { type: 'result'; buffer: ArrayBuffer; ms: number }

export class MeshPool {
  private readonly workers: Worker[] = []
  private readonly inFlight: number[] = []
  private ready = 0
  private disposed = false

  constructor(
    private readonly engine: Engine,
    count = workerCount(navigator.hardwareConcurrency),
  ) {
    for (let i = 0; i < count; i++) this.spawn(i)
    setLive(live + count)
  }

  private spawn(index: number) {
    const worker = new Worker(new URL('./mesh.worker.ts', import.meta.url), { type: 'module' })
    this.workers.push(worker)
    this.inFlight.push(0)
    worker.onmessage = (event: MessageEvent<WorkerMessage>) => {
      if (this.disposed) return
      const message = event.data
      if (message.type === 'ready') {
        this.ready++
        this.engine.set_mesh_slots(slotsFor(this.ready))
      } else {
        this.inFlight[index]--
        this.engine.submit_mesh_result(new Uint8Array(message.buffer), message.ms)
      }
    }
    worker.onerror = (event) => {
      console.error('mesh worker failed', event.message)
      this.dispose()
    }
  }

  pump() {
    if (this.disposed) return
    for (;;) {
      const worker = pickWorker(this.inFlight, JOBS_PER_WORKER)
      if (worker < 0) return
      const job = this.engine.take_mesh_job()
      if (!job) return
      this.inFlight[worker]++
      this.workers[worker].postMessage(job.buffer, [job.buffer])
    }
  }

  dispose() {
    if (this.disposed) return
    this.disposed = true
    for (const worker of this.workers) worker.terminate()
    setLive(live - this.workers.length)
    try {
      this.engine.set_mesh_slots(0)
    } catch {
      return
    }
  }
}
