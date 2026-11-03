import { describe, expect, it } from 'vitest'
import { CommandHistory } from './history'

describe('CommandHistory', () => {
  it('↑ идёт от новых к старым и останавливается на первой', () => {
    const h = new CommandHistory()
    h.push('/a')
    h.push('/b')
    expect(h.previous('')).toBe('/b')
    expect(h.previous('')).toBe('/a')
    expect(h.previous('')).toBeUndefined()
  })

  it('↓ возвращает недописанный ввод', () => {
    const h = new CommandHistory()
    h.push('/a')
    h.push('/b')
    h.previous('/ti')
    h.previous('/ti')
    expect(h.next()).toBe('/b')
    expect(h.next()).toBe('/ti')
    expect(h.next()).toBeUndefined()
  })

  it('пустые строки и подряд идущие дубликаты не сохраняются', () => {
    const h = new CommandHistory()
    h.push('/a')
    h.push('/a')
    h.push('  ')
    expect(h.previous('')).toBe('/a')
    expect(h.previous('')).toBeUndefined()
  })

  it('после отправки курсор сбрасывается в конец', () => {
    const h = new CommandHistory()
    h.push('/a')
    h.push('/b')
    h.previous('')
    h.previous('')
    h.push('/c')
    expect(h.previous('')).toBe('/c')
  })

  it('хранит не более 100 строк', () => {
    const h = new CommandHistory()
    for (let i = 0; i < 150; i++) h.push(`/${i}`)
    let count = 0
    while (h.previous('') !== undefined) count++
    expect(count).toBe(100)
  })
})
