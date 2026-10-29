import { Engine, EngineSettings } from '@engine'
import type { Settings } from '../settings/settings'

/** Настройки из меню — в движок. Идемпотентно: вызывается и после каждого `Engine.create`. */
export function applySettings(engine: Engine, s: Settings) {
  const payload = new EngineSettings(
    s.renderDistance,
    s.ambientOcclusion,
    s.mouseSensitivity,
    s.fov,
  )
  try {
    engine.apply_settings(payload)
  } finally {
    payload.free()
  }
  // Замеры кадра нужны только отладочному HUD и режиму замера.
  engine.set_profiling(s.showDebug)
}

/** Выбранный слот — в движок. Слот живёт в клиенте, а не в движке: движок пересоздаётся (новый мир). */
export function applyBlock(engine: Engine, name: string | null) {
  // Пустая строка — пустая рука: ПКМ ничего не ставит.
  engine.select_block(name ?? '')
}
