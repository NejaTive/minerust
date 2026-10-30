export const JOBS_PER_WORKER = 2
export const MAX_WORKERS = 4

export function workerCount(hardwareConcurrency: number | undefined): number {
  const cores = hardwareConcurrency && hardwareConcurrency > 0 ? hardwareConcurrency : 2
  return Math.max(1, Math.min(MAX_WORKERS, cores - 1))
}

export function pickWorker(inFlight: readonly number[], perWorker = JOBS_PER_WORKER): number {
  let best = -1
  for (let i = 0; i < inFlight.length; i++) {
    if (inFlight[i] < perWorker && (best < 0 || inFlight[i] < inFlight[best])) best = i
  }
  return best
}

export function slotsFor(ready: number, perWorker = JOBS_PER_WORKER): number {
  return Math.max(0, ready) * perWorker
}
