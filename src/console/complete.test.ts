import { describe, expect, it } from 'vitest'
import { TabCycler, type Completion } from './complete'

const complete = (line: string): Completion =>
  line.startsWith('/time n')
    ? { start: 6, options: ['noon', 'night'] }
    : { start: 1, options: line === '/' ? ['time', 'weather'] : [] }

describe('TabCycler', () => {
  it('подставляет первый вариант, повторный Tab — следующий по кругу', () => {
    const tab = new TabCycler()
    expect(tab.next('/time n', complete)).toBe('/time noon')
    expect(tab.next('/time noon', complete)).toBe('/time night')
    expect(tab.next('/time night', complete)).toBe('/time noon')
  })

  it('после ручной правки начинает заново', () => {
    const tab = new TabCycler()
    expect(tab.next('/', complete)).toBe('/time')
    expect(tab.next('/time', complete)).toBe('/weather')
    expect(tab.next('/', complete)).toBe('/time')
  })

  it('без вариантов ничего не меняет', () => {
    const tab = new TabCycler()
    expect(tab.next('/zzz', complete)).toBeUndefined()
  })

  it('reset начинает цикл заново', () => {
    const tab = new TabCycler()
    tab.next('/time n', complete)
    tab.reset()
    expect(tab.next('/time noon', complete)).toBe('/time noon')
  })
})
