import { Engine, EngineSettings } from '@engine'
import type { Settings } from '../settings/settings'

export function applySettings(engine: Engine, s: Settings) {
  const payload = new EngineSettings(
    s.renderDistance,
    s.ambientOcclusion,
    s.clouds,
    s.autoStep,
    s.mouseSensitivity,
    s.fov,
  )
  try {
    engine.apply_settings(payload)
  } finally {
    payload.free()
  }
  engine.set_profiling(s.showDebug)
}

export function applyBlock(engine: Engine, name: string | null) {
  engine.select_block(name ?? '')
}
