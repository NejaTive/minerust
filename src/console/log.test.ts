import { describe, expect, it } from 'vitest'
import { appendLog, LOG_KEEP, LOG_TTL_MS, LOG_VISIBLE, LOG_VISIBLE_OPEN, visibleLog } from './log'

describe('log', () => {
  it('подсказка команды попадает во все строки ответа', () => {
    const entries = appendLog([], 'a\nb', true, 0, 0, '/tp 1 ~ 2')
    expect(entries.map((e) => e.suggest)).toEqual(['/tp 1 ~ 2', '/tp 1 ~ 2'])
    expect(appendLog([], 'a', true, 0, 0)[0]!.suggest).toBeUndefined()
  })

  it('многострочный ответ превращается в несколько записей с разными id', () => {
    const entries = appendLog([], 'a\nb', true, 0, 5)
    expect(entries.map((e) => [e.id, e.text])).toEqual([
      [5, 'a'],
      [6, 'b'],
    ])
  })

  it('хранит только последние записи', () => {
    let entries = appendLog([], 'x', true, 0, 0)
    for (let i = 1; i < LOG_KEEP + 10; i++) entries = appendLog(entries, 'x', true, 0, i)
    expect(entries).toHaveLength(LOG_KEEP)
    expect(entries[entries.length - 1]!.id).toBe(LOG_KEEP + 9)
  })

  it('при закрытой консоли гасит старые записи и ограничивает число', () => {
    let entries = appendLog([], 'old', true, 0, 0)
    for (let i = 1; i <= 10; i++) entries = appendLog(entries, `n${i}`, true, 1000, i)
    const shown = visibleLog(entries, 1000, false)
    expect(shown).toHaveLength(LOG_VISIBLE)
    expect(visibleLog(entries, LOG_TTL_MS * 2, false)).toHaveLength(0)
  })

  it('при открытой консоли показывает последние записи независимо от времени', () => {
    let entries: ReturnType<typeof appendLog> = []
    for (let i = 0; i < 20; i++) entries = appendLog(entries, 'x', true, 0, i)
    expect(visibleLog(entries, 10 * LOG_TTL_MS, true)).toHaveLength(LOG_VISIBLE_OPEN)
  })
})
