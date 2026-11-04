import { describe, expect, it } from 'vitest'
import {
  changedSlots,
  clearSlot,
  clickSlot,
  containerLayout,
  returnCursor,
  shiftClick,
  type ContainerState,
} from './container'

function state(chest: (string | null)[], hotbar: (string | null)[] = []): ContainerState {
  return { chest, hotbar: [...hotbar, ...Array(9 - hotbar.length).fill(null)], cursor: null }
}

describe('окно сундука', () => {
  it('одинарный сундук — три ряда, двойной — шесть', () => {
    expect(containerLayout(27)).toMatchObject({ rows: 3, title: 'Сундук', height: 167 })
    expect(containerLayout(54)).toMatchObject({ rows: 6, title: 'Большой сундук', height: 221 })
  })

  it('нижняя часть окна сдвигается на высоту рядов', () => {
    const single = containerLayout(27)
    const double = containerLayout(54)
    expect(double.hotbarY - single.hotbarY).toBe(3 * 18)
    expect(single.hotbarY).toBe(3 * 18 + 89)
  })
})

describe('перенос предметов', () => {
  it('клик берёт предмет в руку, второй клик кладёт в пустой слот', () => {
    const s = state(['stone', null, null])
    clickSlot(s, 'chest', 0)
    expect(s.cursor).toBe('stone')
    expect(s.chest).toEqual([null, null, null])
    clickSlot(s, 'chest', 2)
    expect(s.cursor).toBeNull()
    expect(s.chest).toEqual([null, null, 'stone'])
  })

  it('клик по занятому слоту с предметом в руке меняет их местами', () => {
    const s = state(['stone', 'dirt'])
    clickSlot(s, 'chest', 0)
    clickSlot(s, 'chest', 1)
    expect(s.cursor).toBe('dirt')
    expect(s.chest).toEqual([null, 'stone'])
  })

  it('предмет переходит из сундука в хотбар и обратно', () => {
    const s = state(['stone'], ['dirt'])
    clickSlot(s, 'chest', 0)
    clickSlot(s, 'hotbar', 0)
    expect(s.cursor).toBe('dirt')
    expect(s.hotbar[0]).toBe('stone')
    clickSlot(s, 'chest', 0)
    expect(s.chest[0]).toBe('dirt')
    expect(s.cursor).toBeNull()
  })

  it('клик по пустому слоту с пустой рукой ничего не меняет', () => {
    const s = state([null])
    clickSlot(s, 'chest', 0)
    expect(s).toEqual(state([null]))
  })

  it('Shift-клик переносит предмет в первый свободный слот другой стороны', () => {
    const s = state(['stone', 'dirt', null], ['sand', null])
    expect(shiftClick(s, 'chest', 1)).toBe(true)
    expect(s.chest).toEqual(['stone', null, null])
    expect(s.hotbar.slice(0, 3)).toEqual(['sand', 'dirt', null])
    expect(shiftClick(s, 'hotbar', 0)).toBe(true)
    expect(s.chest).toEqual(['stone', 'sand', null])
    expect(s.hotbar.slice(0, 2)).toEqual([null, 'dirt'])
  })

  it('Shift-клик не теряет предмет, если другая сторона полна или слот пуст', () => {
    const s = state(['stone', null], Array(9).fill('dirt'))
    expect(shiftClick(s, 'chest', 0)).toBe(false)
    expect(s.chest[0]).toBe('stone')
    expect(shiftClick(s, 'chest', 1)).toBe(false)
  })

  it('очистка слота убирает предмет', () => {
    const s = state(['stone'])
    clearSlot(s, 'chest', 0)
    expect(s.chest).toEqual([null])
  })

  it('различия между снимками — номера изменённых слотов', () => {
    expect(changedSlots(['a', null, 'c'], ['a', 'b', null])).toEqual([1, 2])
    expect(changedSlots([], [null, 'x'])).toEqual([1])
  })
})

describe('возврат предмета из руки при закрытии', () => {
  it('возвращает в слот, откуда взят', () => {
    const s = state(['stone'])
    clickSlot(s, 'chest', 0)
    returnCursor(s)
    expect(s.cursor).toBeNull()
    expect(s.chest[0]).toBe('stone')
  })

  it('если исходный слот занят — в первый свободный слот сундука', () => {
    const s = state(['stone', 'dirt', ...Array(25).fill(null)])
    clickSlot(s, 'chest', 0)
    s.chest[0] = 'sand'
    returnCursor(s)
    expect(s.cursor).toBeNull()
    expect(s.chest[2]).toBe('stone')
  })

  it('если сундук полон — в хотбар', () => {
    const s = state(Array(27).fill('dirt'))
    s.chest[0] = 'stone'
    clickSlot(s, 'chest', 0)
    s.chest[0] = 'sand'
    returnCursor(s)
    expect(s.cursor).toBeNull()
    expect(s.hotbar[0]).toBe('stone')
  })

  it('без места обменивается с исходным слотом и ничего не теряет', () => {
    const s = state(Array(27).fill('dirt'), Array(9).fill('sand'))
    s.chest[0] = 'stone'
    clickSlot(s, 'chest', 0)
    s.chest[0] = 'glass'
    returnCursor(s)
    expect(s.chest[0]).toBe('stone')
    expect(s.cursor).toBe('glass')
  })
})
