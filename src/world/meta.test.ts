import { describe, expect, it } from 'vitest'
import { checkMeta, isReadable } from './meta'
import type { WorldMeta } from './storage'

const meta = (patch: Partial<WorldMeta> = {}): WorldMeta => ({
  version: 3,
  seed: 1337,
  name: 'Новый мир',
  mode: 'creative',
  worldType: 'standard',
  createdAt: 1,
  lastPlayed: 2,
  player: [0, 70, 0, 0, 0, 0],
  ...patch,
})

describe('checkMeta', () => {
  it('версия равна — грузим', () => expect(checkMeta(meta(), 3)).toBe('ok'))
  it('версия старше — грузим (миграция в движке)', () => expect(checkMeta(meta(), 5)).toBe('ok'))
  it('версия новее — не трогаем', () => expect(checkMeta(meta(), 2)).toBe('newer'))
})

describe('isReadable', () => {
  it('принимает целый мир', () => expect(isReadable(meta({ time: 0.3 }))).toBe(true))

  it.each([
    ['версия 0', { version: 0 }],
    ['версия дробная', { version: 1.5 }],
    ['seed дробный', { seed: 1.5 }],
    ['seed не число', { seed: 'x' as unknown as number }],
    ['игрок короче', { player: [0, 0, 0, 0, 0] }],
    ['игрок не массив', { player: null as unknown as number[] }],
    ['игрок с NaN', { player: [0, NaN, 0, 0, 0, 0] }],
    ['нет названия', { name: undefined as unknown as string }],
    ['неизвестный режим', { mode: 'survival' as unknown as 'creative' }],
    ['неизвестный тип мира', { worldType: 'flat' as unknown as 'standard' }],
    ['дата не число', { lastPlayed: 'вчера' as unknown as number }],
    ['игрок с текстом', { player: [0, 'a', 0, 0, 0, 0] as unknown as number[] }],
  ])('отклоняет: %s', (_name, patch) => {
    expect(isReadable(meta(patch))).toBe(false)
    expect(checkMeta(meta(patch), 3)).toBe('broken')
  })

  it('свежий мир без игрока читается', () => {
    expect(isReadable(meta({ player: undefined }))).toBe(true)
  })

  it('битые данные важнее версии', () => {
    expect(checkMeta(meta({ version: 99, seed: NaN }), 3)).toBe('broken')
  })
})
