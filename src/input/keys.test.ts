import { describe, expect, it } from 'vitest'
import { classifyKeyDown, type KeyInfo } from './keys'

const key = (code: string, extra: Partial<KeyInfo> = {}): KeyInfo => ({
  code,
  repeat: false,
  ctrlKey: false,
  metaKey: false,
  altKey: false,
  ...extra,
})

describe('classifyKeyDown', () => {
  it('[ и ] меняют дальность и без захвата курсора, и с автоповтором', () => {
    expect(classifyKeyDown(key('BracketLeft'), false, false)).toEqual({
      type: 'distance',
      delta: -1,
    })
    expect(classifyKeyDown(key('BracketRight', { repeat: true }), true, false)).toEqual({
      type: 'distance',
      delta: 1,
    })
  })

  it('N — новый мир один раз, автоповтор гасится', () => {
    expect(classifyKeyDown(key('KeyN'), true, false)).toEqual({ type: 'new-world' })
    expect(classifyKeyDown(key('KeyN', { repeat: true }), true, false)).toEqual({ type: 'none' })
  })

  it('хоткей с модификатором — системное сочетание, идёт как обычная клавиша', () => {
    expect(classifyKeyDown(key('KeyN', { ctrlKey: true }), true, false)).toEqual({ type: 'key' })
    expect(classifyKeyDown(key('KeyN', { metaKey: true }), false, false)).toEqual({ type: 'none' })
  })

  it('без захвата курсора игровые клавиши игнорируются', () => {
    expect(classifyKeyDown(key('KeyW'), false, false)).toEqual({ type: 'none' })
    expect(classifyKeyDown(key('Digit1'), false, false)).toEqual({ type: 'none' })
  })

  it('цифры выбирают слот 0..8 и в движок не попадают', () => {
    expect(classifyKeyDown(key('Digit1'), true, false)).toEqual({ type: 'slot', slot: 0 })
    expect(classifyKeyDown(key('Digit9'), true, false)).toEqual({ type: 'slot', slot: 8 })
    expect(classifyKeyDown(key('Digit0'), true, false)).toEqual({ type: 'key' })
    expect(classifyKeyDown(key('Digit2', { ctrlKey: true }), true, false)).toEqual({ type: 'none' })
  })

  it('V и T работают только с отладкой и без модификаторов', () => {
    expect(classifyKeyDown(key('KeyV'), true, true)).toEqual({ type: 'noclip' })
    expect(classifyKeyDown(key('KeyT'), true, true)).toEqual({ type: 'time-fast' })
    expect(classifyKeyDown(key('KeyV'), true, false)).toEqual({ type: 'key' })
    expect(classifyKeyDown(key('KeyT', { altKey: true }), true, true)).toEqual({ type: 'key' })
  })

  it('R меняет погоду только с отладкой и без модификаторов', () => {
    expect(classifyKeyDown(key('KeyR'), true, true)).toEqual({ type: 'weather' })
    expect(classifyKeyDown(key('KeyR'), true, false)).toEqual({ type: 'key' })
    expect(classifyKeyDown(key('KeyR', { metaKey: true }), true, true)).toEqual({ type: 'key' })
    expect(classifyKeyDown(key('KeyR'), false, true)).toEqual({ type: 'none' })
  })

  it('автоповтор обычной клавиши игнорируется, остальное уходит в движок', () => {
    expect(classifyKeyDown(key('KeyW', { repeat: true }), true, false)).toEqual({ type: 'none' })
    expect(classifyKeyDown(key('KeyW'), true, false)).toEqual({ type: 'key' })
  })
})
