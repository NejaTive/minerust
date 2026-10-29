import type { Engine, WorldStats } from '@engine'
import { BENCH_POINTS, BENCH_SEED, BENCH_WORKERS } from './config'
import { FrameRecorder, frameSummary, round } from './recorder'

const STAND_MS = 10_000
const LOAD_TIMEOUT_MS = 180_000
const FLIGHT_MS = 20_000
const TAP_GAP_MS = 80

function geometry(s: WorldStats) {
  return {
    solidQuads: s.solidQuads,
    cutoutQuads: s.cutoutQuads,
    waterQuads: s.waterQuads,
    drawCalls: s.drawCalls,
    drawnSections: s.drawnSections,
    meshedSections: s.meshedSections,
    gpuMiB: round(s.gpuKib / 1024),
    gpuUsedMiB: round(s.gpuUsedKib / 1024),
  }
}

function workerUs(s: WorldStats) {
  return { worker: s.workerMeshUs, encode: s.encodeUs, apply: s.applyUs }
}

export interface BenchResult {
  done: boolean
  error?: string
  version: string
  build: string
  seed: number
  workers: boolean
  renderDistance: number
  ambientOcclusion: boolean
  canvas: [number, number]
  points: object[]
  flight?: object
}

declare global {
  interface Window {
    __benchResult?: BenchResult
    __benchEngine?: Engine
  }
}

const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms))

async function runFlight(current: Engine, recorder: FrameRecorder) {
  const start = BENCH_POINTS[0].camera
  current.set_camera_state(Float32Array.from(start))
  const started = performance.now()
  while (current.load_ms() === 0) {
    if (performance.now() - started > LOAD_TIMEOUT_MS) throw new Error('flight: load timeout')
    await sleep(50)
  }
  current.stats()
  recorder.reset()
  current.set_key('KeyW', true)
  await sleep(TAP_GAP_MS)
  current.set_key('KeyW', false)
  await sleep(TAP_GAP_MS)
  current.set_key('KeyW', true)
  await sleep(FLIGHT_MS)
  current.set_key('KeyW', false)
  const stats = current.stats()
  return {
    name: 'flight',
    frame: frameSummary(recorder),
    cpuUs: { stream: stats.streamUs, build: stats.buildUs, submit: stats.submitUs },
    gpuFrameUs: stats.gpuTimer ? stats.gpuFrameUs : null,
    perColumnUs: {
      generate: stats.generateUs,
      light: stats.lightUs,
      mesh: stats.meshUs,
      ...workerUs(stats),
    },
    meshJobs: stats.meshJobs,
    columns: stats.columns,
    meshedColumns: stats.meshedColumns,
  }
}

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
    workers: BENCH_WORKERS,
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
      current.stats()
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
          gpuFrameUs: steady.gpuTimer ? steady.gpuFrameUs : null,
          ...geometry(steady),
        },
        perColumnUs: {
          generate: loaded.generateUs,
          light: loaded.lightUs,
          mesh: loaded.meshUs,
          ...workerUs(loaded),
        },
        columns: loaded.columns,
        meshedColumns: loaded.meshedColumns,
        lightKib: loaded.lightKib,
      })
    }
    result.flight = await runFlight(engine(), recorder)
  } catch (e) {
    result.error = String(e)
  }
  result.done = true
  console.log('bench', result)
}
