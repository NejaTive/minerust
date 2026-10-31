import { describe, expect, it } from 'vitest'
import { backTarget } from './screens'

describe('backTarget', () => {
  it('из списка миров — на титульный', () => expect(backTarget('worlds', false)).toBe('main'))
  it('из создания и удаления — в список миров', () => {
    expect(backTarget('create', false)).toBe('worlds')
    expect(backTarget('delete', false)).toBe('worlds')
  })
  it('из настроек — в главное меню', () => {
    expect(backTarget('settings', false)).toBe('main')
    expect(backTarget('settings', true)).toBe('main')
  })
  it('из управления: без мира — в настройки, в паузе — в паузу', () => {
    expect(backTarget('controls', false)).toBe('settings')
    expect(backTarget('controls', true)).toBe('main')
  })
})
