import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

const read = (path: string) => readFileSync(new URL(path, import.meta.url), 'utf8')

describe('строки списка в меню', () => {
  it('строка мира помечена px-row и не получает спрайт кнопки', () => {
    expect(read('../components/menu/WorldListScreen.vue')).toMatch(/class="row px-row"/)
    const css = read('./pixel.css')
    expect(css).toContain('.px-menu button:not(.px-row)::before')
    expect(css).not.toMatch(/\.px-menu button::(before|after)/)
  })
})
