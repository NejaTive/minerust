import { describe, expect, it } from 'vitest'
import { DEFAULT_WORLD_NAME, MAX_WORLD_NAME, uniqueName } from './names'

describe('uniqueName', () => {
  it('свободное имя остаётся', () => expect(uniqueName('Дом', ['Лес'])).toBe('Дом'))
  it('пустое — имя по умолчанию', () => expect(uniqueName('  ', [])).toBe(DEFAULT_WORLD_NAME))
  it('занятое получает номер', () => {
    expect(uniqueName(DEFAULT_WORLD_NAME, [DEFAULT_WORLD_NAME])).toBe('Новый мир (2)')
  })
  it('номер подбирается первый свободный', () => {
    const taken = ['Новый мир', 'Новый мир (2)', 'Новый мир (4)']
    expect(uniqueName('Новый мир', taken)).toBe('Новый мир (3)')
  })
  it('длинное имя обрезается', () => {
    expect(uniqueName('я'.repeat(100), [])).toHaveLength(MAX_WORLD_NAME)
  })
})
