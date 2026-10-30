import { describe, expect, it } from 'vitest'
import {
  KEYS,
  columnsWord,
  defaultSettings,
  loadSettings,
  saveSettings,
  type SettingsLimits,
} from './settings'
import { memoryStore } from './store'

const limits: SettingsLimits = {
  renderDistanceMin: 4,
  renderDistanceMax: 16,
  mouseSensitivityMin: 0.25,
  mouseSensitivityMax: 3,
  fovMin: 60,
  fovMax: 110,
}
const defaults = defaultSettings({
  renderDistance: 8,
  ambientOcclusion: true,
  clouds: true,
  mouseSensitivity: 1,
  fov: 70,
})

const load = (data: Record<string, string> = {}) =>
  loadSettings(memoryStore(data), limits, defaults)

describe('loadSettings', () => {
  it('пустое хранилище даёт значения по умолчанию, отладка выключена', () => {
    expect(load()).toEqual({ ...defaults, showDebug: false })
  })

  it('читает сохранённые значения', () => {
    const data = {
      [KEYS.renderDistance]: '12',
      [KEYS.ambientOcclusion]: 'false',
      [KEYS.clouds]: 'false',
      [KEYS.mouseSensitivity]: '1.5',
      [KEYS.fov]: '90',
      [KEYS.showDebug]: 'true',
    }
    expect(load(data)).toEqual({
      renderDistance: 12,
      ambientOcclusion: false,
      clouds: false,
      mouseSensitivity: 1.5,
      fov: 90,
      showDebug: true,
    })
  })

  it('зажимает значения по пределам', () => {
    const s = load({
      [KEYS.renderDistance]: '99',
      [KEYS.mouseSensitivity]: '0',
      [KEYS.fov]: '500',
    })
    expect(s.renderDistance).toBe(16)
    expect(s.mouseSensitivity).toBe(0.25)
    expect(s.fov).toBe(110)
    expect(load({ [KEYS.renderDistance]: '-3' }).renderDistance).toBe(4)
  })

  it('округляет дальность', () => {
    expect(load({ [KEYS.renderDistance]: '9.6' }).renderDistance).toBe(10)
  })

  it('битые числа заменяются значением по умолчанию', () => {
    const s = load({
      [KEYS.renderDistance]: 'abc',
      [KEYS.mouseSensitivity]: 'NaN',
      [KEYS.fov]: 'Infinity',
    })
    expect(s.renderDistance).toBe(8)
    expect(s.mouseSensitivity).toBe(1)
    expect(s.fov).toBe(70)
  })

  it('логические читаются строго по "true"', () => {
    expect(load({ [KEYS.ambientOcclusion]: '1' }).ambientOcclusion).toBe(false)
    expect(load({ [KEYS.showDebug]: 'yes' }).showDebug).toBe(false)
    expect(load({ [KEYS.clouds]: '1' }).clouds).toBe(false)
  })

  it('чистит ключи убранных настроек', () => {
    const store = memoryStore({ 'minerust.controlsHidden': '1', 'minerust.graphicsApi': 'webgl' })
    loadSettings(store, limits, defaults)
    expect(store.get('minerust.controlsHidden')).toBeNull()
    expect(store.get('minerust.graphicsApi')).toBeNull()
  })

  it('сохранённое читается обратно', () => {
    const store = memoryStore()
    const saved = { ...defaults, renderDistance: 6, fov: 100, showDebug: true }
    saveSettings(store, saved)
    expect(loadSettings(store, limits, defaults)).toEqual(saved)
  })
})

describe('columnsWord', () => {
  it.each([
    [1, 'колонка'],
    [2, 'колонки'],
    [4, 'колонки'],
    [5, 'колонок'],
    [11, 'колонок'],
    [12, 'колонок'],
    [16, 'колонок'],
    [21, 'колонка'],
  ])('%i → %s', (n, word) => expect(columnsWord(n)).toBe(word))
})
