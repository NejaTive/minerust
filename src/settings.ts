export type GraphicsApi = 'webgpu' | 'webgl2'

export const API_LABELS: Record<GraphicsApi, string> = {
  webgpu: 'WebGPU',
  webgl2: 'WebGL2',
}

/** Блоки, которые игрок ставит; имена — как в `Engine.select_block`. */
export type BlockName = 'grass' | 'dirt' | 'stone'

export const BLOCK_LABELS: Record<BlockName, string> = {
  grass: 'трава',
  dirt: 'земля',
  stone: 'камень',
}

const GRAPHICS_API_KEY = 'minerust.graphicsApi'
const CONTROLS_HIDDEN_KEY = 'minerust.controlsHidden'
const RENDER_DISTANCE_KEY = 'minerust.renderDistance'

/** Дальность прорисовки в колонках; пределы — как в `engine/src/loader.rs`. */
export const MIN_RENDER_DISTANCE = 4
export const MAX_RENDER_DISTANCE = 16
const DEFAULT_RENDER_DISTANCE = 8

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

export function loadGraphicsApi(): GraphicsApi {
  return read(GRAPHICS_API_KEY) === 'webgl2' ? 'webgl2' : 'webgpu'
}

export function saveGraphicsApi(api: GraphicsApi) {
  write(GRAPHICS_API_KEY, api)
}

export function loadControlsHidden(): boolean {
  return read(CONTROLS_HIDDEN_KEY) === 'true'
}

export function saveControlsHidden(hidden: boolean) {
  write(CONTROLS_HIDDEN_KEY, String(hidden))
}

export function loadRenderDistance(): number {
  const value = Number.parseInt(read(RENDER_DISTANCE_KEY) ?? '', 10)
  if (Number.isNaN(value)) return DEFAULT_RENDER_DISTANCE
  return Math.min(MAX_RENDER_DISTANCE, Math.max(MIN_RENDER_DISTANCE, value))
}

export function saveRenderDistance(columns: number) {
  write(RENDER_DISTANCE_KEY, String(columns))
}
