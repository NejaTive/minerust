export const WHEEL_STEP = 100
export const WHEEL_LINE_PX = 40
export const WHEEL_PAGE_PX = 800

export class WheelAccumulator {
  private accum = 0

  push(deltaY: number, deltaMode: number): -1 | 0 | 1 {
    const unit = deltaMode === 1 ? WHEEL_LINE_PX : deltaMode === 2 ? WHEEL_PAGE_PX : 1
    const delta = deltaY * unit
    if (Math.sign(delta) !== Math.sign(this.accum)) this.accum = 0
    this.accum += delta
    if (Math.abs(this.accum) < WHEEL_STEP) return 0
    const step = Math.sign(this.accum) as -1 | 1
    this.accum = 0
    return step
  }
}

export function stepSlot(slot: number, step: number, count: number): number {
  return (slot + step + count) % count
}
