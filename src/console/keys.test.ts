import { describe, expect, it } from 'vitest'
import { classifyConsoleKey } from './keys'

describe('classifyConsoleKey', () => {
  it('сопоставляет управляющие клавиши', () => {
    expect(classifyConsoleKey('Enter')).toBe('submit')
    expect(classifyConsoleKey('NumpadEnter')).toBe('submit')
    expect(classifyConsoleKey('Tab')).toBe('complete')
    expect(classifyConsoleKey('ArrowUp')).toBe('history-up')
    expect(classifyConsoleKey('ArrowDown')).toBe('history-down')
    expect(classifyConsoleKey('Escape')).toBe('close')
  })

  it('остальные клавиши идут в поле ввода', () => {
    expect(classifyConsoleKey('KeyE')).toBe('none')
    expect(classifyConsoleKey('Digit1')).toBe('none')
  })
})
