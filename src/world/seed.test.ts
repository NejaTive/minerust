import { describe, expect, it } from 'vitest'
import { seedFromInput } from './seed'

const random = () => 12345

describe('seedFromInput', () => {
  it('пусто или пробелы — случайный', () => {
    expect(seedFromInput('', random)).toBe(12345)
    expect(seedFromInput('   ', random)).toBe(12345)
  })

  it('целое число — как есть', () => {
    expect(seedFromInput('42', random)).toBe(42)
    expect(seedFromInput(' -7 ', random)).toBe(-7)
    expect(seedFromInput('2147483647', random)).toBe(2147483647)
    expect(seedFromInput('-2147483648', random)).toBe(-2147483648)
  })

  it('текст — хэш строки как String.hashCode в Java', () => {
    expect(seedFromInput('a', random)).toBe(97)
    expect(seedFromInput('hello', random)).toBe(99162322)
    expect(seedFromInput('Minecraft', random)).toBe(-1595926131)
  })

  it('хэш детерминирован и укладывается в i32', () => {
    const text = 'Какой-то очень длинный сид для проверки переполнения'
    const hash = seedFromInput(text, random)
    expect(seedFromInput(text, random)).toBe(hash)
    expect(Number.isInteger(hash) && hash === (hash | 0)).toBe(true)
  })

  it('число вне i32 и дробное — тоже текст', () => {
    expect(seedFromInput('2147483648', random)).toBe(seedFromInput('2147483648', () => 1))
    expect(seedFromInput('2147483648', random)).not.toBe(2147483648)
    expect(seedFromInput('1.5', random)).toBe(seedFromInput('1.5', () => 1))
  })
})
