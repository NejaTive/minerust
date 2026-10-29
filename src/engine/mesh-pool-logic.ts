/** В полёте на воркер: пока один считает, второй уже ждёт в очереди и воркер не простаивает. */
export const JOBS_PER_WORKER = 2
export const MAX_WORKERS = 4

/**
 * Воркеров столько, сколько ядер минус одно (основной поток), но не больше `MAX_WORKERS`:
 * выше выигрыша нет, а каждый воркер держит свою копию wasm. Не меньше одного, в том числе
 * если браузер не сообщил число ядер.
 */
export function workerCount(hardwareConcurrency: number | undefined): number {
  const cores = hardwareConcurrency && hardwareConcurrency > 0 ? hardwareConcurrency : 2
  return Math.max(1, Math.min(MAX_WORKERS, cores - 1))
}

/** Воркер для следующего задания: наименее загруженный из тех, где есть место. -1 — все заняты. */
export function pickWorker(inFlight: readonly number[], perWorker = JOBS_PER_WORKER): number {
  let best = -1
  for (let i = 0; i < inFlight.length; i++) {
    if (inFlight[i] < perWorker && (best < 0 || inFlight[i] < inFlight[best])) best = i
  }
  return best
}

/** Сколько заданий движок может держать в воркерах, когда готовы `ready` из них. */
export function slotsFor(ready: number, perWorker = JOBS_PER_WORKER): number {
  return Math.max(0, ready) * perWorker
}
