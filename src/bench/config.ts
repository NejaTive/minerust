import { parseBenchParams } from './params'

const params = parseBenchParams(location.search)
export const BENCH = params.enabled
export const BENCH_DEBUG = params.debug

/** Сид замера: мир в памяти, IndexedDB не читается и не пишется. Тот же сид — в `bench_points` (terrain.rs). */
export const BENCH_SEED = 1337
/** Полдень (`daytime::NOON`): замеры идут при одном и том же свете. */
export const BENCH_TIME = 0.25
export const BENCH_RENDER_DISTANCE = 12

export interface BenchPoint {
  name: string
  /** `[x, y, z, yaw, pitch, mode]` как `Engine.camera_state()`; mode 1 — полёт: игрок висит на месте без ввода. */
  camera: number[]
}

/**
 * Точки подобраны тестом `bench_points` (terrain.rs) на сиде 1337: ground = `surface_height`
 * (64 и 69), глаза выше на 8 блоков. Расстояние между точками больше 1000 блоков:
 * при переходе перегружается весь круг (дальность 12 = 192 блока).
 */
export const BENCH_POINTS: BenchPoint[] = [
  { name: 'plains', camera: [-248.5, 72, 264.5, 0, -0.25, 1] },
  { name: 'forest-water', camera: [-1016.5, 77, 1416.5, 0, -0.25, 1] },
]
