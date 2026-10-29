import { describe, expect, it } from 'vitest'
import {
  MOUSE_LOCK_ANOMALY,
  MOUSE_LOCK_GUARD_MS,
  armMouseGuard,
  filterMouseDelta,
} from './mouse-guard'

describe('filterMouseDelta', () => {
  it('без охраны пропускает любые дельты', () => {
    expect(filterMouseDelta(null, 0, 9999, -9999)).toEqual({
      guard: null,
      delta: { dx: 9999, dy: -9999 },
    })
  })

  it('первое событие после lock отбрасывает', () => {
    const g = armMouseGuard(1000)
    const r = filterMouseDelta(g, 1000, 3, -2)
    expect(r.delta).toBeNull()
    expect(r.guard?.skip).toBe(0)
  })

  it('в окне охраны режет аномальные дельты', () => {
    let g: ReturnType<typeof armMouseGuard> | null = armMouseGuard(1000)
    g = filterMouseDelta(g, 1001, 1, 1).guard
    expect(g).not.toBeNull()
    const big = filterMouseDelta(g, 1002, MOUSE_LOCK_ANOMALY + 1, 0)
    expect(big.delta).toBeNull()
    const ok = filterMouseDelta(g, 1003, 10, -10)
    expect(ok.delta).toEqual({ dx: 10, dy: -10 })
  })

  it('после окна охраны аномалии больше не режет', () => {
    const g = armMouseGuard(1000)
    const skipped = filterMouseDelta(g, 1000 + MOUSE_LOCK_GUARD_MS, 1, 1)
    expect(skipped.delta).toBeNull()
    const r = filterMouseDelta(skipped.guard, 1000 + MOUSE_LOCK_GUARD_MS + 1, 5000, 5000)
    expect(r.guard).toBeNull()
    expect(r.delta).toEqual({ dx: 5000, dy: 5000 })
  })
})
