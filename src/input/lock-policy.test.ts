import { describe, expect, it } from 'vitest'
import { pauseMenuEscAction, startOverlay } from './lock-policy'

describe('pauseMenuEscAction', () => {
  const base = {
    inWorld: true,
    started: true,
    busy: false,
    hasError: false,
    bench: false,
    onMainScreen: true,
  }

  it('на главном после старта — продолжить', () => {
    expect(pauseMenuEscAction(base)).toBe('resume')
  })

  it('до старта / busy / ошибка / bench — игнор', () => {
    expect(pauseMenuEscAction({ ...base, started: false })).toBe('none')
    expect(pauseMenuEscAction({ ...base, busy: true })).toBe('none')
    expect(pauseMenuEscAction({ ...base, hasError: true })).toBe('none')
    expect(pauseMenuEscAction({ ...base, bench: true })).toBe('none')
  })

  it('в главном меню без мира Esc ничего не делает', () => {
    expect(pauseMenuEscAction({ ...base, inWorld: false })).toBe('none')
  })

  it('из подменю — назад на главный', () => {
    expect(pauseMenuEscAction({ ...base, onMainScreen: false })).toBe('back')
    expect(pauseMenuEscAction({ ...base, onMainScreen: false, started: false })).toBe('back')
    expect(pauseMenuEscAction({ ...base, onMainScreen: false, busy: true })).toBe('back')
  })
})

describe('startOverlay', () => {
  const base = { inWorld: true, pendingStart: true, busy: true, locked: false, hasError: false }

  it('мир грузится — экран загрузки', () => expect(startOverlay(base)).toBe('loading'))
  it('загрузился, курсор не захвачен — «Кликните, чтобы начать»', () => {
    expect(startOverlay({ ...base, busy: false })).toBe('click')
  })
  it('курсор захвачен — оверлея нет', () => {
    expect(startOverlay({ ...base, busy: false, locked: true })).toBe('none')
  })
  it('Esc (старт отменён), ошибка, нет мира — оверлея нет', () => {
    expect(startOverlay({ ...base, pendingStart: false })).toBe('none')
    expect(startOverlay({ ...base, hasError: true })).toBe('none')
    expect(startOverlay({ ...base, inWorld: false })).toBe('none')
  })
})
