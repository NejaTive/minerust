import { parseBenchParams } from './params'

const params = parseBenchParams(location.search)
export const BENCH = params.enabled
export const BENCH_DEBUG = params.debug
export const BENCH_WORKERS = params.workers

export const BENCH_SEED = 1337
export const BENCH_TIME = 0.25
export const BENCH_RENDER_DISTANCE = 12

export interface BenchPoint {
  name: string
  camera: number[]
}

export const BENCH_POINTS: BenchPoint[] = [
  { name: 'plains', camera: [-248.5, 72, 264.5, 0, -0.25, 1] },
  { name: 'forest-water', camera: [-1016.5, 77, 1416.5, 0, -0.25, 1] },
]
