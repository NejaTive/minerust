import { describe, expect, it } from 'vitest'
import { GAME_MODES, WORLD_TYPES, WORLD_TYPE_LABELS, cycle, selectableWorldTypes } from './kinds'

describe('cycle', () => {
  it('единственное значение остаётся собой', () => {
    expect(cycle(GAME_MODES, 'creative')).toBe('creative')
  })
  it('тип мира переключается Стандартный → Плоский → Отладка', () => {
    expect(cycle(WORLD_TYPES, 'standard')).toBe('flat')
    expect(cycle(WORLD_TYPES, 'flat')).toBe('debug')
    expect(cycle(WORLD_TYPES, 'debug')).toBe('standard')
    expect(WORLD_TYPE_LABELS).toEqual({
      standard: 'Стандартный',
      flat: 'Плоский',
      debug: 'Отладка',
    })
  })
  it('без режима отладки тип debug недоступен для выбора', () => {
    expect(selectableWorldTypes(false)).toEqual(['standard', 'flat'])
    expect(selectableWorldTypes(true)).toEqual(['standard', 'flat', 'debug'])
    expect(cycle(selectableWorldTypes(false), 'flat')).toBe('standard')
  })
  it('несколько значений идут по кругу', () => {
    expect(cycle(['a', 'b', 'c'], 'a')).toBe('b')
    expect(cycle(['a', 'b', 'c'], 'c')).toBe('a')
  })
})
