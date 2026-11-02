import type { KeyValueStore } from './store'

export const KEYS = {
  renderDistance: 'minerust.renderDistance',
  ambientOcclusion: 'minerust.ambientOcclusion',
  clouds: 'minerust.clouds',
  weather: 'minerust.weather',
  autoStep: 'minerust.autoStep',
  mouseSensitivity: 'minerust.mouseSensitivity',
  fov: 'minerust.fov',
  coloredLight: 'minerust.coloredLight',
  waterReflections: 'minerust.waterReflections',
  groundFog: 'minerust.groundFog',
  plantSway: 'minerust.plantSway',
  hdr: 'minerust.hdr',
  bloom: 'minerust.bloom',
  waterDepth: 'minerust.waterDepth',
  sunRays: 'minerust.sunRays',
  shadows: 'minerust.shadows',
  shadowQualityHigh: 'minerust.shadowQualityHigh',
  cloudShadows: 'minerust.cloudShadows',
  ssr: 'minerust.ssr',
  autoExposure: 'minerust.autoExposure',
  showDebug: 'minerust.showDebug',
} as const
// Ключи убранных настроек (панель управления, выбор графического API): чистим, чтобы не копился мусор.
const OBSOLETE_KEYS = ['minerust.controlsHidden', 'minerust.graphicsApi', 'minerust.fxaa']

export interface EngineSettings {
  renderDistance: number
  ambientOcclusion: boolean
  clouds: boolean
  weather: boolean
  autoStep: boolean
  mouseSensitivity: number
  fov: number
  coloredLight: boolean
  waterReflections: boolean
  groundFog: boolean
  plantSway: boolean
  hdr: boolean
  bloom: boolean
  waterDepth: boolean
  sunRays: boolean
  shadows: boolean
  shadowQualityHigh: boolean
  cloudShadows: boolean
  ssr: boolean
  autoExposure: boolean
}

export interface Settings extends EngineSettings {
  showDebug: boolean
}

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
    clouds: readBool(store, KEYS.clouds, defaults.clouds),
    weather: readBool(store, KEYS.weather, defaults.weather),
    autoStep: readBool(store, KEYS.autoStep, defaults.autoStep),
    mouseSensitivity: readNumber(
      store,
      KEYS.mouseSensitivity,
      limits.mouseSensitivityMin,
      limits.mouseSensitivityMax,
      defaults.mouseSensitivity,
    ),
    fov: readNumber(store, KEYS.fov, limits.fovMin, limits.fovMax, defaults.fov),
    coloredLight: readBool(store, KEYS.coloredLight, defaults.coloredLight),
    waterReflections: readBool(store, KEYS.waterReflections, defaults.waterReflections),
    groundFog: readBool(store, KEYS.groundFog, defaults.groundFog),
    plantSway: readBool(store, KEYS.plantSway, defaults.plantSway),
    hdr: readBool(store, KEYS.hdr, defaults.hdr),
    bloom: readBool(store, KEYS.bloom, defaults.bloom),
    waterDepth: readBool(store, KEYS.waterDepth, defaults.waterDepth),
    sunRays: readBool(store, KEYS.sunRays, defaults.sunRays),
    shadows: readBool(store, KEYS.shadows, defaults.shadows),
    shadowQualityHigh: readBool(store, KEYS.shadowQualityHigh, defaults.shadowQualityHigh),
    cloudShadows: readBool(store, KEYS.cloudShadows, defaults.cloudShadows),
    ssr: readBool(store, KEYS.ssr, defaults.ssr),
    autoExposure: readBool(store, KEYS.autoExposure, defaults.autoExposure),
    showDebug: readBool(store, KEYS.showDebug, defaults.showDebug),
  }
}

export function saveSettings(store: KeyValueStore, s: Settings) {
  store.set(KEYS.renderDistance, String(s.renderDistance))
  store.set(KEYS.ambientOcclusion, String(s.ambientOcclusion))
  store.set(KEYS.clouds, String(s.clouds))
  store.set(KEYS.weather, String(s.weather))
  store.set(KEYS.autoStep, String(s.autoStep))
  store.set(KEYS.mouseSensitivity, String(s.mouseSensitivity))
  store.set(KEYS.fov, String(s.fov))
  store.set(KEYS.coloredLight, String(s.coloredLight))
  store.set(KEYS.waterReflections, String(s.waterReflections))
  store.set(KEYS.groundFog, String(s.groundFog))
  store.set(KEYS.plantSway, String(s.plantSway))
  store.set(KEYS.hdr, String(s.hdr))
  store.set(KEYS.bloom, String(s.bloom))
  store.set(KEYS.waterDepth, String(s.waterDepth))
  store.set(KEYS.sunRays, String(s.sunRays))
  store.set(KEYS.shadows, String(s.shadows))
  store.set(KEYS.shadowQualityHigh, String(s.shadowQualityHigh))
  store.set(KEYS.cloudShadows, String(s.cloudShadows))
  store.set(KEYS.ssr, String(s.ssr))
  store.set(KEYS.autoExposure, String(s.autoExposure))
  store.set(KEYS.showDebug, String(s.showDebug))
}

export function columnsWord(n: number): string {
  const mod10 = n % 10
  const mod100 = n % 100
  if (mod10 === 1 && mod100 !== 11) return 'колонка'
  if (mod10 >= 2 && mod10 <= 4 && (mod100 < 12 || mod100 > 14)) return 'колонки'
  return 'колонок'
}
