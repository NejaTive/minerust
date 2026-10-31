import { describe, expect, it } from 'vitest'
import { GAME_MODES, WORLD_TYPES, cycle } from './kinds'

describe('cycle', () => {
  it('единственное значение остаётся собой', () => {
    expect(cycle(GAME_MODES, 'creative')).toBe('creative')
    expect(cycle(WORLD_TYPES, 'standard')).toBe('standard')
  })
  it('несколько значений идут по кругу', () => {
    expect(cycle(['a', 'b', 'c'], 'a')).toBe('b')
    expect(cycle(['a', 'b', 'c'], 'c')).toBe('a')
  })
})
