export const MOUSE_LOCK_GUARD_MS = 100

export const MOUSE_LOCK_ANOMALY = 250

export type MouseGuard = {
  until: number
  skip: number
}

export function armMouseGuard(now: number): MouseGuard {
  return { until: now + MOUSE_LOCK_GUARD_MS, skip: 1 }
}

export type MouseFilterResult = {
  guard: MouseGuard | null
  delta: { dx: number; dy: number } | null
}

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
