import type { Engine } from '@engine'
import type { GraphicsApi } from './settings'
import { type WorldStats, parseStats } from './stats'

/** Режим замера: `?bench` в адресе. Дополнительно `&api=webgpu|webgl2` выбирает API, не трогая сохранённый. */
const params = new URLSearchParams(location.search)
export const BENCH = params.has('bench')

/** `&debug=0` — как обычная игра с выключенным `showDebug`: без замеров кадра и HUD (для сравнения накладных). */
export const BENCH_DEBUG = params.get('debug') !== '0'

export function benchApi(): GraphicsApi | undefined {
  const api = params.get('api')
  return api === 'webgpu' || api === 'webgl2' ? api : undefined
}

/** Сид замера: мир в памяти, IndexedDB не читается и не пишется. Тот же сид — в `bench_points` (terrain.rs). */
export const BENCH_SEED = 1337
/** Полдень (`daytime::NOON`): замеры идут при одном и том же свете. */
export const BENCH_TIME = 0.25
export const BENCH_RENDER_DISTANCE = 12
/** Сколько секунд стоять на точке после полной загрузки. */
const STAND_MS = 10_000
/** Если мир не догрузился за это время, замер прерывается с ошибкой. */
const LOAD_TIMEOUT_MS = 180_000

interface BenchPoint {
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

export function percentile(values: number[], p: number): number {
  if (values.length === 0) return 0
  const sorted = [...values].sort((a, b) => a - b)
  const index = Math.min(sorted.length - 1, Math.ceil((p / 100) * sorted.length) - 1)
  return sorted[Math.max(0, index)]
}

const round = (value: number, digits = 2) => Number(value.toFixed(digits))
const mean = (values: number[]) =>
  values.length === 0 ? 0 : values.reduce((a, b) => a + b, 0) / values.length

/** Время кадров между rAF и время самого `render()` в JS (CPU wasm-вызова). */
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

function frameSummary(rec: FrameRecorder) {
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

function geometry(s: WorldStats) {
  return {
    solidVertices: s.solidVertices,
    solidIndices: s.solidIndices,
    waterVertices: s.waterVertices,
    waterIndices: s.waterIndices,
    drawCalls: s.drawCalls,
    drawnSections: s.drawnSections,
    meshedSections: s.meshedSections,
    gpuMiB: round(s.gpuKib / 1024),
  }
}

export interface BenchResult {
  done: boolean
  error?: string
  /** Версия и коммит сборки: замеры разных версий сравниваются по ним. */
  version: string
  backend: string
  build: string
  seed: number
  renderDistance: number
  ambientOcclusion: boolean
  canvas: [number, number]
  points: object[]
}

declare global {
  interface Window {
    __benchResult?: BenchResult
    /** Движок режима `?bench`: для проверки из консоли (время суток, камера). */
    __benchEngine?: Engine
  }
}

const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms))

/**
 * Две точки подряд: переход → ждём полной загрузки → стоим `STAND_MS` → сводка.
 * Сводка пишется в `window.__benchResult` по мере готовности (`done` — после последней точки)
 * и целиком в `console.log`.
 */
export async function runBench(
  engine: () => Engine,
  recorder: FrameRecorder,
  canvas: HTMLCanvasElement,
) {
  const first = engine()
  const result: BenchResult = {
    done: false,
    version: __APP_COMMIT__ ? `${__APP_VERSION__}+${__APP_COMMIT__}` : __APP_VERSION__,
    backend: first.backend(),
    build: (first.constructor as typeof Engine).build_profile(),
    seed: BENCH_SEED,
    renderDistance: first.render_distance(),
    ambientOcclusion: first.ambient_occlusion(),
    canvas: [canvas.width, canvas.height],
    points: [],
  }
  window.__benchResult = result
  try {
    for (const point of BENCH_POINTS) {
      const current = engine()
      current.set_camera_state(Float32Array.from(point.camera))
      current.stats() // сбрасывает средние: дальше они относятся только к этой загрузке
      recorder.reset()
      const started = performance.now()
      while (current.load_ms() === 0) {
        if (performance.now() - started > LOAD_TIMEOUT_MS) throw new Error(`${point.name}: load timeout`)
        await sleep(50)
      }
      const loaded = parseStats(current.stats())
      const loading = frameSummary(recorder)
      recorder.reset()
      await sleep(STAND_MS)
      const steady = parseStats(current.stats())
      result.points.push({
        name: point.name,
        loadMs: loaded.loadMs,
        loading: {
          frame: loading,
          cpuUs: { stream: loaded.streamUs, build: loaded.buildUs, submit: loaded.submitUs },
        },
        steady: {
          frame: frameSummary(recorder),
          cpuUs: { stream: steady.streamUs, build: steady.buildUs, submit: steady.submitUs },
          ...geometry(steady),
        },
        perColumnUs: { generate: loaded.generateUs, light: loaded.lightUs, mesh: loaded.meshUs },
        columns: loaded.columns,
        meshedColumns: loaded.meshedColumns,
        lightKib: loaded.lightKib,
      })
    }
  } catch (e) {
    result.error = String(e)
  }
  result.done = true
  console.log('bench', result)
}
