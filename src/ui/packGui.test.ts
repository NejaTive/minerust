import { reactive } from 'vue'
import { describe, expect, it } from 'vitest'
import { bindPackGui } from './packGui'

describe('переменные интерфейса из пака', () => {
  it('прописывает и обновляет CSS-переменные', () => {
    const vars: Record<string, string> = {}
    const root = { style: { setProperty: (n: string, v: string) => (vars[n] = v) } }
    const state = reactive({
      gui: {
        widgets: 'blob:w1',
        icons: 'blob:i',
        inventory: 'blob:n',
        container: 'blob:c',
        panel: 'blob:p',
        panelSlice: 8,
      },
    })
    bindPackGui(state, root)
    expect(vars['--gui-widgets']).toBe('url("blob:w1")')
    expect(vars['--gui-panel-slice']).toBe('8')
    expect(vars['--gui-container']).toBe('url("blob:c")')
    state.gui = { ...state.gui, widgets: 'blob:w2' }
    expect(vars['--gui-widgets']).toBe('url("blob:w2")')
  })
})
