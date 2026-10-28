import type { Engine, WorldStats } from '@engine'
import { BENCH_POINTS, BENCH_SEED } from './config'
import { FrameRecorder, frameSummary, round } from './recorder'

/** Сколько секунд стоять на точке после полной загрузки. */
const STAND_MS = 10_000
/** Если мир не догрузился за это время, замер прерывается с ошибкой. */
const LOAD_TIMEOUT_MS = 180_000

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
    gpuUsedMiB: round(s.gpuUsedKib / 1024),
  }
}

export interface BenchResult {
  done: boolean
  error?: string
  /** Версия и коммит сборки: замеры разных версий сравниваются по ним. */
  version: string
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
  settings: { renderDistance: number; ambientOcclusion: boolean },
) {
  const first = engine()
  const result: BenchResult = {
    done: false,
    version: __APP_COMMIT__ ? `${__APP_VERSION__}+${__APP_COMMIT__}` : __APP_VERSION__,
    build: (first.constructor as typeof Engine).build_profile(),
    seed: BENCH_SEED,
    renderDistance: settings.renderDistance,
    ambientOcclusion: settings.ambientOcclusion,
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
        if (performance.now() - started > LOAD_TIMEOUT_MS)
          throw new Error(`${point.name}: load timeout`)
        await sleep(50)
      }
      const loaded = current.stats()
      const loading = frameSummary(recorder)
      recorder.reset()
      await sleep(STAND_MS)
      const steady = current.stats()
      result.points.push({
        name: point.name,
        loadMs: loaded.loadMs,
        loading: {
          frame: loading,
          cpuUs: { stream: loaded.streamUs, build: loaded.buildUs, submit: loaded.submitUs },
          gpuFrameUs: loaded.gpuTimer ? loaded.gpuFrameUs : null,
        },
        steady: {
          frame: frameSummary(recorder),
          cpuUs: { stream: steady.streamUs, build: steady.buildUs, submit: steady.submitUs },
          // null — адаптер без timestamp-query; 0 при gpuTimer — замеров ещё нет.
          gpuFrameUs: steady.gpuTimer ? steady.gpuFrameUs : null,
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
