import { describe, expect, it } from 'vitest'
import { pauseMenuEscAction } from './lock-policy'

describe('pauseMenuEscAction', () => {
  const base = {
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

  it('из подменю — назад на главный', () => {
    expect(pauseMenuEscAction({ ...base, onMainScreen: false })).toBe('back-to-main')
    expect(pauseMenuEscAction({ ...base, onMainScreen: false, started: false })).toBe(
      'back-to-main',
    )
    expect(pauseMenuEscAction({ ...base, onMainScreen: false, busy: true })).toBe('back-to-main')
  })
})
