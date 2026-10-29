/**
 * После pointer lock Chrome (особенно на macOS) часто шлёт первый mousemove
 * с огромным movementX/Y — разница экранных координат до/после захвата, а не жест игрока.
 */

/** Сколько мс после захвата считать «только что захвачен» и резать аномалии. */
export const MOUSE_LOCK_GUARD_MS = 100

/**
 * Порог одного события: ~четверть Full HD по оси — нормальный жест мыши за кадр
 * на порядок меньше; скачок Chrome после lock часто тысячи пикселей.
 */
export const MOUSE_LOCK_ANOMALY = 250

export type MouseGuard = {
  /** До этого `performance.now()` ещё режем аномалии. */
  until: number
  /** Сколько следующих событий выбросить целиком (первый кадр после lock). */
  skip: number
}

/** Вооружить фильтр сразу после успешного pointer lock. */
export function armMouseGuard(now: number): MouseGuard {
  return { until: now + MOUSE_LOCK_GUARD_MS, skip: 1 }
}

export type MouseFilterResult = {
  guard: MouseGuard | null
  /** null — событие не передавать в движок. */
  delta: { dx: number; dy: number } | null
}

/**
 * Фильтр delta после захвата курсора.
 * Первое событие всегда отбрасывается; в окне `MOUSE_LOCK_GUARD_MS` ещё и аномалии.
 */
export function filterMouseDelta(
  guard: MouseGuard | null,
  now: number,
  dx: number,
  dy: number,
): MouseFilterResult {
  if (!guard) return { guard: null, delta: { dx, dy } }

  if (guard.skip > 0) {
    const skip = guard.skip - 1
    const next: MouseGuard | null =
      skip === 0 && now >= guard.until ? null : { until: guard.until, skip }
    return { guard: next, delta: null }
  }

  if (now >= guard.until) {
    return { guard: null, delta: { dx, dy } }
  }

  if (Math.abs(dx) > MOUSE_LOCK_ANOMALY || Math.abs(dy) > MOUSE_LOCK_ANOMALY) {
    return { guard, delta: null }
  }
  return { guard, delta: { dx, dy } }
}
