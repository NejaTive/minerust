export type GraphicsApi = 'webgpu' | 'webgl2'

export const API_LABELS: Record<GraphicsApi, string> = {
  webgpu: 'WebGPU',
  webgl2: 'WebGL2',
}

/** Блоки, которые игрок ставит; имена — как в `Engine.select_block`. */
export type BlockName =
  | 'grass'
  | 'dirt'
  | 'stone'
  | 'sand'
  | 'log'
  | 'leaves'
  | 'glowstone'
  | 'tall_grass'
  | 'poppy'
  | 'dandelion'
  | 'torch'

export const BLOCK_LABELS: Record<BlockName, string> = {
  grass: 'трава',
  dirt: 'земля',
  stone: 'камень',
  sand: 'песок',
  log: 'бревно',
  leaves: 'листва',
  glowstone: 'светокамень',
  tall_grass: 'высокая трава',
  poppy: 'мак',
  dandelion: 'одуванчик',
  torch: 'факел',
}

/**
 * Содержимое хотбара по слотам; `null` — пустой слот. Будущий инвентарь заменит константу
 * на состояние, остальной код читает только этот массив.
 */
export const HOTBAR: (BlockName | null)[] = [
  'grass',
  'dirt',
  'stone',
  'sand',
  'log',
  'leaves',
  'glowstone',
  'torch',
  'poppy',
]

const GRAPHICS_API_KEY = 'minerust.graphicsApi'
// Ключ дальности остался прежним, чтобы обновление не сбросило её у игрока.
const RENDER_DISTANCE_KEY = 'minerust.renderDistance'
const AMBIENT_OCCLUSION_KEY = 'minerust.ambientOcclusion'
const MOUSE_SENSITIVITY_KEY = 'minerust.mouseSensitivity'
const FOV_KEY = 'minerust.fov'
const SHOW_DEBUG_KEY = 'minerust.showDebug'
const HOTBAR_SLOT_KEY = 'minerust.hotbarSlot'
// Ключ убранной панели управления: чистим, чтобы не копился мусор.
const OBSOLETE_KEYS = ['minerust.controlsHidden']

/** Дальность прорисовки в колонках; пределы — как в `engine/src/loader.rs`. */
export const MIN_RENDER_DISTANCE = 4
export const MAX_RENDER_DISTANCE = 16
/** Множитель чувствительности мыши; пределы — как в `engine/src/camera.rs`. */
export const MIN_MOUSE_SENSITIVITY = 0.25
export const MAX_MOUSE_SENSITIVITY = 3
/** Вертикальный угол обзора, градусы; пределы — как в `engine/src/camera.rs`. */
export const MIN_FOV = 60
export const MAX_FOV = 110

/** Настройки игры. Графический API хранится отдельно: его меняет пересоздание движка. */
export interface Settings {
  renderDistance: number
  ambientOcclusion: boolean
  mouseSensitivity: number
  fov: number
  showDebug: boolean
}

export const DEFAULT_SETTINGS: Settings = {
  renderDistance: 8,
  ambientOcclusion: true,
  mouseSensitivity: 1,
  fov: 70,
  showDebug: false,
}

// localStorage может бросать (приватный режим, запрет site data) — тогда работаем
// со значениями по умолчанию и ничего не запоминаем.
function read(key: string): string | null {
  try {
    return localStorage.getItem(key)
  } catch {
    return null
  }
}

function write(key: string, value: string) {
  try {
    localStorage.setItem(key, value)
  } catch {
    // Настройка просто не переживёт перезагрузку.
  }
}

function remove(key: string) {
  try {
    localStorage.removeItem(key)
  } catch {
    // Нечего чистить.
  }
}

function readNumber(key: string, min: number, max: number, fallback: number): number {
  const raw = read(key)
  const value = raw === null ? Number.NaN : Number(raw)
  if (!Number.isFinite(value)) return fallback
  return Math.min(max, Math.max(min, value))
}

function readBool(key: string, fallback: boolean): boolean {
  const raw = read(key)
  return raw === null ? fallback : raw === 'true'
}

export function loadGraphicsApi(): GraphicsApi {
  return read(GRAPHICS_API_KEY) === 'webgl2' ? 'webgl2' : 'webgpu'
}

export function saveGraphicsApi(api: GraphicsApi) {
  write(GRAPHICS_API_KEY, api)
}

export function loadSettings(): Settings {
  OBSOLETE_KEYS.forEach(remove)
  const d = DEFAULT_SETTINGS
  return {
    renderDistance: Math.round(
      readNumber(RENDER_DISTANCE_KEY, MIN_RENDER_DISTANCE, MAX_RENDER_DISTANCE, d.renderDistance),
    ),
    ambientOcclusion: readBool(AMBIENT_OCCLUSION_KEY, d.ambientOcclusion),
    mouseSensitivity: readNumber(
      MOUSE_SENSITIVITY_KEY,
      MIN_MOUSE_SENSITIVITY,
      MAX_MOUSE_SENSITIVITY,
      d.mouseSensitivity,
    ),
    fov: readNumber(FOV_KEY, MIN_FOV, MAX_FOV, d.fov),
    showDebug: readBool(SHOW_DEBUG_KEY, d.showDebug),
  }
}

export function saveSettings(s: Settings) {
  write(RENDER_DISTANCE_KEY, String(s.renderDistance))
  write(AMBIENT_OCCLUSION_KEY, String(s.ambientOcclusion))
  write(MOUSE_SENSITIVITY_KEY, String(s.mouseSensitivity))
  write(FOV_KEY, String(s.fov))
  write(SHOW_DEBUG_KEY, String(s.showDebug))
}

/** Выбранный слот хотбара; вне 0..HOTBAR.length−1 или не число — первый. */
export function loadHotbarSlot(): number {
  const raw = read(HOTBAR_SLOT_KEY)
  const value = raw === null ? Number.NaN : Number(raw)
  return Number.isInteger(value) && value >= 0 && value < HOTBAR.length ? value : 0
}

export function saveHotbarSlot(slot: number) {
  write(HOTBAR_SLOT_KEY, String(slot))
}
