import { describe, expect, it } from 'vitest'
import { createWorldSession } from './session'

describe('createWorldSession', () => {
  it('в главном меню мира нет — движок не нужен', () => {
    const session = createWorldSession()
    expect(session.activeId.value).toBeNull()
    expect(session.inWorld.value).toBe(false)
  })

  it('вход и выход переключают активный мир', () => {
    const session = createWorldSession()
    session.enter('a')
    expect(session.inWorld.value).toBe(true)
    session.enter('b')
    expect(session.activeId.value).toBe('b')
    session.exit()
    expect(session.inWorld.value).toBe(false)
  })
})
