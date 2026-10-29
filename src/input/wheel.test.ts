import { describe, expect, it } from 'vitest'
import { WheelAccumulator, stepSlot } from './wheel'

const PIXELS = 0
const LINES = 1
const PAGES = 2

describe('WheelAccumulator', () => {
  it('щелчок колеса в пикселях — один шаг', () => {
    const w = new WheelAccumulator()
    expect(w.push(120, PIXELS)).toBe(1)
    expect(w.push(-120, PIXELS)).toBe(-1)
  })

  it('мелкие события тачпада копятся до порога', () => {
    const w = new WheelAccumulator()
    const steps = Array.from({ length: 5 }, () => w.push(30, PIXELS))
    expect(steps).toEqual([0, 0, 0, 1, 0])
  })

  it('строки и страницы переводятся в пиксели', () => {
    expect(new WheelAccumulator().push(1, LINES)).toBe(0)
    expect(new WheelAccumulator().push(3, LINES)).toBe(1)
    expect(new WheelAccumulator().push(1, PAGES)).toBe(1)
  })

  it('большая прокрутка за одно событие — не больше одного слота', () => {
    expect(new WheelAccumulator().push(5000, PIXELS)).toBe(1)
    expect(new WheelAccumulator().push(-5000, PIXELS)).toBe(-1)
  })

  it('смена направления сбрасывает остаток', () => {
    const w = new WheelAccumulator()
    expect(w.push(90, PIXELS)).toBe(0)
    expect(w.push(-90, PIXELS)).toBe(0)
    expect(w.push(-90, PIXELS)).toBe(-1)
  })

  it('нулевая прокрутка ничего не даёт', () => {
    expect(new WheelAccumulator().push(0, PIXELS)).toBe(0)
  })
})

describe('stepSlot', () => {
  it('идёт по кругу', () => {
    expect(stepSlot(8, 1, 9)).toBe(0)
    expect(stepSlot(0, -1, 9)).toBe(8)
    expect(stepSlot(3, 1, 9)).toBe(4)
  })
})
