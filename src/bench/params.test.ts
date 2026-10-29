import { describe, expect, it } from 'vitest'
import { parseBenchParams } from './params'
import { percentile } from './recorder'

describe('parseBenchParams', () => {
  it('обычный адрес — не замер', () => {
    expect(parseBenchParams('')).toEqual({ enabled: false, debug: true, workers: true })
    expect(parseBenchParams('?nogpu')).toEqual({ enabled: false, debug: true, workers: true })
  })

  it('?bench включает замер с отладкой', () => {
    expect(parseBenchParams('?bench')).toEqual({ enabled: true, debug: true, workers: true })
    expect(parseBenchParams('?bench=1&debug=1')).toEqual({
      enabled: true,
      debug: true,
      workers: true,
    })
  })

  it('workers=0 выключает воркеры мешера', () => {
    expect(parseBenchParams('?bench&workers=0').workers).toBe(false)
    expect(parseBenchParams('?bench&workers=2').workers).toBe(true)
  })

  it('debug=0 выключает отладку, прочие значения — нет', () => {
    expect(parseBenchParams('?bench&debug=0')).toEqual({
      enabled: true,
      debug: false,
      workers: true,
    })
    expect(parseBenchParams('?bench&debug=no').debug).toBe(true)
  })
})

describe('percentile', () => {
  it('считает по отсортированной копии', () => {
    const values = [5, 1, 4, 2, 3]
    expect(percentile(values, 50)).toBe(3)
    expect(percentile(values, 95)).toBe(5)
    expect(values).toEqual([5, 1, 4, 2, 3])
  })

  it('пустой список — ноль', () => expect(percentile([], 50)).toBe(0))
})
