import { describe, expect, it } from 'vitest'
// Копия `blocks()` движка; её сверяет Rust-тест `js_fixture_matches_blocks`.
import blocks from './blocks.fixture.json'
import {
  DEFAULT_HOTBAR,
  HOTBAR_SIZE,
  clearHotbar,
  clearHotbarSlot,
  loadHotbarContents,
  loadHotbarSlot,
  putHotbarSlot,
  saveHotbarContents,
  saveHotbarSlot,
  swapHotbarSlots,
} from './hotbar'
import { memoryStore } from './store'

const known = new Set(blocks.map((b) => b.name))
const placeable = new Set(blocks.filter((b) => b.placeable).map((b) => b.name))

/** Непустое содержимое только для тестов операций; умолчание игры — пустые слоты. */
const SAMPLE: (string | null)[] = [
  'grass',
  'dirt',
  'stone',
  'sand',
  'log',
  'leaves',
  'glowstone',
  'torch',
  'poppy',
]

describe('DEFAULT_HOTBAR', () => {
  it('всегда пустой', () => {
    expect(DEFAULT_HOTBAR).toEqual(Array.from({ length: HOTBAR_SIZE }, () => null))
  })

  it('содержит только блоки, которые можно поставить', () => {
    for (const name of DEFAULT_HOTBAR) {
      if (name !== null) expect(placeable, `блок «${name}»`).toContain(name)
    }
  })

  it('не повторяет блоки', () => {
    const names = DEFAULT_HOTBAR.filter((name) => name !== null)
    expect(new Set(names).size).toBe(names.length)
  })

  it('помещается на клавиши 1–9', () => {
    expect(DEFAULT_HOTBAR.length).toBe(HOTBAR_SIZE)
    expect(DEFAULT_HOTBAR.length).toBeLessThanOrEqual(9)
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

describe('содержимое хотбара', () => {
  it('без записи — пустое умолчание', () => {
    expect(loadHotbarContents(memoryStore(), known)).toEqual([...DEFAULT_HOTBAR])
  })

  it('читает сохранённые слоты', () => {
    const store = memoryStore()
    const slots = [...SAMPLE]
    slots[0] = 'dandelion'
    slots[1] = null
    saveHotbarContents(store, slots)
    expect(loadHotbarContents(store, known)).toEqual(slots)
  })

  it('сохраняет пустой хотбар и переживает перечитывание', () => {
    const store = memoryStore()
    const slots = Array.from({ length: HOTBAR_SIZE }, () => null)
    saveHotbarContents(store, slots)
    expect(loadHotbarContents(store, known)).toEqual(slots)
  })

  it.each([
    ['не JSON', '{'],
    ['не массив', '{"a":1}'],
    ['короткий массив', '["grass"]'],
    ['длинный массив', JSON.stringify([...SAMPLE, 'dirt'])],
    ['неизвестное имя', JSON.stringify(['no_such_block', ...SAMPLE.slice(1)])],
    ['не строка и не null', JSON.stringify([1, ...SAMPLE.slice(1)])],
  ])('мусор «%s» → умолчание', (_label, raw) => {
    expect(loadHotbarContents(memoryStore({ 'minerust.hotbar': raw }), known)).toEqual([
      ...DEFAULT_HOTBAR,
    ])
  })

  it('имя из blocks() (в т.ч. неразмещаемое) принимается', () => {
    const store = memoryStore()
    const slots = [...SAMPLE]
    slots[0] = 'air'
    saveHotbarContents(store, slots)
    expect(loadHotbarContents(store, known)[0]).toBe('air')
  })
})

describe('операции со слотами', () => {
  it('put кладёт блок в слот', () => {
    const slots = [...SAMPLE]
    putHotbarSlot(slots, 2, 'dandelion')
    expect(slots[2]).toBe('dandelion')
  })

  it('put с null очищает слот', () => {
    const slots = [...SAMPLE]
    putHotbarSlot(slots, 0, null)
    expect(slots[0]).toBeNull()
  })

  it('put вне диапазона — no-op', () => {
    const slots = [...SAMPLE]
    const before = [...slots]
    putHotbarSlot(slots, -1, 'dirt')
    putHotbarSlot(slots, HOTBAR_SIZE, 'dirt')
    expect(slots).toEqual(before)
  })

  it('swap меняет два слота', () => {
    const slots = [...SAMPLE]
    const a = slots[0]
    const b = slots[3]
    swapHotbarSlots(slots, 0, 3)
    expect(slots[0]).toBe(b)
    expect(slots[3]).toBe(a)
  })

  it('swap одного слота с собой — no-op', () => {
    const slots = [...SAMPLE]
    const before = [...slots]
    swapHotbarSlots(slots, 2, 2)
    expect(slots).toEqual(before)
  })

  it('clear очищает слот', () => {
    const slots = [...SAMPLE]
    clearHotbarSlot(slots, 4)
    expect(slots[4]).toBeNull()
  })

  it('clearHotbar очищает все слоты', () => {
    const slots = [...SAMPLE]
    clearHotbar(slots)
    expect(slots).toEqual(Array.from({ length: HOTBAR_SIZE }, () => null))
  })
})
