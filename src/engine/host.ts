import { Engine, type WorldStats } from '@engine'
import { NoWebGpuError } from './gpu'
import type { PlayerInfo, TimeInfo } from './types'

export interface FrameSink {
  fps(value: number): void
  stats(value: WorldStats): void
  time(value: TimeInfo): void
  player(value: PlayerInfo): void
  rendered?(now: number, renderMs: number): void
  afterRender?(): void
}

export function fitCanvas(canvas: HTMLCanvasElement, engine?: Engine) {
  const dpr = window.devicePixelRatio
  canvas.width = Math.max(1, Math.floor(canvas.clientWidth * dpr))
  canvas.height = Math.max(1, Math.floor(canvas.clientHeight * dpr))
  engine?.resize(canvas.width, canvas.height)
}

export async function createEngine(
  canvas: HTMLCanvasElement,
  seed: number,
  camera?: ArrayLike<number>,
): Promise<Engine> {
  fitCanvas(canvas)
  try {
    return await Engine.create(canvas, camera ? Float32Array.from(camera) : undefined, seed)
  } catch (e) {
    const text = e instanceof Error ? e.message : String(e)
    throw text.startsWith('WebGPU ') ? new NoWebGpuError(text) : e
  }
}

export class FrameLoop {
  private rafId = 0
  private observer: ResizeObserver | undefined

  constructor(
    private readonly engine: Engine,
    private readonly canvas: HTMLCanvasElement,
    private readonly sink: FrameSink,
  ) {}

  start() {
    this.observer = new ResizeObserver(() => fitCanvas(this.canvas, this.engine))
    this.observer.observe(this.canvas)
    let frames = 0
    let lastFpsTime = performance.now()
    let player: PlayerInfo | undefined
    const { engine, sink } = this
    const frame = (now: number) => {
      const renderStart = sink.rendered ? performance.now() : 0
      engine.render(now / 1000)
      sink.rendered?.(now, performance.now() - renderStart)
      sink.afterRender?.()
      frames++
      const mode = engine.player_mode() as PlayerInfo['mode']
      const onGround = engine.on_ground()
      if (player?.mode !== mode || player.onGround !== onGround) {
        player = { mode, onGround }
        sink.player(player)
      }
      if (now - lastFpsTime >= 1000) {
        sink.fps(frames)
        sink.stats(engine.stats())
        sink.time({ time: engine.time_of_day(), phase: engine.day_phase() })
        frames = 0
        lastFpsTime = now
      }
      this.rafId = requestAnimationFrame(frame)
    }
    this.rafId = requestAnimationFrame(frame)
  }

  stop() {
    cancelAnimationFrame(this.rafId)
    this.observer?.disconnect()
    this.observer = undefined
  }
}
