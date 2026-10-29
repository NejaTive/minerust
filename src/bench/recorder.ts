export function percentile(values: number[], p: number): number {
  if (values.length === 0) return 0
  const sorted = [...values].sort((a, b) => a - b)
  const index = Math.min(sorted.length - 1, Math.ceil((p / 100) * sorted.length) - 1)
  return sorted[Math.max(0, index)]
}

export const round = (value: number, digits = 2) => Number(value.toFixed(digits))
export const mean = (values: number[]) =>
  values.length === 0 ? 0 : values.reduce((a, b) => a + b, 0) / values.length

export class FrameRecorder {
  deltas: number[] = []
  renders: number[] = []
  private last: number | undefined

  reset() {
    this.deltas = []
    this.renders = []
    this.last = undefined
  }

  push(now: number, renderMs: number) {
    if (this.last !== undefined) this.deltas.push(now - this.last)
    this.last = now
    this.renders.push(renderMs)
  }
}

export function frameSummary(rec: FrameRecorder) {
  const total = rec.deltas.reduce((a, b) => a + b, 0)
  return {
    frames: rec.deltas.length,
    medianMs: round(percentile(rec.deltas, 50)),
    p95Ms: round(percentile(rec.deltas, 95)),
    maxMs: round(Math.max(0, ...rec.deltas)),
    fps: round(total > 0 ? (rec.deltas.length * 1000) / total : 0, 1),
    renderMeanMs: round(mean(rec.renders)),
    renderP95Ms: round(percentile(rec.renders, 95)),
  }
}
