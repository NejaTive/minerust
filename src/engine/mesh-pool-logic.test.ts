import { describe, expect, it } from 'vitest'
import { pickWorker, slotsFor, workerCount } from './mesh-pool-logic'

describe('workerCount', () => {
  it('оставляет ядро основному потоку', () => {
    expect(workerCount(4)).toBe(3)
    expect(workerCount(8)).toBe(4)
  })
  it('не больше четырёх и не меньше одного', () => {
    expect(workerCount(64)).toBe(4)
    expect(workerCount(1)).toBe(1)
    expect(workerCount(2)).toBe(1)
  })
  it('без данных о ядрах берёт один воркер', () => {
    expect(workerCount(undefined)).toBe(1)
    expect(workerCount(0)).toBe(1)
  })
})

describe('pickWorker', () => {
  it('берёт наименее загруженного', () => {
    expect(pickWorker([2, 1, 2])).toBe(1)
    expect(pickWorker([1, 0, 1])).toBe(1)
  })
  it('при равной загрузке — первого', () => {
    expect(pickWorker([0, 0])).toBe(0)
  })
  it('-1, когда все заняты', () => {
    expect(pickWorker([2, 2])).toBe(-1)
    expect(pickWorker([])).toBe(-1)
  })
  it('уважает лимит на воркер', () => {
    expect(pickWorker([1], 1)).toBe(-1)
  })
})

describe('slotsFor', () => {
  it('по два задания на готовый воркер', () => {
    expect(slotsFor(3)).toBe(6)
    expect(slotsFor(0)).toBe(0)
  })
})
