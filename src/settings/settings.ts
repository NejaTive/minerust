import type { KeyValueStore } from './store'

// Ключ дальности остался прежним, чтобы обновление не сбросило её у игрока.
export const KEYS = {
  renderDistance: 'minerust.renderDistance',
  ambientOcclusion: 'minerust.ambientOcclusion',
  mouseSensitivity: 'minerust.mouseSensitivity',
  fov: 'minerust.fov',
  showDebug: 'minerust.showDebug',
} as const
// Ключи убранных настроек (панель управления, выбор графического API): чистим, чтобы не копился мусор.
const OBSOLETE_KEYS = ['minerust.controlsHidden', 'minerust.graphicsApi']

/** Настройки, которыми управляет движок; пределы и значения по умолчанию даёт он же. */
export interface EngineSettings {
  renderDistance: number
  ambientOcclusion: boolean
  mouseSensitivity: number
  fov: number
}

/** Настройки игры. `showDebug` — чисто клиентская, движок о ней не знает. */
export interface Settings extends EngineSettings {
  showDebug: boolean
}

/** Пределы из `settings_limits()` движка. */
export interface SettingsLimits {
  renderDistanceMin: number
  renderDistanceMax: number
  mouseSensitivityMin: number
  mouseSensitivityMax: number
  fovMin: number
  fovMax: number
}

export function defaultSettings(engineDefaults: EngineSettings): Settings {
  return { ...engineDefaults, showDebug: false }
}

export function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value))
}

function readNumber(
  store: KeyValueStore,
  key: string,
  min: number,
  max: number,
  fallback: number,
): number {
  const raw = store.get(key)
  const value = raw === null ? Number.NaN : Number(raw)
  if (!Number.isFinite(value)) return fallback
  return clamp(value, min, max)
}

function readBool(store: KeyValueStore, key: string, fallback: boolean): boolean {
  const raw = store.get(key)
  return raw === null ? fallback : raw === 'true'
}

/** Читает настройки; битые и вышедшие за пределы значения заменяются по умолчанию или зажимаются. */
export function loadSettings(
  store: KeyValueStore,
  limits: SettingsLimits,
  defaults: Settings,
): Settings {
  OBSOLETE_KEYS.forEach((key) => store.remove(key))
  return {
    renderDistance: Math.round(
      readNumber(
        store,
        KEYS.renderDistance,
        limits.renderDistanceMin,
        limits.renderDistanceMax,
        defaults.renderDistance,
      ),
    ),
    ambientOcclusion: readBool(store, KEYS.ambientOcclusion, defaults.ambientOcclusion),
    mouseSensitivity: readNumber(
      store,
      KEYS.mouseSensitivity,
      limits.mouseSensitivityMin,
      limits.mouseSensitivityMax,
      defaults.mouseSensitivity,
    ),
    fov: readNumber(store, KEYS.fov, limits.fovMin, limits.fovMax, defaults.fov),
    showDebug: readBool(store, KEYS.showDebug, defaults.showDebug),
  }
}

export function saveSettings(store: KeyValueStore, s: Settings) {
  store.set(KEYS.renderDistance, String(s.renderDistance))
  store.set(KEYS.ambientOcclusion, String(s.ambientOcclusion))
  store.set(KEYS.mouseSensitivity, String(s.mouseSensitivity))
  store.set(KEYS.fov, String(s.fov))
  store.set(KEYS.showDebug, String(s.showDebug))
}

/** Склонение для «Дальность: N колонок». */
export function columnsWord(n: number): string {
  const mod10 = n % 10
  const mod100 = n % 100
  if (mod10 === 1 && mod100 !== 11) return 'колонка'
  if (mod10 >= 2 && mod10 <= 4 && (mod100 < 12 || mod100 > 14)) return 'колонки'
  return 'колонок'
}
