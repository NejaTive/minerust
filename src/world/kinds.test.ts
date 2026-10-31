import { describe, expect, it } from 'vitest'
import { GAME_MODES, WORLD_TYPES, WORLD_TYPE_LABELS, cycle } from './kinds'

describe('cycle', () => {
  it('единственное значение остаётся собой', () => {
    expect(cycle(GAME_MODES, 'creative')).toBe('creative')
  })
  it('тип мира переключается Стандартный ⇄ Плоский', () => {
    expect(cycle(WORLD_TYPES, 'standard')).toBe('flat')
    expect(cycle(WORLD_TYPES, 'flat')).toBe('standard')
    expect(WORLD_TYPE_LABELS).toEqual({ standard: 'Стандартный', flat: 'Плоский' })
  })
  it('несколько значений идут по кругу', () => {
    expect(cycle(['a', 'b', 'c'], 'a')).toBe('b')
    expect(cycle(['a', 'b', 'c'], 'c')).toBe('a')
  })
})
