export type GraphicsApi = 'webgpu' | 'webgl2'

export const API_LABELS: Record<GraphicsApi, string> = {
  webgpu: 'WebGPU',
  webgl2: 'WebGL2',
}

const GRAPHICS_API_KEY = 'minerust.graphicsApi'
const CONTROLS_HIDDEN_KEY = 'minerust.controlsHidden'

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
