import { describe, expect, it } from 'vitest'
// Копия `blocks()` движка; её сверяет Rust-тест `js_fixture_matches_blocks`.
import blocks from './blocks.fixture.json'
import { HOTBAR, loadHotbarSlot, saveHotbarSlot } from './hotbar'
import { memoryStore } from './store'

describe('HOTBAR', () => {
  const placeable = new Set(blocks.filter((b) => b.placeable).map((b) => b.name))

  it('содержит только блоки, которые можно поставить', () => {
    for (const name of HOTBAR) {
      if (name !== null) expect(placeable, `блок «${name}»`).toContain(name)
    }
  })

  it('не повторяет блоки', () => {
    const names = HOTBAR.filter((name) => name !== null)
    expect(new Set(names).size).toBe(names.length)
  })

  it('помещается на клавиши 1–9', () => {
    expect(HOTBAR.length).toBeLessThanOrEqual(9)
  })
})

describe('слот хотбара', () => {
  it('читает сохранённый слот', () => {
    const store = memoryStore()
    saveHotbarSlot(store, 3)
    expect(loadHotbarSlot(store)).toBe(3)
  })

  it.each(['-1', '99', '1.5', 'abc', ''])('значение «%s» даёт первый слот', (raw) => {
    expect(loadHotbarSlot(memoryStore({ 'minerust.hotbarSlot': raw }))).toBe(0)
  })

  it('без записи — первый слот', () => {
    expect(loadHotbarSlot(memoryStore())).toBe(0)
  })
})
